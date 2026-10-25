use super::Store;
use crate::{application::ports::AccountRepository, domain::accounts::*};
use rusqlite::{Connection, OptionalExtension, params};

impl Store {
    pub(super) async fn account_db<T: Send + 'static>(
        &self,
        f: impl FnOnce(&Connection) -> anyhow::Result<T> + Send + 'static,
    ) -> anyhow::Result<T> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || {
            let db = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            f(&db)
        })
        .await?
    }
}
impl AccountRepository for Store {
    async fn try_buff_read(
        &self,
        at: u64,
    ) -> anyhow::Result<crate::application::ports::BuffReadPermit> {
        use crate::application::ports::BuffReadPermit;
        self.account_db(move |db| {
            let tx=rusqlite::Transaction::new_unchecked(db,rusqlite::TransactionBehavior::Immediate)?;
            let raw:Option<String>=tx.query_row("SELECT value FROM auth_policy WHERE key='buff'",[],|r|r.get(0)).optional()?;
            let (retry,blocked): (u64,bool)=raw.map(|s|serde_json::from_str(&s)).transpose()?.unwrap_or((0,false));
            if blocked { return Ok(BuffReadPermit::Blocked); }
            if retry>at { return Ok(BuffReadPermit::CoolingUntil(retry)); }
            let next=at.checked_add(2).ok_or_else(||anyhow::anyhow!("pacing time overflow"))?;
            tx.execute("INSERT INTO auth_policy(key,value) VALUES('buff',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(&(next,false))?])?;
            tx.commit()?;Ok(BuffReadPermit::Allowed)
        }).await
    }

    async fn add_account(&self, account: Account) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute(
                "INSERT INTO accounts(id, steam_id, metadata) VALUES(?1,?2,?3)",
                params![
                    account.id,
                    account.steam_id,
                    serde_json::to_string(&account)?
                ],
            )?;
            Ok(())
        })
        .await
    }
    async fn accounts(&self) -> anyhow::Result<Vec<Account>> {
        self.account_db(|db| {
            let mut stmt = db.prepare("SELECT metadata FROM accounts ORDER BY rowid")?;
            stmt.query_map([], |r| r.get::<_, String>(0))?
                .map(|row| Ok(serde_json::from_str(&row?)?))
                .collect()
        })
        .await
    }
    async fn account(&self, id: String) -> anyhow::Result<Option<Account>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row("SELECT metadata FROM accounts WHERE id=?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
                .transpose()
        })
        .await
    }
    async fn session(
        &self,
        id: String,
        platform: Platform,
    ) -> anyhow::Result<Option<StoredSession>> {
        self.account_db(move |db| {
            let raw: Option<(String,String)> = db.query_row("SELECT metadata, credential_key FROM account_sessions WHERE account_id=?1 AND platform=?2", params![id, platform.key()], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
            raw.map(|(s, key)| Ok(StoredSession { view: serde_json::from_str(&s)?, credential_key:key })).transpose()
        }).await
    }
    async fn save_session(&self, id: String, session: StoredSession) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute("INSERT INTO account_sessions(account_id,platform,metadata,credential_key) VALUES(?1,?2,?3,?4) ON CONFLICT(account_id,platform) DO UPDATE SET metadata=excluded.metadata,credential_key=excluded.credential_key", params![id,session.view.platform.key(),serde_json::to_string(&session.view)?,session.credential_key])?;
            Ok(())
        }).await
    }
    async fn sessions(&self, id: String) -> anyhow::Result<Vec<StoredSession>> {
        self.account_db(move |db| {
            let mut stmt = db.prepare("SELECT metadata,credential_key FROM account_sessions WHERE account_id=?1 ORDER BY platform")?;
            stmt.query_map([id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.map(|row| {
                let (s,key)=row?;
                Ok(StoredSession { view:serde_json::from_str(&s)?,credential_key:key })
            }).collect()
        }).await
    }
    async fn buff_policy(&self) -> anyhow::Result<(u64, bool)> {
        self.account_db(|db| {
            let raw: Option<String> = db
                .query_row("SELECT value FROM auth_policy WHERE key='buff'", [], |r| {
                    r.get(0)
                })
                .optional()?;
            raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
                .transpose()
                .map(|v| v.unwrap_or((0, false)))
        })
        .await
    }
    async fn set_buff_policy(&self, at: u64, blocked: bool) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute("INSERT INTO auth_policy(key,value) VALUES('buff',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(&(at,blocked))?])?;
            Ok(())
        }).await
    }
}
