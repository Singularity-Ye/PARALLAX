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
        let configured = self.vault.guard_secret(key.clone()).await?.is_some();
        let identity = self.vault.identity_secret(key).await?;
        let identity_configured = identity.is_some();
        let device_id = identity
            .and_then(|(_, d)| d)
            .or_else(|| Some(GuardSecret::default_device_id(&steam_id)));
        Ok(GuardStatus {
            steam_id,
            configured,
            identity_configured,
            device_id,
            scope: if identity_configured {
                "login_and_mobileconf"
            } else {
                "login_code_only"
            },
            automatic_login_available: configured,
        })
    }
    pub async fn import(&self, id: &str, raw: &str) -> Result<GuardStatus, AppError> {
        self.import_full(id, Some(raw), None, None).await
    }
    pub async fn import_full(
        &self,
        id: &str,
        shared_secret: Option<&str>,
        identity_secret: Option<&str>,
        device_id: Option<&str>,
    ) -> Result<GuardStatus, AppError> {
        {
            let _gate = self.gate.lock().await;
            let (key, steam_id) = self.binding(id).await?;
            if let Some(raw) = shared_secret.map(str::trim).filter(|s| !s.is_empty()) {
                let secret = GuardSecret::parse(raw).map_err(AppError::Invalid)?;
                self.vault.put_guard_secret(key.clone(), secret).await?;
            }
            if let Some(id_sec) = identity_secret.map(str::trim).filter(|s| !s.is_empty()) {
                let _ = GuardSecret::signature(id_sec, "conf", 1_700_000_000)
                    .map_err(AppError::Invalid)?;
                let dev = device_id
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .or_else(|| Some(GuardSecret::default_device_id(&steam_id)));
                self.vault
                    .put_identity_secret(key, id_sec.to_owned(), dev)
                    .await?;
            }
        }
        self.status(id).await
    }
    pub async fn identity_credentials(
        &self,
        id: &str,
    ) -> Result<(String, String, String), AppError> {
        let _gate = self.gate.lock().await;
        let (key, steam_id) = self.binding(id).await?;
        let (identity_secret, dev) = self
            .vault
            .identity_secret(key)
            .await?
            .ok_or(AppError::Missing(
                "此账户尚未配置 identity_secret（可在 Steam 令牌页导入 maFile 或粘贴 identity_secret）",
            ))?;
        let device_id = dev.unwrap_or_else(|| GuardSecret::default_device_id(&steam_id));
        Ok((steam_id, identity_secret, device_id))
    }
    pub async fn code(&self, id: &str) -> Result<GuardCode, AppError> {
        self.code_at(id, now()).await
    }
    pub async fn code_at(&self, id: &str, at: u64) -> Result<GuardCode, AppError> {
        let _gate = self.gate.lock().await;
        let (key, _) = self.binding(id).await?;
        let secret = self
            .vault
            .guard_secret(key)
            .await?
            .ok_or(AppError::Missing("此账户尚未配置Steam Guard shared_secret"))?;
        Ok(secret.code(at))
    }
    pub async fn remove(&self, id: &str) -> Result<(), AppError> {
        let _gate = self.gate.lock().await;
        let (key, _) = self.binding(id).await?;
        self.vault.remove_guard_secret(key.clone()).await?;
        let _ = self.vault.remove_identity_secret(key).await;
        Ok(())
    }
}
