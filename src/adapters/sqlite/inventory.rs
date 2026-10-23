use super::Store;
use crate::{application::ports::InventoryRepository, domain::inventory::InventorySnapshot};
use rusqlite::{OptionalExtension, params};
impl InventoryRepository for Store {
    async fn insert_inventory(&self, snapshot: InventorySnapshot) -> anyhow::Result<()> {
        self.account_db(move|db| {
            db.execute("INSERT INTO inventory_snapshots(id,account_id,observed_at,snapshot) VALUES(?1,?2,?3,?4)",params![snapshot.id,snapshot.account_id,i64::try_from(snapshot.observed_at)?,serde_json::to_string(&snapshot)?])?;
            Ok(())
        }).await
    }
    async fn inventory(&self, id: String) -> anyhow::Result<Option<InventorySnapshot>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM inventory_snapshots WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
                .transpose()
        })
        .await
    }
    async fn inventories(&self, account_id: String) -> anyhow::Result<Vec<InventorySnapshot>> {
        self.account_db(move|db| {
            let mut stmt=db.prepare("SELECT snapshot FROM inventory_snapshots WHERE account_id=?1 ORDER BY observed_at DESC,rowid DESC LIMIT 10")?;
            stmt.query_map([account_id],|r|r.get::<_,String>(0))?.map(|row|Ok(serde_json::from_str(&row?)?)).collect()
        }).await
    }
}
