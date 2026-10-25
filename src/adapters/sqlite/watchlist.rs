use super::Store;
use crate::{application::ports::WatchRepository, domain::watchlist::*};
use rusqlite::{OptionalExtension, params};
impl WatchRepository for Store {
    async fn save_watch(&self, w: WatchItem) -> anyhow::Result<WatchItem> {
        self.account_db(move|db|{
 db.execute("INSERT INTO watch_items(id,account_id,item_key,snapshot) VALUES(?1,?2,?3,?4) ON CONFLICT(account_id,item_key) DO NOTHING",params![w.id,w.account_id,w.item_key,serde_json::to_string(&w)?])?;
 let raw:String=db.query_row("SELECT snapshot FROM watch_items WHERE account_id=?1 AND item_key=?2",params![w.account_id,w.item_key],|r|r.get(0))?;Ok(serde_json::from_str(&raw)?) }).await
    }
    async fn watches(&self, a: String) -> anyhow::Result<Vec<WatchItem>> {
        self.account_db(move |db| {
            let mut s = db.prepare(
                "SELECT snapshot FROM watch_items WHERE account_id=?1 ORDER BY rowid DESC",
            )?;
            s.query_map([a], |r| r.get::<_, String>(0))?
                .map(|r| Ok(serde_json::from_str(&r?)?))
                .collect()
        })
        .await
    }
    async fn watch(&self, a: String, id: String) -> anyhow::Result<Option<WatchItem>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM watch_items WHERE account_id=?1 AND id=?2",
                    params![a, id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
        })
        .await
    }
    async fn save_observation(&self, o: WatchObservation) -> anyhow::Result<WatchObservation> {
        self.account_db(move|db|{
 db.execute("INSERT INTO watch_observations(id,account_id,watch_id,research_id,snapshot) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(watch_id,research_id) DO NOTHING",params![o.id,o.account_id,o.watch_id,o.research.id,serde_json::to_string(&o)?])?;
 let raw:String=db.query_row("SELECT snapshot FROM watch_observations WHERE watch_id=?1 AND research_id=?2",params![o.watch_id,o.research.id],|r|r.get(0))?;Ok(serde_json::from_str(&raw)?) }).await
    }
    async fn observations(&self, a: String, id: String) -> anyhow::Result<Vec<WatchObservation>> {
        self.account_db(move|db|{let mut s=db.prepare("SELECT snapshot FROM watch_observations WHERE account_id=?1 AND watch_id=?2 ORDER BY rowid ASC")?;s.query_map(params![a,id],|r|r.get::<_,String>(0))?.map(|r|Ok(serde_json::from_str(&r?)?)).collect()}).await
    }
}
