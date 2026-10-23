use crate::{application::ports::CredentialStore, domain::accounts::Credentials};

pub struct NativeVault;
#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "writes and removes one random synthetic entry in the current Windows profile"]
    async fn native_vault_roundtrip() {
        let key = format!("test-{}", uuid::Uuid::new_v4());
        NativeVault
            .put(
                key.clone(),
                Credentials {
                    cookies: "session=synthetic-test-only".into(),
                    user_agent: "fixture-UA".into(),
                },
            )
            .await
            .unwrap();
        let read = NativeVault.get(key.clone()).await;
        let removed = NativeVault.remove(key.clone()).await;
        assert!(removed.is_ok(), "temporary native entry cleanup failed");
        let value = read.unwrap().unwrap();
        assert_eq!(value.cookies, "session=synthetic-test-only");
        assert!(NativeVault.get(key).await.unwrap().is_none());
    }
}
#[cfg(windows)]
impl CredentialStore for NativeVault {
    async fn put(&self, key: String, secret: Credentials) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || {
            let raw = serde_json::to_vec(&(secret.cookies, secret.user_agent))?;
            if raw.len() > 2560 {
                anyhow::bail!("凭据超过 Windows 存储容量；请缩减 Cookie 请求头");
            }
            keyring::Entry::new("Parallax.sessions.v1", &key)?
                .set_secret(&raw)
                .map_err(|_| anyhow::anyhow!("Windows 凭据写入失败"))
        })
        .await?
    }
    async fn get(&self, key: String) -> anyhow::Result<Option<Credentials>> {
        tokio::task::spawn_blocking(move || {
            match keyring::Entry::new("Parallax.sessions.v1", &key)?.get_secret() {
                Ok(bytes) => {
                    let (cookies, user_agent) = serde_json::from_slice(&bytes)?;
                    Ok(Some(Credentials {
                        cookies,
                        user_agent,
                    }))
                }
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => anyhow::bail!("Windows 凭据读取失败"),
            }
        })
        .await?
    }
    async fn remove(&self, key: String) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || {
            match keyring::Entry::new("Parallax.sessions.v1", &key)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(_) => anyhow::bail!("Windows 凭据删除失败"),
            }
        })
        .await?
    }
}
#[cfg(not(windows))]
impl CredentialStore for NativeVault {
    async fn put(&self, _: String, _: Credentials) -> anyhow::Result<()> {
        anyhow::bail!("当前凭据后端仅支持 Windows")
    }
    async fn get(&self, _: String) -> anyhow::Result<Option<Credentials>> {
        anyhow::bail!("当前凭据后端仅支持 Windows")
    }
    async fn remove(&self, _: String) -> anyhow::Result<()> {
        anyhow::bail!("当前凭据后端仅支持 Windows")
    }
}
