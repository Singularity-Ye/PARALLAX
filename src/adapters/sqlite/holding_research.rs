use super::Store;
use crate::{application::ports::HoldingResearchRepository, domain::holding_research::*};
use rusqlite::{OptionalExtension, params};
impl HoldingResearchRepository for Store {
    async fn insert_holding_research(&self, r: HoldingResearchReport) -> anyhow::Result<()> {
        self.account_db(move|db|{db.execute("INSERT INTO holding_research(id,account_id,created_at,snapshot) VALUES(?1,?2,?3,?4)",params![r.id,r.account_id,i64::try_from(r.created_at)?,serde_json::to_string(&r)?])?;Ok(())}).await
    }
    async fn holding_research(&self, id: String) -> anyhow::Result<Option<HoldingResearchReport>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM holding_research WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
        })
        .await
    }
    async fn holding_researches(
        &self,
        account: String,
    ) -> anyhow::Result<Vec<HoldingResearchSummary>> {
        self.account_db(move|db|{let mut s=db.prepare("SELECT snapshot FROM holding_research WHERE account_id=?1 ORDER BY created_at DESC,rowid DESC LIMIT 10")?;s.query_map([account],|r|r.get::<_,String>(0))?.map(|r|Ok(HoldingResearchSummary::from(serde_json::from_str::<HoldingResearchReport>(&r?)?))).collect()}).await
    }
}
