use super::{AppError, ports::*};
use crate::domain::{selection::now, strategy::*};
use std::sync::Arc;

/// Offline research depends only on storage. It cannot read credentials or call a platform.
pub struct StrategyResearch<R> {
    repo: R,
}
impl<R: StrategyRepository + Repository + HistoryRepository + AccountRepository>
    StrategyResearch<R>
{
    pub fn new(repo: R) -> Arc<Self> {
        Arc::new(Self { repo })
    }
    pub async fn create(&self, draft: StrategyDraft) -> Result<StrategyVersion, AppError> {
        self.save(uuid::Uuid::new_v4().to_string(), 0, draft).await
    }
    pub async fn revise(
        &self,
        id: String,
        expected: u32,
        draft: StrategyDraft,
    ) -> Result<StrategyVersion, AppError> {
        if expected == 0 {
            return Err(AppError::Invalid("修改必须指定已有版本".into()));
        }
        self.version(id.clone(), expected).await?;
        self.save(id, expected, draft).await
    }
    async fn save(
        &self,
        id: String,
        expected: u32,
        draft: StrategyDraft,
    ) -> Result<StrategyVersion, AppError> {
        draft.validate().map_err(AppError::Invalid)?;
        let revision = expected
            .checked_add(1)
            .ok_or(AppError::Conflict("策略版本已达上限"))?;
        let version = StrategyVersion {
            strategy_id: id,
            revision,
            created_at: now(),
            draft,
        };
        if !self.repo.append_strategy(version.clone(), expected).await? {
            return Err(AppError::Conflict(
                "策略已被其他操作更新；输入保留，请读取最新版本后再保存",
            ));
        }
        Ok(version)
    }
    pub async fn heads(&self) -> Result<Vec<StrategyVersion>, AppError> {
        Ok(self.repo.strategy_heads().await?)
    }
    pub async fn versions(&self, id: String) -> Result<Vec<StrategyVersion>, AppError> {
        Ok(self.repo.strategy_versions(id).await?)
    }
    pub async fn version(&self, id: String, revision: u32) -> Result<StrategyVersion, AppError> {
        self.repo
            .strategy_version(id, revision)
            .await?
            .ok_or(AppError::Missing("策略版本不存在"))
    }
    pub async fn evaluate(
        &self,
        request: StrategyEvaluationRequest,
    ) -> Result<StrategyEvaluation, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let strategy = self
            .version(request.strategy_id.clone(), request.revision)
            .await?;
        let plan = self
            .repo
            .get_plan(request.plan_id.clone())
            .await?
            .ok_or(AppError::Missing("预算计划不存在"))?;
        if let Some(id) = &request.account_id {
            self.repo
                .account(id.clone())
                .await?
                .ok_or(AppError::Missing("账户不存在"))?;
        }
        let mut histories = vec![];
        for id in &request.history_snapshot_ids {
            let history = self
                .repo
                .history(id.clone())
                .await?
                .filter(|h| Some(&h.account_id) == request.account_id.as_ref())
                .ok_or(AppError::Missing("账户价格历史快照不存在"))?;
            histories.push(history);
        }
        let evaluation =
            evaluate(strategy, plan, histories, request, now()).map_err(AppError::Invalid)?;
        self.repo.insert_evaluation(evaluation.clone()).await?;
        Ok(evaluation)
    }
    pub async fn get(&self, id: String) -> Result<StrategyEvaluation, AppError> {
        self.repo
            .evaluation(id)
            .await?
            .ok_or(AppError::Missing("策略研究记录不存在"))
    }
    pub async fn list(&self) -> Result<Vec<StrategyEvaluationSummary>, AppError> {
        Ok(self.repo.evaluations().await?)
    }
}
