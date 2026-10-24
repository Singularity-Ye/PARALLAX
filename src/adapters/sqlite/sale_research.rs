use super::Store;
use crate::{application::ports::SaleResearchRepository, domain::sale_research::*};
use rusqlite::{OptionalExtension, params};
impl SaleResearchRepository for Store {
    async fn insert_sale_research(&self, v: SaleResearch) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute(
                "INSERT INTO sale_research(id,account_id,created_at,snapshot) VALUES(?1,?2,?3,?4)",
                params![
                    v.id,
                    v.account_id,
                    i64::try_from(v.created_at)?,
                    serde_json::to_string(&v)?
                ],
            )?;
            Ok(())
        })
        .await
    }
    async fn sale_research(
        &self,
        account: String,
        id: String,
    ) -> anyhow::Result<Option<SaleResearch>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM sale_research WHERE account_id=?1 AND id=?2",
                    params![account, id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
        })
        .await
    }
    async fn sale_researches(&self, account: String) -> anyhow::Result<Vec<SaleResearchSummary>> {
        self.account_db(move|db|{
        let mut stmt=db.prepare("SELECT snapshot FROM sale_research WHERE account_id=?1 ORDER BY created_at DESC,rowid DESC LIMIT 20")?;
        stmt.query_map([account],|r|r.get::<_,String>(0))?.map(|r|Ok(SaleResearchSummary::from(serde_json::from_str::<SaleResearch>(&r?)?))).collect()}).await
    }
}
