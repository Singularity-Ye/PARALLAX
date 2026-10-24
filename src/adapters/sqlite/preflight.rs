use super::Store;
use crate::{application::ports::PreflightRepository, domain::preflight::*};
use rusqlite::{OptionalExtension, params};
impl PreflightRepository for Store {
    async fn preflight_context(&self, id: String) -> anyhow::Result<Option<AccountContext>> {
        self.account_db(move |db| {
            let tx = db.unchecked_transaction()?;
            let raw: Option<String> = tx
                .query_row("SELECT metadata FROM accounts WHERE id=?1", [&id], |r| {
                    r.get(0)
                })
                .optional()?;
            let Some(raw) = raw else { return Ok(None) };
            let sessions = {
                let mut stmt = tx.prepare(
                    "SELECT metadata FROM account_sessions WHERE account_id=?1 ORDER BY platform",
                )?;
                stmt.query_map([&id], |r| r.get::<_, String>(0))?
                    .map(|r| Ok(serde_json::from_str(&r?)?))
                    .collect::<anyhow::Result<Vec<_>>>()?
            };
            let policy: Option<String> = tx
                .query_row("SELECT value FROM auth_policy WHERE key='buff'", [], |r| {
                    r.get(0)
                })
                .optional()?;
            let (buff_retry_at, buff_blocked) = policy
                .map(|s| serde_json::from_str(&s))
                .transpose()?
                .unwrap_or((0, false));
            let result = AccountContext {
                account: serde_json::from_str(&raw)?,
                sessions,
                buff_retry_at,
                buff_blocked,
            };
            tx.commit()?;
            Ok(Some(result))
        })
        .await
    }
    async fn insert_preflight(&self, p: Preflight) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute(
                "INSERT INTO preflight_inspections(id,created_at,snapshot) VALUES(?1,?2,?3)",
                params![
                    p.id,
                    i64::try_from(p.created_at)?,
                    serde_json::to_string(&p)?
                ],
            )?;
            Ok(())
        })
        .await
    }
    async fn preflight(&self, id: String) -> anyhow::Result<Option<Preflight>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM preflight_inspections WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
        })
        .await
    }
    async fn preflights(&self) -> anyhow::Result<Vec<PreflightSummary>> {
        self.account_db(|db| {let mut stmt=db.prepare("SELECT snapshot FROM preflight_inspections ORDER BY created_at DESC,rowid DESC LIMIT 20")?;stmt.query_map([],|r|r.get::<_,String>(0))?.map(|r|Ok(PreflightSummary::from(serde_json::from_str::<Preflight>(&r?)?))).collect()}).await
    }
}
