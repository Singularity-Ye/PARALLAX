use super::{AppError, Application, ports::*};
use crate::domain::{
    planning::PlanPreview,
    replanning::{ReplanRequest, origin},
};
impl<
    S: MarketSource + 'static,
    R: Repository
        + AccountRepository
        + OrderBookRepository
        + BuffQuoteRepository
        + StrategyRepository
        + 'static,
> Application<S, R>
{
    pub async fn replan(&self, request: ReplanRequest) -> Result<PlanPreview, AppError> {
        let source = self
            .store
            .evaluation(request.evaluation_id)
            .await?
            .ok_or(AppError::Missing("来源研究不存在，请先保存研究"))?;
        let provenance = origin(&source, &request.request).map_err(AppError::Invalid)?;
        let mut plan = self.prepare_plan(request.request).await?;
        plan.research_origin = Some(provenance);
        plan.execution_blockers
            .push("按原研究选择重新分配；旧通过结论不适用于新数量，必须对本计划重新研究".into());
        self.store.insert_plan(plan.clone()).await?;
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::{market::Provider, sqlite::Store},
        domain::{
            selection::{ScanReport, ScanRequest, now},
            strategy::{ResearchStatus, evaluate, tests::fixture},
        },
    };
    #[tokio::test]
    async fn replanning_preserves_parent_research_scope_and_reopens_without_inheriting_pass() {
        let path =
            std::env::temp_dir().join(format!("parallax-replan-{}.db", uuid::Uuid::new_v4()));
        let db = Store::open(&path).unwrap();
        let at = now();
        let (s, p, h, r) = fixture(at);
        let evaluation = evaluate(s, p.clone(), vec![h], r, at).unwrap();
        assert_eq!(evaluation.items[0].status, ResearchStatus::Pass);
        let mut scan = ScanReport::new(ScanRequest {
            source: p.source,
            max_ratio_bps: 20000,
            ..Default::default()
        });
        scan.id = p.request.scan_id.clone();
        scan.status = "completed".into();
        scan.coverage = p.source_coverage.clone();
        scan.stop_reason = p.source_stop_reason.clone();
        scan.candidates = p.lines.iter().map(|l| l.candidate.clone()).collect();
        let mut other = scan.candidates[0].clone();
        other.item.key = "unresearched".into();
        scan.candidates.push(other);
        db.save(scan).await.unwrap();
        db.insert_plan(p.clone()).await.unwrap();
        db.insert_evaluation(evaluation.clone()).await.unwrap();
        let original = serde_json::to_value(&evaluation).unwrap();
        let app = Application::new(Provider::new().unwrap(), db.clone());
        let mut request = p.request.clone();
        request.selected_item_keys = Some(vec![evaluation.items[0].item_key.clone()]);
        request.max_units_per_item = 3;
        let next = app
            .replan(ReplanRequest {
                evaluation_id: evaluation.id.clone(),
                request: request.clone(),
            })
            .await
            .unwrap();
        assert_ne!(next.id, p.id);
        assert_eq!(next.lines[0].quantity, 3);
        assert_eq!(p.lines[0].quantity, 2);
        assert!(
            next.skipped
                .iter()
                .any(|i| i.key == "unresearched" && i.reason.contains("未被本次明确选择"))
        );
        assert!(
            next.research_origin
                .as_ref()
                .unwrap()
                .new_plan_research_required
        );
        assert_eq!(
            serde_json::to_value(db.evaluation(evaluation.id.clone()).await.unwrap().unwrap())
                .unwrap(),
            original
        );
        for key in ["unresearched", "missing"] {
            let mut invalid = request.clone();
            invalid.selected_item_keys = Some(vec![key.into()]);
            assert!(
                app.replan(ReplanRequest {
                    evaluation_id: evaluation.id.clone(),
                    request: invalid
                })
                .await
                .is_err()
            );
        }
        let mut invalid = request.clone();
        invalid.scan_id = "other scan".into();
        assert!(origin(&evaluation, &invalid).is_err());
        let mut unknown = evaluation.clone();
        unknown.items[0].status = ResearchStatus::Unknown;
        assert!(origin(&unknown, &request).is_err());
        let mut failed = evaluation.clone();
        failed.items[0].status = ResearchStatus::Fail;
        assert!(origin(&failed, &request).is_err());
        request.selected_item_keys = Some(vec![]);
        assert!(origin(&evaluation, &request).is_err());
        use axum::{
            body::{Body, to_bytes},
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;
        let router = crate::transport::http::router(app.clone());
        let payload = serde_json::json!({"evaluation_id":evaluation.id,"request":next.request});
        let cross = router
            .clone()
            .oneshot(
                Request::post("/api/plans/replan")
                    .header("origin", "https://other.invalid")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cross.status(), StatusCode::FORBIDDEN);
        let response = router
            .clone()
            .oneshot(
                Request::post("/api/plans/replan")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await.unwrap())
                .unwrap();
        assert_eq!(json["research_origin"]["evaluation_id"], evaluation.id);
        assert_eq!(json["lines"][0]["quantity"], 3);
        assert_eq!(db.list_plans().await.unwrap().len(), 3);
        drop(router);
        drop(app);
        drop(db);
        let db = Store::open(&path).unwrap();
        let restored = db.get_plan(next.id.clone()).await.unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(next).unwrap()
        );
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}
