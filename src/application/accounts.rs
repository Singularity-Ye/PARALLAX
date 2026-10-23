use super::{
    AppError,
    ports::{AccountRepository, CredentialStore, SessionVerifier},
};
use crate::domain::{accounts::*, selection::now};
use std::sync::Arc;
use tokio::sync::Mutex;

#[path = "accounts_inventory.rs"]
mod inventory;

pub struct Accounts<R, C, V> {
    repo: R,
    vault: C,
    verifier: V,
    // Serialize replacement, revocation and verification, including native vault calls.
    gate: Mutex<()>,
}
impl<R: AccountRepository, C: CredentialStore, V: SessionVerifier> Accounts<R, C, V> {
    pub fn new(repo: R, vault: C, verifier: V) -> Arc<Self> {
        Arc::new(Self {
            repo,
            vault,
            verifier,
            gate: Mutex::new(()),
        })
    }
    pub async fn list(&self) -> Result<Vec<Account>, AppError> {
        Ok(self.repo.accounts().await?)
    }
    pub async fn create(&self, label: String, steam_id: String) -> Result<Account, AppError> {
        let label = label.trim().to_string();
        if label.is_empty()
            || label.chars().count() > 80
            || label.chars().any(char::is_control)
            || steam_id.len() != 17
            || !steam_id.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(AppError::Invalid(
                "名称需要 1–80 个字符；Steam ID 需要 17 位数字".into(),
            ));
        }
        let account = Account {
            id: uuid::Uuid::new_v4().to_string(),
            label,
            steam_id,
            created_at: now(),
        };
        self.repo.add_account(account.clone()).await?;
        Ok(account)
    }
    pub(crate) async fn account(&self, id: &str) -> Result<Account, AppError> {
        self.repo
            .account(id.into())
            .await?
            .ok_or(AppError::Missing("账户不存在"))
    }
    pub(crate) async fn generation(
        &self,
        id: &str,
        platform: Platform,
    ) -> Result<Option<String>, AppError> {
        self.account(id).await?;
        Ok(self
            .repo
            .session(id.into(), platform)
            .await?
            .map(|s| s.view.generation))
    }

    // Browser recovery is a distinct operation: verify the captured identity before
    // replacing any stored secret. Generation binding prevents an old login window
    // from undoing a later manual import or revocation.
    pub(crate) async fn recover_browser(
        &self,
        id: &str,
        platform: Platform,
        expected_generation: &Option<String>,
        secret: Credentials,
    ) -> Result<SessionView, AppError> {
        let _guard = self.gate.lock().await;
        let account = self.account(id).await?;
        let old = self.repo.session(id.into(), platform).await?;
        if &old.as_ref().map(|s| s.view.generation.clone()) != expected_generation {
            return Err(AppError::Conflict(
                "账户登录态已变化，请取消旧窗口并重新发起登录",
            ));
        }
        secret
            .validate(platform, &account.steam_id)
            .map_err(AppError::Invalid)?;
        let (global_retry, blocked) = self.repo.buff_policy().await?;
        if old.as_ref().is_some_and(|s| s.view.retry_at > now())
            || (platform == Platform::Buff && global_retry > now())
        {
            return Err(AppError::Conflict("验证请求处于冷却中，请稍后再试"));
        }
        // Pacing survives failure and restart. A failed recovery leaves the old
        // credentials intact; only verified success may clear a BUFF challenge.
        if let Some(mut prior) = old.clone() {
            prior.view.retry_at = now() + 30;
            self.repo.save_session(id.into(), prior).await?;
        }
        if platform == Platform::Buff {
            self.repo.set_buff_policy(now() + 30, blocked).await?;
        }
        let result = self
            .verifier
            .verify(
                &account,
                platform,
                Credentials {
                    cookies: secret.cookies.clone(),
                    user_agent: secret.user_agent.clone(),
                },
            )
            .await;
        if platform == Platform::Buff {
            // Even a valid probe must not release the challenge before the new
            // credential reference has been committed successfully.
            self.repo
                .set_buff_policy(
                    now() + result.retry_secs,
                    blocked || result.status == SessionStatus::VerificationRequired,
                )
                .await?;
        }
        if result.status != SessionStatus::Valid || result.subject.is_none() {
            if let Some(mut prior) = old {
                prior.view.retry_at = now() + result.retry_secs;
                self.repo.save_session(id.into(), prior).await?;
            }
            return Err(AppError::Invalid(result.message.into()));
        }
        let key = uuid::Uuid::new_v4().to_string();
        self.vault.put(key.clone(), secret).await?;
        let stored = StoredSession {
            credential_key: key.clone(),
            view: SessionView {
                platform,
                generation: key.clone(),
                status: SessionStatus::Valid,
                checked_at: Some(now()),
                retry_at: now() + result.retry_secs,
                message: "浏览器登录态已通过在线身份验证并保存".into(),
                subject: result.subject,
            },
        };
        if let Err(err) = self.repo.save_session(id.into(), stored.clone()).await {
            self.vault.remove(key).await?;
            return Err(err.into());
        }
        if platform == Platform::Buff {
            self.repo
                .set_buff_policy(stored.view.retry_at, false)
                .await?;
        }
        let mut view = stored.view;
        if let Some(old) = old
            && self.vault.remove(old.credential_key).await.is_err()
        {
            view.message = "新登录态已验证并保存；旧凭据清理失败，需检查 Windows 凭据管理器".into();
        }
        Ok(view)
    }
    pub async fn sessions(&self, id: &str) -> Result<Vec<SessionView>, AppError> {
        self.account(id).await?;
        Ok(self
            .repo
            .sessions(id.into())
            .await?
            .into_iter()
            .map(|s| s.view)
            .collect())
    }
    pub async fn import(
        &self,
        id: &str,
        platform: Platform,
        secret: Credentials,
    ) -> Result<SessionView, AppError> {
        let _guard = self.gate.lock().await;
        let account = self.account(id).await?;
        secret
            .validate(platform, &account.steam_id)
            .map_err(AppError::Invalid)?;
        let old = self.repo.session(id.into(), platform).await?;
        let key = uuid::Uuid::new_v4().to_string();
        self.vault.put(key.clone(), secret).await?;
        let stored = StoredSession {
            credential_key: key.clone(),
            view: SessionView {
                platform,
                generation: key.clone(),
                status: SessionStatus::Unverified,
                checked_at: None,
                // Replacing cookies must not bypass the platform cooldown or challenge state.
                retry_at: old.as_ref().map_or(0, |s| s.view.retry_at),
                message: "已保存，尚未在线验证".into(),
                subject: None,
            },
        };
        if let Err(err) = self.repo.save_session(id.into(), stored.clone()).await {
            self.vault.remove(key).await?;
            return Err(err.into());
        }
        let mut view = stored.view;
        if let Some(old) = old
            && self.vault.remove(old.credential_key).await.is_err()
        {
            view.message = "新登录态已保存；旧凭据清理失败，需检查 Windows 凭据管理器".into();
        }
        Ok(view)
    }
    pub async fn verify(&self, id: &str, platform: Platform) -> Result<SessionView, AppError> {
        let _guard = self.gate.lock().await;
        let account = self.account(id).await?;
        let mut session = self
            .repo
            .session(id.into(), platform)
            .await?
            .ok_or(AppError::Missing("请先导入登录态"))?;
        if session.view.status == SessionStatus::Missing {
            return Err(AppError::Conflict("登录态已停用，请重新导入"));
        }
        if session.view.status == SessionStatus::VerificationRequired {
            return Err(AppError::Conflict(
                "平台要求浏览器验证，请使用独立登录窗口完成验证",
            ));
        }
        let (global_retry, blocked) = self.repo.buff_policy().await?;
        if platform == Platform::Buff && blocked {
            return Err(AppError::Conflict(
                "BUFF 需要浏览器验证；替换 Cookie 或新增账户不能解除阻断",
            ));
        }
        if session.view.retry_at > now() || (platform == Platform::Buff && global_retry > now()) {
            return Err(AppError::Conflict("验证请求处于冷却中，请稍后再试"));
        }
        let result = if let Some(secret) = self.vault.get(session.credential_key.clone()).await? {
            // Persist pacing before the network call, so a process restart cannot reset it.
            session.view.retry_at = now() + 30;
            session.view.status = SessionStatus::Unknown;
            session.view.message = "验证进行中或上次验证被中断".into();
            self.repo.save_session(id.into(), session.clone()).await?;
            if platform == Platform::Buff {
                self.repo.set_buff_policy(now() + 30, false).await?;
            }
            self.verifier.verify(&account, platform, secret).await
        } else {
            Verification {
                status: SessionStatus::Missing,
                message: "凭据存储中没有登录态，请重新导入",
                subject: None,
                retry_secs: 0,
            }
        };
        session.view.status = result.status;
        session.view.message = result.message.into();
        session.view.subject = result.subject;
        session.view.checked_at = Some(now());
        session.view.retry_at = now() + result.retry_secs;
        if platform == Platform::Buff {
            self.repo
                .set_buff_policy(
                    session.view.retry_at,
                    result.status == SessionStatus::VerificationRequired,
                )
                .await?;
        }
        self.repo.save_session(id.into(), session.clone()).await?;
        Ok(session.view)
    }
    pub async fn revoke(&self, id: &str, platform: Platform) -> Result<SessionView, AppError> {
        let _guard = self.gate.lock().await;
        self.account(id).await?;
        let mut session = self
            .repo
            .session(id.into(), platform)
            .await?
            .ok_or(AppError::Missing("没有已保存的登录态"))?;
        // First remove valid status; a vault failure must never leave this session executable.
        session.view.status = SessionStatus::Missing;
        session.view.generation = uuid::Uuid::new_v4().to_string();
        session.view.message = "本地登录态已停用；不代表平台远程注销".into();
        self.repo.save_session(id.into(), session.clone()).await?;
        self.vault.remove(session.credential_key).await?;
        Ok(session.view)
    }

    pub(crate) async fn read_buff<P: super::ports::BuffQuoteProvider>(
        &self,
        id: &str,
        goods_id: u64,
        name: &str,
        provider: &P,
    ) -> Result<(crate::domain::buff_quotes::BuffSellPage, String), AppError> {
        use super::ports::BuffReadError as E;
        let _guard = self.gate.lock().await;
        self.account(id).await?;
        let mut session = self
            .repo
            .session(id.into(), Platform::Buff)
            .await?
            .ok_or(AppError::Missing("请先导入并验证 BUFF 登录态"))?;
        let checked_now = now();
        if session.view.status != SessionStatus::Valid
            || session.view.subject.is_none()
            || session
                .view
                .checked_at
                .is_none_or(|at| at > checked_now || checked_now - at > 300)
        {
            return Err(AppError::Conflict(
                "需要最近五分钟内在线身份验证通过的 BUFF 登录态",
            ));
        }
        let (retry, blocked) = self.repo.buff_policy().await?;
        if blocked {
            return Err(AppError::Buff(E::Challenge));
        }
        if retry > now() || session.view.retry_at > now() {
            return Err(AppError::Buff(E::RateLimited(
                retry.max(session.view.retry_at),
            )));
        }
        let Some(secret) = self.vault.get(session.credential_key.clone()).await? else {
            session.view.status = SessionStatus::Missing;
            session.view.message = "凭据存储缺失，请重新导入".into();
            self.repo.save_session(id.into(), session).await?;
            return Err(AppError::Conflict("凭据存储缺失，请重新导入"));
        };
        // Shares the gate and persistent policy with verification and browser recovery.
        self.repo.set_buff_policy(now() + 2, false).await?;
        match provider.sell_page(goods_id, secret).await {
            Ok(page) => {
                page.validate(goods_id, name)
                    .map_err(|_| AppError::Buff(E::IdentityMismatch))?;
                Ok((page, session.view.generation))
            }
            Err(error) => {
                match error {
                    E::Challenge => {
                        self.repo.set_buff_policy(now() + 30, true).await?;
                        session.view.status = SessionStatus::VerificationRequired;
                    }
                    E::RateLimited(secs) => {
                        let retry = now() + secs.clamp(300, 604800);
                        self.repo.set_buff_policy(retry, false).await?;
                        session.view.retry_at = retry;
                        // This is a read failure, not an identity verification result.
                        session.view.message = error.message().into();
                        self.repo.save_session(id.into(), session).await?;
                        return Err(AppError::Buff(E::RateLimited(retry)));
                    }
                    E::Expired => session.view.status = SessionStatus::Invalid,
                    E::CredentialsChanged => {
                        session.view.status = SessionStatus::Unverified;
                        session.view.generation = uuid::Uuid::new_v4().to_string();
                    }
                    _ => return Err(AppError::Buff(error)),
                }
                session.view.message = error.message().into();
                self.repo.save_session(id.into(), session).await?;
                Err(AppError::Buff(error))
            }
        }
    }
}
