use super::{AppError, ports::*};
use crate::domain::{
    holdings::{self, Holdings},
    ledger::*,
    selection::now,
};
use std::sync::Arc;
pub struct PurchaseLedger<R> {
    repo: R,
}
pub trait LedgerStore: LedgerRepository + AccountRepository + InventoryRepository {}
impl<T: LedgerRepository + AccountRepository + InventoryRepository> LedgerStore for T {}
impl<R: LedgerStore> PurchaseLedger<R> {
    pub fn new(repo: R) -> Arc<Self> {
        Arc::new(Self { repo })
    }
    async fn account(&self, id: &str) -> Result<crate::domain::accounts::Account, AppError> {
        self.repo
            .account(id.into())
            .await?
            .ok_or(AppError::Missing("账户不存在"))
    }
    pub async fn create(
        &self,
        account: String,
        request: LotRequest,
    ) -> Result<LedgerWrite, AppError> {
        self.account(&account).await?;
        request.validate(now()).map_err(AppError::Invalid)?;
        Self::outcome(self.repo.create_lot(account, request, now()).await?)
    }
    pub async fn get(&self, account: String, id: String) -> Result<PurchaseLot, AppError> {
        self.repo
            .lot(account, id)
            .await?
            .ok_or(AppError::Missing("账户采购批次不存在"))
    }
    pub async fn list(&self, account: String) -> Result<Vec<PurchaseLot>, AppError> {
        self.account(&account).await?;
        Ok(self.repo.lots(account).await?)
    }
    pub async fn events(&self, account: String, id: String) -> Result<Vec<LotEvent>, AppError> {
        self.get(account.clone(), id.clone()).await?;
        Ok(self.repo.lot_events(account, id).await?)
    }
    pub async fn append(
        &self,
        account: String,
        id: String,
        request: LotEventRequest,
    ) -> Result<LedgerWrite, AppError> {
        let owner = self.account(&account).await?;
        let inventory = if let LotAction::Receive {
            inventory_snapshot_id,
            ..
        } = &request.action
        {
            Some(
                self.repo
                    .inventory(inventory_snapshot_id.clone())
                    .await?
                    .filter(|i| i.account_id == account && i.steam_id == owner.steam_id)
                    .ok_or(AppError::Missing("账户库存快照不存在"))?,
            )
        } else {
            None
        };
        Self::outcome(
            self.repo
                .append_lot_event(account, id, request, inventory, now())
                .await?,
        )
    }
    pub async fn holdings(
        &self,
        account: String,
        inventory_id: Option<String>,
    ) -> Result<Holdings, AppError> {
        let owner = self.account(&account).await?;
        let inventory = if let Some(id) = inventory_id {
            Some(
                self.repo
                    .inventory(id)
                    .await?
                    .filter(|i| i.account_id == account && i.steam_id == owner.steam_id)
                    .ok_or(AppError::Missing("账户库存快照不存在"))?,
            )
        } else {
            None
        };
        holdings::summarize(
            &account,
            self.repo.lots(account.clone()).await?,
            inventory,
            now(),
        )
        .map_err(AppError::Invalid)
    }
    fn outcome(result: LedgerOutcome) -> Result<LedgerWrite, AppError> {
        match result {
            LedgerOutcome::Saved(v) => Ok(v),
            LedgerOutcome::Missing => Err(AppError::Missing("账户采购批次不存在")),
            LedgerOutcome::Conflict(e) => Err(AppError::Conflict(e)),
            LedgerOutcome::Invalid(e) => Err(AppError::Invalid(e)),
        }
    }
}
