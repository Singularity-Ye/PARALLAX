use super::Store;
use crate::domain::accounts::{Platform, SessionStatus};
use crate::{application::ports::BuffOrderRepository, domain::buff_orders::OrderHistorySnapshot};
use rusqlite::{OptionalExtension, params};
impl BuffOrderRepository for Store {
    async fn insert_buff_orders(&self, v: OrderHistorySnapshot) -> anyhow::Result<bool> {
        self.account_db(move |db| {
            let tx=rusqlite::Transaction::new_unchecked(db,rusqlite::TransactionBehavior::Immediate)?;
            let context=super::preflight::account_context(&tx,&v.account_id)?;
            let matching=context.is_some_and(|c|c.sessions.iter().any(|s|
                s.platform==Platform::Buff && s.status==SessionStatus::Valid
                && s.generation==v.credential_generation && s.subject.as_deref()==Some(v.principal.as_str())));
            if !matching { return Ok(false); }
            tx.execute("INSERT INTO buff_order_observations(id,account_id,observed_at,snapshot) VALUES(?1,?2,?3,?4)",
                params![v.id,v.account_id,i64::try_from(v.observed_at)?,serde_json::to_string(&v)?])?;
            tx.commit()?;
            Ok(true)
        }).await
    }
    async fn buff_orders(&self, id: String) -> anyhow::Result<Option<OrderHistorySnapshot>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM buff_order_observations WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
        })
        .await
    }
    async fn buff_order_observations(
        &self,
        account: String,
    ) -> anyhow::Result<Vec<OrderHistorySnapshot>> {
        self.account_db(move |db| {
            let mut stmt=db.prepare("SELECT snapshot FROM buff_order_observations WHERE account_id=?1 ORDER BY observed_at DESC,rowid DESC LIMIT 10")?;
            stmt.query_map([account],|r|r.get::<_,String>(0))?.map(|r|Ok(serde_json::from_str(&r?)?)).collect()
        }).await
    }
}
