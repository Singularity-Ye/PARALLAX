use super::{
    AppError,
    ports::{AccountRepository, GuardSecretStore},
};
#[cfg(test)]
#[path = "steam_guard_tests.rs"]
mod tests;
use crate::domain::{
    selection::now,
    steam_guard::{GuardCode, GuardSecret, GuardStatus},
};
use std::sync::Arc;
pub struct SteamGuard<R, V> {
    repo: R,
    vault: V,
    gate: tokio::sync::Mutex<()>,
}
impl<R: AccountRepository, V: GuardSecretStore> SteamGuard<R, V> {
    pub fn new(repo: R, vault: V) -> Arc<Self> {
        Arc::new(Self {
            repo,
            vault,
            gate: tokio::sync::Mutex::new(()),
        })
    }
    async fn binding(&self, id: &str) -> Result<(String, String), AppError> {
        let account = self
            .repo
            .account(id.into())
            .await?
            .ok_or(AppError::Missing("账户不存在"))?;
        Ok((
            format!("{}:{}", account.id, account.steam_id),
            account.steam_id,
        ))
    }
    pub async fn status(&self, id: &str) -> Result<GuardStatus, AppError> {
        let _gate = self.gate.lock().await;
        let (key, steam_id) = self.binding(id).await?;
        let configured = self.vault.guard_secret(key).await?.is_some();
        Ok(GuardStatus {
            steam_id,
            configured,
            scope: "login_code_only",
            automatic_login_available: false,
        })
    }
    pub async fn import(&self, id: &str, raw: &str) -> Result<GuardStatus, AppError> {
        let secret = GuardSecret::parse(raw).map_err(AppError::Invalid)?;
        {
            let _gate = self.gate.lock().await;
            let (key, _) = self.binding(id).await?;
            self.vault.put_guard_secret(key, secret).await?;
        }
        self.status(id).await
    }
    pub async fn code(&self, id: &str) -> Result<GuardCode, AppError> {
        let _gate = self.gate.lock().await;
        let (key, _) = self.binding(id).await?;
        let secret = self
            .vault
            .guard_secret(key)
            .await?
            .ok_or(AppError::Missing("此账户尚未配置Steam Guard shared_secret"))?;
        Ok(secret.code(now()))
    }
    pub async fn remove(&self, id: &str) -> Result<(), AppError> {
        let _gate = self.gate.lock().await;
        let (key, _) = self.binding(id).await?;
        self.vault.remove_guard_secret(key).await?;
        Ok(())
    }
}
