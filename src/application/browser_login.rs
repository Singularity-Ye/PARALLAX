//! Interactive login ownership and expiry, independent of browser and HTTP APIs.
use super::{AppError, accounts::Accounts, ports::*};
use crate::domain::{accounts::*, selection::now};
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[cfg(test)]
#[path = "browser_login_tests.rs"]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginStatus {
    Pending,
    Completed,
    Cancelled,
    Expired,
}

#[derive(Clone, Serialize)]
pub struct LoginView {
    pub id: String,
    pub account_id: String,
    pub platform: Platform,
    pub status: LoginStatus,
    pub expires_at: u64,
    pub message: String,
    pub session: Option<SessionView>,
}
struct Flow<W> {
    view: LoginView,
    generation: Option<String>,
    window: Option<W>,
    retry_at: u64,
}
pub struct BrowserLogins<R, C, V, B: BrowserAuth> {
    accounts: Arc<Accounts<R, C, V>>,
    browser: B,
    active: Mutex<Option<Flow<B::Window>>>,
}
impl<
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    B: BrowserAuth + 'static,
> BrowserLogins<R, C, V, B>
{
    pub fn new(accounts: Arc<Accounts<R, C, V>>, browser: B) -> Arc<Self> {
        Arc::new(Self {
            accounts,
            browser,
            active: Mutex::new(None),
        })
    }
    async fn expire(&self, flow: &mut Flow<B::Window>) -> Result<(), AppError> {
        if flow.view.status == LoginStatus::Pending && now() >= flow.view.expires_at {
            flow.view.status = LoginStatus::Expired;
            flow.view.message = "登录窗口已超时，请重新发起；原登录态未被替换".into();
            if let Some(window) = flow.window.take() {
                self.browser
                    .close(window)
                    .await
                    .map_err(|e| AppError::Invalid(e.into()))?;
            }
        }
        Ok(())
    }
    pub async fn start(
        self: &Arc<Self>,
        account: &str,
        platform: Platform,
    ) -> Result<LoginView, AppError> {
        let mut active = self.active.lock().await;
        if let Some(flow) = active.as_mut() {
            self.expire(flow).await?;
            if flow.view.status == LoginStatus::Pending {
                return Err(AppError::Conflict("已有登录窗口，请先完成或取消"));
            }
        }
        let generation = self.accounts.generation(account, platform).await?;
        let window = self
            .browser
            .launch(platform)
            .await
            .map_err(|e| AppError::Invalid(e.into()))?;
        let view = LoginView {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: account.into(),
            platform,
            status: LoginStatus::Pending,
            expires_at: now() + 600,
            message:
                "请在独立浏览器完成登录或验证，回到此处点击完成登录；密码和 Guard 码仅输入官方页面"
                    .into(),
            session: None,
        };
        *active = Some(Flow {
            view: view.clone(),
            generation,
            window: Some(window),
            retry_at: 0,
        });
        let weak = Arc::downgrade(self);
        let id = view.id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(600)).await;
            if let Some(app) = weak.upgrade() {
                let mut active = app.active.lock().await;
                if let Some(flow) = active.as_mut().filter(|f| f.view.id == id)
                    && app.expire(flow).await.is_err()
                {
                    flow.view.message = "登录已过期，浏览器清理失败，请手动关闭独立窗口".into();
                }
            }
        });
        Ok(view)
    }
    pub async fn current(&self) -> Result<Option<LoginView>, AppError> {
        let mut active = self.active.lock().await;
        if let Some(flow) = active.as_mut() {
            self.expire(flow).await?;
        }
        Ok(active.as_ref().map(|f| f.view.clone()))
    }
    pub async fn finish(&self, id: &str) -> Result<LoginView, AppError> {
        let mut active = self.active.lock().await;
        let flow = active
            .as_mut()
            .filter(|f| f.view.id == id)
            .ok_or(AppError::Missing(
                "登录流程不存在，可能已重启或被新流程替代",
            ))?;
        self.expire(flow).await?;
        if flow.view.status == LoginStatus::Completed {
            return Ok(flow.view.clone());
        }
        if flow.view.status != LoginStatus::Pending {
            return Err(AppError::Conflict("登录流程已结束，请重新发起"));
        }
        if flow.retry_at > now() {
            return Err(AppError::Conflict("登录验证处于冷却中，请稍后再试"));
        }
        let secret = self
            .browser
            .capture(flow.window.as_ref().unwrap(), flow.view.platform)
            .await
            .map_err(|e| AppError::Invalid(e.into()))?;
        flow.retry_at = now() + 30;
        let session = self
            .accounts
            .recover_browser(
                &flow.view.account_id,
                flow.view.platform,
                &flow.generation,
                secret,
            )
            .await?;
        flow.view.status = LoginStatus::Completed;
        flow.view.message = session.message.clone();
        flow.view.session = Some(session);
        if let Some(window) = flow.window.take()
            && self.browser.close(window).await.is_err()
        {
            flow.view
                .message
                .push_str("；浏览器清理失败，请手动关闭独立窗口");
        }
        Ok(flow.view.clone())
    }
    pub async fn cancel(&self, id: &str) -> Result<LoginView, AppError> {
        let mut active = self.active.lock().await;
        let flow = active
            .as_mut()
            .filter(|f| f.view.id == id)
            .ok_or(AppError::Missing("登录流程不存在"))?;
        self.expire(flow).await?;
        if flow.view.status == LoginStatus::Pending {
            flow.view.status = LoginStatus::Cancelled;
            flow.view.message = "已取消登录；原登录态未被替换".into();
            if let Some(window) = flow.window.take() {
                self.browser
                    .close(window)
                    .await
                    .map_err(|e| AppError::Invalid(e.into()))?;
            }
        }
        Ok(flow.view.clone())
    }
}
