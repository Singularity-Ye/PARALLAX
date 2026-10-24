use super::Store;
use crate::{application::ports::FxRepository, domain::fx::FxSnapshot};
use rusqlite::{OptionalExtension, params};
impl FxRepository for Store {
    async fn insert_fx(&self, snapshot: FxSnapshot) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute(
                "INSERT INTO exchange_rate_snapshots(id,fetched_at,snapshot) VALUES(?1,?2,?3)",
                params![
                    snapshot.id,
                    i64::try_from(snapshot.fetched_at)?,
                    serde_json::to_string(&snapshot)?
                ],
            )?;
            Ok(())
        })
        .await
    }
    async fn latest_fx(&self) -> anyhow::Result<Option<FxSnapshot>> {
        self.account_db(|db|{let raw:Option<String>=db.query_row("SELECT snapshot FROM exchange_rate_snapshots ORDER BY fetched_at DESC,rowid DESC LIMIT 1",[],|r|r.get(0)).optional()?;raw.map(|s|serde_json::from_str(&s).map_err(Into::into)).transpose()}).await
    }
}
