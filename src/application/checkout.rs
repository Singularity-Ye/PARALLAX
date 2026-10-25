use super::{AppError, ports::CheckoutRepository};
use crate::domain::{checkout::*, funding::*, selection::now};
use std::sync::Arc;
pub struct CheckoutPreparation<R> {
    repo: R,
}
impl<R: CheckoutRepository> CheckoutPreparation<R> {
    pub fn new(repo: R) -> Arc<Self> {
        Arc::new(Self { repo })
    }
    fn result<T>(outcome: CheckoutOutcome<T>) -> Result<CheckoutWrite<T>, AppError> {
        match outcome {
            CheckoutOutcome::Saved(v) => Ok(v),
            CheckoutOutcome::Missing(s) => Err(AppError::Missing(s)),
            CheckoutOutcome::Conflict(s) => Err(AppError::Conflict(s)),
            CheckoutOutcome::Invalid(s) => Err(AppError::Invalid(s)),
        }
    }
    pub async fn observe(
        &self,
        account: String,
        r: FundingRequest,
    ) -> Result<CheckoutWrite<FundingSnapshot>, AppError> {
        r.validate(now()).map_err(AppError::Invalid)?;
        Self::result(self.repo.observe_funding(account, r, now()).await?)
    }
    pub async fn funds(&self, account: String) -> Result<FundingView, AppError> {
        Ok(Self::result(self.repo.funding_view(account, now()).await?)?.item)
    }
    pub async fn prepare(
        &self,
        account: String,
        r: PrepareRequest,
    ) -> Result<CheckoutWrite<PurchaseIntent>, AppError> {
        r.validate().map_err(AppError::Invalid)?;
        Self::result(self.repo.prepare_intent(account, r, now()).await?)
    }
    pub async fn cancel(
        &self,
        account: String,
        id: String,
        r: CancelRequest,
    ) -> Result<CheckoutWrite<PurchaseIntent>, AppError> {
        Self::result(self.repo.cancel_intent(account, id, r, now()).await?)
    }
    pub async fn get(&self, account: String, id: String) -> Result<PurchaseIntent, AppError> {
        self.repo
            .purchase_intent(account, id)
            .await?
            .ok_or(AppError::Missing("账户意图不存在"))
    }
    pub async fn list(&self, account: String) -> Result<Vec<IntentSummary>, AppError> {
        Ok(self.repo.purchase_intents(account).await?)
    }
    pub async fn events(&self, account: String, id: String) -> Result<Vec<IntentEvent>, AppError> {
        self.get(account.clone(), id.clone()).await?;
        Ok(self.repo.intent_events(account, id).await?)
    }
}
