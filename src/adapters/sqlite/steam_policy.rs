use super::Store;
use crate::application::ports::{SteamReadPolicy, SteamReadScope};
use rusqlite::{OptionalExtension, params};
impl SteamReadPolicy for Store {
    async fn steam_read_retry(&self, scope: SteamReadScope) -> anyhow::Result<u64> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT value FROM auth_policy WHERE key=?1",
                    [scope.key()],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| s.parse().map_err(Into::into))
                .transpose()
                .map(|v| v.unwrap_or(0))
        })
        .await
    }
    async fn set_steam_read_retry(&self, scope: SteamReadScope, at: u64) -> anyhow::Result<()> {
        self.account_db(move|db| {
            db.execute("INSERT INTO auth_policy(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![scope.key(),at.to_string()])?;Ok(())
        }).await
    }
}
