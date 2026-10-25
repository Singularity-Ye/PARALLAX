use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{holding_research::HoldingResearchService, ports::*},
    domain::holding_research::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn router<
    R: AccountRepository
        + StrategyRepository
        + HistoryRepository
        + HoldingResearchRepository
        + 'static,
>(
    state: Arc<HoldingResearchService<R>>,
) -> Router {
    Router::new()
        .route(
            "/api/accounts/{account}/holding-research",
            get(list::<R>).post(create::<R>),
        )
        .route(
            "/api/accounts/{account}/holding-research/{id}",
            get(detail::<R>),
        )
        .layer(DefaultBodyLimit::max(4096))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut r = next.run(req).await;
                r.headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                r
            },
        ))
        .with_state(state)
}
async fn list<
    R: AccountRepository + StrategyRepository + HistoryRepository + HoldingResearchRepository,
>(
    State(s): State<Arc<HoldingResearchService<R>>>,
    Path(account): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":s.list(&account).await.map_err(app_error)?}),
    ))
}
async fn create<
    R: AccountRepository + StrategyRepository + HistoryRepository + HoldingResearchRepository,
>(
    State(s): State<Arc<HoldingResearchService<R>>>,
    Path(account): Path<String>,
    body: Result<Json<HoldingResearchRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<HoldingResearchReport>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(s.create(&account, input(body)?).await.map_err(app_error)?),
    ))
}
async fn detail<
    R: AccountRepository + StrategyRepository + HistoryRepository + HoldingResearchRepository,
>(
    State(s): State<Arc<HoldingResearchService<R>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<HoldingResearchReport>, ApiError> {
    Ok(Json(s.get(&account, id).await.map_err(app_error)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::sqlite::Store,
        domain::{
            accounts::Account,
            selection::now,
            strategy::{ResearchObjective, tests::fixture},
        },
    };
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn independent_http_is_account_bound_immutable_and_survives_reopen() {
        let path =
            std::env::temp_dir().join(format!("parallax-independent-{}.db", uuid::Uuid::new_v4()));
        let store = Store::open(&path).unwrap();
        let (mut s, _, mut h, _) = fixture(now());
        s.draft.schema_version = 3;
        s.draft.objective = ResearchObjective::Holding;
        s.draft.rules.remove(0);
        h.request.scan_id.clear();
        h.request.manual_item = true;
        h.identity_binding = "manual_market_name_and_request_parameters".into();
        store
            .add_account(Account {
                id: h.account_id.clone(),
                label: "fixture".into(),
                steam_id: "76561198000000001".into(),
                created_at: now(),
            })
            .await
            .unwrap();
        let account = h.account_id.clone();
        store.insert_history(h.clone()).await.unwrap();
        store.append_strategy(s.clone(), 0).await.unwrap();
        let request = HoldingResearchRequest {
            strategy_id: s.strategy_id,
            revision: s.revision,
            history_snapshot_ids: vec![h.id],
        };
        let app = router(HoldingResearchService::new(store.clone()));
        let url = format!("/api/accounts/{account}/holding-research");
        let cross = app
            .clone()
            .oneshot(
                Request::post(&url)
                    .header("origin", "https://elsewhere.invalid")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cross.status(), StatusCode::FORBIDDEN);
        let response = app
            .clone()
            .oneshot(
                Request::post(&url)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let report: HoldingResearchReport =
            serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await.unwrap())
                .unwrap();
        assert_eq!(report.status, crate::domain::strategy::ResearchStatus::Pass);
        assert!(store.insert_holding_research(report.clone()).await.is_err());
        let other = Account {
            id: "other".into(),
            label: "other".into(),
            steam_id: "76561198000000002".into(),
            created_at: now(),
        };
        store.add_account(other).await.unwrap();
        let service = HoldingResearchService::new(store.clone());
        assert!(service.get("other", report.id.clone()).await.is_err());
        assert!(service.create("other", request).await.is_err());
        assert!(service.list("other").await.unwrap().is_empty());
        assert_eq!(service.list(&account).await.unwrap().len(), 1);
        drop(service);
        drop(app);
        drop(store);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(
            serde_json::to_value(
                reopened
                    .holding_research(report.id.clone())
                    .await
                    .unwrap()
                    .unwrap()
            )
            .unwrap(),
            serde_json::to_value(report).unwrap()
        );
        drop(reopened);
        std::fs::remove_file(path).unwrap();
    }
}
