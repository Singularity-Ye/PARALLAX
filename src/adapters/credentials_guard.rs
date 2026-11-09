use super::credentials::NativeVault;
use crate::{application::ports::GuardSecretStore, domain::steam_guard::GuardSecret};
#[cfg(windows)]
impl GuardSecretStore for NativeVault {
    async fn guard_secret(&self, key: String) -> anyhow::Result<Option<GuardSecret>> {
        tokio::task::spawn_blocking(move || {
            match keyring::Entry::new("Parallax.steam-guard.v1", &key)?.get_secret() {
                Ok(raw) => {
                    let text = String::from_utf8(raw)
                        .map_err(|_| anyhow::anyhow!("令牌密钥存储格式无效"))?;
                    Ok(Some(
                        GuardSecret::parse(&text)
                            .map_err(|_| anyhow::anyhow!("令牌密钥格式无效"))?,
                    ))
                }
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => anyhow::bail!("Windows令牌密钥读取失败"),
            }
        })
        .await?
    }
    async fn put_guard_secret(&self, key: String, secret: GuardSecret) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || {
            keyring::Entry::new("Parallax.steam-guard.v1", &key)?
                .set_secret(secret.encoded().as_bytes())
                .map_err(|_| anyhow::anyhow!("Windows令牌密钥保存失败"))
        })
        .await?
    }
    async fn remove_guard_secret(&self, key: String) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || {
            match keyring::Entry::new("Parallax.steam-guard.v1", &key)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => anyhow::bail!("Windows令牌密钥停用失败"),
            }
        })
        .await?
    }
    async fn identity_secret(&self, key: String) -> anyhow::Result<Option<(String, Option<String>)>> {
        tokio::task::spawn_blocking(move || {
            match keyring::Entry::new("Parallax.steam-identity.v1", &key)?.get_secret() {
                Ok(raw) => {
                    let parsed: (String, Option<String>) = serde_json::from_slice(&raw)
                        .map_err(|_| anyhow::anyhow!("identity_secret 存储格式无效"))?;
                    Ok(Some(parsed))
                }
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => anyhow::bail!("Windows identity_secret 读取失败"),
            }
        })
        .await?
    }
    async fn put_identity_secret(
        &self,
        key: String,
        identity_secret: String,
        device_id: Option<String>,
    ) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || {
            let bytes = serde_json::to_vec(&(identity_secret, device_id))?;
            keyring::Entry::new("Parallax.steam-identity.v1", &key)?
                .set_secret(&bytes)
                .map_err(|_| anyhow::anyhow!("Windows identity_secret 保存失败"))
        })
        .await?
    }
    async fn remove_identity_secret(&self, key: String) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || {
            match keyring::Entry::new("Parallax.steam-identity.v1", &key)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => anyhow::bail!("Windows identity_secret 停用失败"),
            }
        })
        .await?
    }
}
#[cfg(not(windows))]
impl GuardSecretStore for NativeVault {
    async fn guard_secret(&self, _: String) -> anyhow::Result<Option<GuardSecret>> {
        anyhow::bail!("当前令牌密钥后端仅支持Windows")
    }
    async fn put_guard_secret(&self, _: String, _: GuardSecret) -> anyhow::Result<()> {
        anyhow::bail!("当前令牌密钥后端仅支持Windows")
    }
    async fn remove_guard_secret(&self, _: String) -> anyhow::Result<()> {
        anyhow::bail!("当前令牌密钥后端仅支持Windows")
    }
}
