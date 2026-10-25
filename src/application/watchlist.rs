use super::{AppError, ports::*};
use crate::domain::watch_comparison::*;
use crate::domain::{selection::now, watchlist::*};
use std::sync::Arc;
pub struct WatchService<R> {
    repo: R,
}
impl<R: AccountRepository + HoldingResearchRepository + WatchRepository> WatchService<R> {
    pub fn new(repo: R) -> Arc<Self> {
        Arc::new(Self { repo })
    }
    async fn account(&self, a: &str) -> Result<(), AppError> {
        self.repo
            .account(a.into())
            .await?
            .ok_or(AppError::Missing("账户不存在"))?;
        Ok(())
    }
    pub async fn list(&self, a: &str) -> Result<Vec<WatchItem>, AppError> {
        self.account(a).await?;
        Ok(self.repo.watches(a.into()).await?)
    }
    pub async fn create(&self, a: &str, r: WatchRequest) -> Result<WatchItem, AppError> {
        r.validate().map_err(AppError::Invalid)?;
        self.account(a).await?;
        Ok(self
            .repo
            .save_watch(WatchItem {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: a.into(),
                app_id: 730,
                item_key: r.item_key,
                reason: r.reason,
                created_at: now(),
            })
            .await?)
    }
    async fn item(&self, a: &str, id: &str) -> Result<WatchItem, AppError> {
        self.account(a).await?;
        self.repo
            .watch(a.into(), id.into())
            .await?
            .ok_or(AppError::Missing("账户关注商品不存在"))
    }
    pub async fn observe(
        &self,
        a: &str,
        id: &str,
        r: ObserveRequest,
    ) -> Result<WatchObservation, AppError> {
        let item = self.item(a, id).await?;
        let report = self
            .repo
            .holding_research(r.research_id)
            .await?
            .filter(|r| r.account_id == a)
            .ok_or(AppError::Missing("账户持有研究不存在"))?;
        if !report.items.iter().any(|i| i.item_key == item.item_key) {
            return Err(AppError::Invalid("研究不含该关注商品".into()));
        }
        Ok(self
            .repo
            .save_observation(WatchObservation {
                id: uuid::Uuid::new_v4().to_string(),
                watch_id: id.into(),
                account_id: a.into(),
                recorded_at: now(),
                research: report,
            })
            .await?)
    }

