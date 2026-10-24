use super::{
    AppError,
    ports::{PreflightRepository, StrategyRepository},
};
use crate::domain::{preflight::*, selection::now};
use std::sync::Arc;

/// Storage-only inspection: intentionally cannot access a vault or platform provider.
pub struct Preflights<R> {
    repo: R,
}
impl<R: PreflightRepository + StrategyRepository> Preflights<R> {
    pub fn new(repo: R) -> Arc<Self> {
        Arc::new(Self { repo })
    }
    pub async fn create(&self, request: PreflightRequest) -> Result<Preflight, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let source = self
            .repo
            .evaluation(request.evaluation_id.clone())
            .await?
            .ok_or(AppError::Missing("研究记录不存在"))?;
        let context = if let Some(id) = &request.account_id {
            Some(
                self.repo
                    .preflight_context(id.clone())
                    .await?
                    .ok_or(AppError::Missing("账户不存在"))?,
            )
        } else {
            None
        };
        let result = inspect(request, source, context, now()).map_err(AppError::Invalid)?;
        self.repo.insert_preflight(result.clone()).await?;
        Ok(result)
    }
    pub async fn get(&self, id: String) -> Result<Preflight, AppError> {
        self.repo
            .preflight(id)
            .await?
            .ok_or(AppError::Missing("预检记录不存在"))
    }
    pub async fn list(&self) -> Result<Vec<PreflightSummary>, AppError> {
        Ok(self.repo.preflights().await?)
    }
}
