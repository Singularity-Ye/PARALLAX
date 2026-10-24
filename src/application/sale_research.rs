use super::{AppError, ports::*};
use crate::domain::{sale_research::*, selection::now};
use std::sync::Arc;
pub struct SaleResearchService<R> {
    repo: R,
}
pub trait SaleStore:
    LedgerRepository
    + StrategyRepository
    + InventoryRepository
    + PreflightRepository
    + SaleResearchRepository
{
}
impl<
    T: LedgerRepository
        + StrategyRepository
        + InventoryRepository
        + PreflightRepository
        + SaleResearchRepository,
> SaleStore for T
{
}
impl<R: SaleStore> SaleResearchService<R> {
    pub fn new(repo: R) -> Arc<Self> {
        Arc::new(Self { repo })
    }
    pub async fn create(
        &self,
        account: String,
        request: SaleResearchRequest,
    ) -> Result<SaleResearch, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let lot = self
            .repo
            .lot(account.clone(), request.lot_id.clone())
            .await?
            .ok_or(AppError::Missing("账户采购批次不存在"))?;
        let source = self
            .repo
            .evaluation(request.evaluation_id.clone())
            .await?
            .ok_or(AppError::Missing("研究记录不存在"))?;
        let inventory = self
            .repo
            .inventory(request.inventory_snapshot_id.clone())
            .await?
            .filter(|i| i.account_id == account)
            .ok_or(AppError::Missing("账户库存快照不存在"))?;
        let context = self
            .repo
            .preflight_context(account)
            .await?
            .ok_or(AppError::Missing("账户不存在"))?;
        let result =
            evaluate(request, lot, source, inventory, context, now()).map_err(AppError::Invalid)?;
        self.repo.insert_sale_research(result.clone()).await?;
        Ok(result)
    }
    pub async fn list(&self, account: String) -> Result<Vec<SaleResearchSummary>, AppError> {
        Ok(self.repo.sale_researches(account).await?)
    }
    pub async fn get(&self, account: String, id: String) -> Result<SaleResearch, AppError> {
        self.repo
            .sale_research(account, id)
            .await?
            .ok_or(AppError::Missing("账户出售研究不存在"))
    }
}
