use super::Store;
use crate::{application::ports::StrategyRepository, domain::strategy::*};
use rusqlite::{OptionalExtension, params};

impl StrategyRepository for Store {
    async fn append_strategy(
        &self,
        version: StrategyVersion,
        expected_revision: u32,
    ) -> anyhow::Result<bool> {
        self.account_db(move |db| {
            if expected_revision.checked_add(1) != Some(version.revision) { anyhow::bail!("invalid revision transition"); }
            // One SQL statement is atomic across connections/processes as well as this Store's mutex.
            let count = db.execute("INSERT INTO strategy_versions(strategy_id,revision,created_at,snapshot)
                SELECT ?1,?2,?3,?4 WHERE COALESCE((SELECT MAX(revision) FROM strategy_versions WHERE strategy_id=?1),0)=?5",
                params![version.strategy_id, version.revision, i64::try_from(version.created_at)?, serde_json::to_string(&version)?, expected_revision])?;
            Ok(count == 1)
        }).await
    }
    async fn strategy_version(
        &self,
        id: String,
        revision: u32,
    ) -> anyhow::Result<Option<StrategyVersion>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM strategy_versions WHERE strategy_id=?1 AND revision=?2",
                    params![id, revision],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
        })
        .await
    }
    async fn strategy_heads(&self) -> anyhow::Result<Vec<StrategyVersion>> {
        self.account_db(|db| {
            let mut stmt=db.prepare("SELECT s.snapshot FROM strategy_versions s WHERE revision=(SELECT MAX(revision) FROM strategy_versions WHERE strategy_id=s.strategy_id) ORDER BY created_at DESC, rowid DESC LIMIT 50")?;
            stmt.query_map([], |r| r.get::<_,String>(0))?.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
        }).await
    }
    async fn strategy_versions(&self, id: String) -> anyhow::Result<Vec<StrategyVersion>> {
        self.account_db(move |db| {
            let mut stmt=db.prepare("SELECT snapshot FROM strategy_versions WHERE strategy_id=?1 ORDER BY revision DESC LIMIT 100")?;
            stmt.query_map([id], |r| r.get::<_,String>(0))?.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
        }).await
    }
    async fn insert_evaluation(&self, evaluation: StrategyEvaluation) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute(
                "INSERT INTO strategy_evaluations(id,created_at,snapshot) VALUES(?1,?2,?3)",
                params![
                    evaluation.id,
                    i64::try_from(evaluation.created_at)?,
                    serde_json::to_string(&evaluation)?
                ],
            )?;
            Ok(())
        })
        .await
    }
    async fn evaluation(&self, id: String) -> anyhow::Result<Option<StrategyEvaluation>> {
        self.account_db(move |db| {
            let raw: Option<String> = db
                .query_row(
                    "SELECT snapshot FROM strategy_evaluations WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .optional()?;
            raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
        })
        .await
    }
    async fn evaluations(&self) -> anyhow::Result<Vec<StrategyEvaluationSummary>> {
        self.account_db(|db| {
            let mut stmt=db.prepare("SELECT snapshot FROM strategy_evaluations ORDER BY created_at DESC, rowid DESC LIMIT 20")?;
            stmt.query_map([], |r| r.get::<_,String>(0))?.map(|r| Ok(StrategyEvaluationSummary::from(serde_json::from_str::<StrategyEvaluation>(&r?)?))).collect()
        }).await
    }
}
