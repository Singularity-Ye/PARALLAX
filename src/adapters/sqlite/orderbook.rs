use super::Store;
use crate::{application::ports::OrderBookRepository, domain::orderbook::OrderBookSnapshot};
use rusqlite::{OptionalExtension, params};

impl OrderBookRepository for Store {
    async fn insert_order_book(&self, snapshot: OrderBookSnapshot) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute(
                "INSERT INTO steam_order_books(id,observed_at,snapshot) VALUES(?1,?2,?3)",
                params![
                    snapshot.id,
                    i64::try_from(snapshot.observed_at)?,
                    serde_json::to_string(&snapshot)?
                ],
            )?;
            Ok(())
        })
        .await
    }
    async fn order_book(&self, id: String) -> anyhow::Result<Option<OrderBookSnapshot>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM steam_order_books WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
                .transpose()
        })
        .await
    }
    async fn order_books(&self) -> anyhow::Result<Vec<OrderBookSnapshot>> {
        self.account_db(|db| {
            let mut stmt=db.prepare("SELECT snapshot FROM steam_order_books ORDER BY observed_at DESC,rowid DESC LIMIT 10")?;
            stmt.query_map([],|r|r.get::<_,String>(0))?.map(|row|Ok(serde_json::from_str(&row?)?)).collect()
        }).await
    }
}
