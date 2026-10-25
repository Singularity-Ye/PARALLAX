//! Saved-evidence research only: no market providers or credential store dependency.
use super::{AppError, ports::*};
use crate::domain::{holding_research::*, selection::now};
use std::sync::Arc;
pub struct HoldingResearchService<R> {
    repo: R,
}
impl<R: AccountRepository + StrategyRepository + HistoryRepository + HoldingResearchRepository>
    HoldingResearchService<R>
{
    pub fn new(repo: R) -> Arc<Self> {
        Arc::new(Self { repo })
    }
    async fn account(&self, id: &str) -> Result<(), AppError> {
        self.repo
            .account(id.into())
            .await?
            .ok_or(AppError::Missing("账户不存在"))?;
        Ok(())
    }
    pub async fn create(
        &self,
        account: &str,
        request: HoldingResearchRequest,
    ) -> Result<HoldingResearchReport, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        self.account(account).await?;
        let strategy = self
            .repo
            .strategy_version(request.strategy_id.clone(), request.revision)
            .await?
            .ok_or(AppError::Missing("策略版本不存在"))?;
        let mut histories = Vec::new();
        for id in &request.history_snapshot_ids {
            histories.push(
                self.repo
                    .history(id.clone())
                    .await?
                    .filter(|h| h.account_id == account)
                    .ok_or(AppError::Missing("账户价格历史快照不存在"))?,
            );
        }
        let report =
            evaluate(account, strategy, request, histories, now()).map_err(AppError::Invalid)?;
        self.repo.insert_holding_research(report.clone()).await?;
        Ok(report)
    }
    pub async fn list(&self, account: &str) -> Result<Vec<HoldingResearchSummary>, AppError> {
        self.account(account).await?;
        Ok(self.repo.holding_researches(account.into()).await?)
    }
    pub async fn get(&self, account: &str, id: String) -> Result<HoldingResearchReport, AppError> {
        self.account(account).await?;
        self.repo
            .holding_research(id)
            .await?
            .filter(|r| r.account_id == account)
            .ok_or(AppError::Missing("账户持有研究不存在"))
    }
}