    pub async fn baseline(&self, a: &str, id: &str) -> Result<Option<WatchBaseline>, AppError> {
        self.item(a, id).await?;
        Ok(self.repo.watch_baseline(a.into(), id.into()).await?)
    }
    pub async fn fix_baseline(
        &self,
        a: &str,
        id: &str,
        r: BaselineRequest,
    ) -> Result<WatchBaseline, AppError> {
        let observations = self.observations(a, id).await?;
        let observation = observations
            .into_iter()
            .find(|o| o.id == r.observation_id)
            .ok_or(AppError::Missing("关注商品观察不存在"))?;
        let saved = self
            .repo
            .fix_watch_baseline(WatchBaseline {
                account_id: a.into(),
                watch_id: id.into(),
                fixed_at: now(),
                observation,
            })
            .await?;
        if saved.observation.id != r.observation_id {
            return Err(AppError::Conflict("基准已固定，不能以另一观察覆盖"));
        }
        Ok(saved)
    }
    pub async fn compare(
        &self,
        a: &str,
        id: &str,
        r: CompareRequest,
    ) -> Result<WatchComparison, AppError> {
        let item = self.item(a, id).await?;
        let baseline = self
            .baseline(a, id)
            .await?
            .ok_or(AppError::Missing("请先固定观察基准"))?;
        let later = self
            .observations(a, id)
            .await?
            .into_iter()
            .find(|o| o.id == r.observation_id)
            .ok_or(AppError::Missing("关注商品观察不存在"))?;
        compare(&item, &baseline, &later).map_err(AppError::Invalid)
    }
    pub async fn observations(&self, a: &str, id: &str) -> Result<Vec<WatchObservation>, AppError> {
        self.item(a, id).await?;
        Ok(self.repo.observations(a.into(), id.into()).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::sqlite::Store,
        domain::{
            accounts::Account,
            holding_research::{HoldingResearchRequest, evaluate},
            strategy::{ResearchObjective, ResearchStatus, tests::fixture},
        },
    };
    #[tokio::test]
    async fn watches_preserve_reason_outcomes_account_binding_and_survive_reopen() {
        let path = std::env::temp_dir().join(format!("parallax-watch-{}.db", uuid::Uuid::new_v4()));
        let db = Store::open(&path).unwrap();
        let at = now();
        let (mut s, _, h, _) = fixture(at);
        s.draft.schema_version = 3;
        s.draft.objective = ResearchObjective::Holding;
        s.draft.rules.remove(0);
        let a = h.account_id.clone();
        for id in [&a, "other"] {
            db.add_account(Account {
                id: id.into(),
                label: "test".into(),
                steam_id: "76561198000000001".into(),
                created_at: at,
            })
            .await
            .unwrap();
        }
        let service = WatchService::new(db.clone());
        let w = service
            .create(
                &a,
                WatchRequest {
                    item_key: h.request.item_key.clone(),
                    reason: "original reason".into(),
                },
            )
            .await
            .unwrap();
        let again = service
            .create(
                &a,
                WatchRequest {
                    item_key: w.item_key.clone(),
                    reason: "replacement".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(again.id, w.id);
        assert_eq!(again.reason, w.reason);
        assert!(service.observations("other", &w.id).await.is_err());
        let mut saved = Vec::new();
        for status in [
            ResearchStatus::Pass,
            ResearchStatus::Fail,
            ResearchStatus::Unknown,
        ] {
            let req = HoldingResearchRequest {
                strategy_id: s.strategy_id.clone(),
                revision: s.revision,
                history_snapshot_ids: vec![h.id.clone()],
            };
            let mut report = evaluate(&a, s.clone(), req, vec![h.clone()], at).unwrap();
            report.status = status;
            report.items[0].status = status;
            db.insert_holding_research(report.clone()).await.unwrap();
            let req = ObserveRequest {
                research_id: report.id.clone(),
            };
            let o = service.observe(&a, &w.id, req.clone()).await.unwrap();
            assert_eq!(service.observe(&a, &w.id, req).await.unwrap().id, o.id);
            assert_eq!(o.research.items[0].status, status);
            saved.push(o);
        }
        let different = service
            .create(
                &a,
                WatchRequest {
                    item_key: "different".into(),
                    reason: "mismatch".into(),
                },
            )
            .await
            .unwrap();
        assert!(
            service
                .observe(
                    &a,
                    &different.id,
                    ObserveRequest {
                        research_id: saved[0].research.id.clone()
                    }
                )
                .await
                .is_err()
        );
        let foreign = service
            .create(
                "other",
                WatchRequest {
                    item_key: w.item_key.clone(),
                    reason: "other".into(),
                },
            )
            .await
            .unwrap();
        assert!(
            service
                .observe(
                    "other",
                    &foreign.id,
                    ObserveRequest {
                        research_id: saved[0].research.id.clone()
                    }
                )
                .await
                .is_err()
        );
        assert_eq!(service.observations(&a, &w.id).await.unwrap().len(), 3);
        let baseline = service
            .fix_baseline(
                &a,
                &w.id,
                BaselineRequest {
                    observation_id: saved[0].id.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            service
                .fix_baseline(
                    &a,
                    &w.id,
                    BaselineRequest {
                        observation_id: saved[0].id.clone()
                    }
                )
                .await
                .unwrap()
                .fixed_at,
            baseline.fixed_at
        );
        assert!(matches!(
            service
                .fix_baseline(
                    &a,
                    &w.id,
                    BaselineRequest {
                        observation_id: saved[1].id.clone()
                    }
                )
                .await,
            Err(AppError::Conflict(_))
        ));
        assert!(service.baseline("other", &w.id).await.is_err());
        assert!(
            service
                .compare(
                    &a,
                    &w.id,
                    CompareRequest {
                        observation_id: saved[0].id.clone()
                    }
                )
                .await
                .is_err()
        );
        // A genuinely later history is compared through the same persisted use case.
        let (mut later_strategy, _, mut later_history, _) = fixture(at + 86400);
        later_strategy.draft = s.draft.clone();
        later_history.id = "later-history".into();
        let report = evaluate(
            &a,
            later_strategy.clone(),
            HoldingResearchRequest {
                strategy_id: later_strategy.strategy_id.clone(),
                revision: later_strategy.revision,
                history_snapshot_ids: vec![later_history.id.clone()],
            },
            vec![later_history],
            at + 86400,
        )
        .unwrap();
        db.insert_holding_research(report.clone()).await.unwrap();
        let later = service
            .observe(
                &a,
                &w.id,
                ObserveRequest {
                    research_id: report.id,
                },
            )
            .await
            .unwrap();
        let comparison = service
            .compare(
                &a,
                &w.id,
                CompareRequest {
                    observation_id: later.id.clone(),
                },
            )
            .await
            .unwrap();
        assert!(comparison.new_completed_day);
        assert_eq!(comparison.historical_price_change_bps, Some(0));
        assert!(!comparison.returns_validated);
        saved.push(later);
        drop(service);
        drop(db);
        let db = Store::open(&path).unwrap();
        let restored = WatchService::new(db.clone())
            .observations(&a, &w.id)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(saved).unwrap()
        );
        assert_eq!(
            serde_json::to_value(
                WatchService::new(db.clone())
                    .baseline(&a, &w.id)
                    .await
                    .unwrap()
                    .unwrap()
            )
            .unwrap(),
            serde_json::to_value(baseline).unwrap()
        );
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn request_rejects_ambiguous_name_and_empty_reason() {
        for (name, reason) in [
            (" name", "valid"),
            ("name", " "),
            ("name\n", "valid"),
            ("", "valid"),
        ] {
            assert!(
                WatchRequest {
                    item_key: name.into(),
                    reason: reason.into()
                }
                .validate()
                .is_err()
            );
        }
    }
}
