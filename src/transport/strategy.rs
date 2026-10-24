use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{ports::*, strategy::StrategyResearch},
    domain::strategy::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionRequest {
    expected_revision: u32,
    draft: StrategyDraft,
}
pub fn router<
    R: StrategyRepository
        + Repository
        + HistoryRepository
        + AccountRepository
        + OrderBookRepository
        + 'static,
>(
    state: Arc<StrategyResearch<R>>,
) -> Router {
    Router::new()
        .route("/api/strategies", get(heads::<R>).post(create::<R>))
        .route(
            "/api/strategies/{id}/versions",
            get(versions::<R>).post(revise::<R>),
        )
        .route(
            "/api/strategies/{id}/versions/{revision}",
            get(version::<R>),
        )
        .route(
            "/api/strategy-evaluations",
            get(list::<R>).post(evaluate::<R>),
        )
        .route("/api/strategy-evaluations/{id}", get(detail::<R>))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(req).await;
                response
                    .headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                response
            },
        ))
        .with_state(state)
}
async fn heads<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({"items":app.heads().await.map_err(app_error)?})))
}
async fn versions<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.versions(id).await.map_err(app_error)?}),
    ))
}
async fn version<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
    Path((id, revision)): Path<(String, u32)>,
) -> Result<Json<StrategyVersion>, ApiError> {
    Ok(Json(app.version(id, revision).await.map_err(app_error)?))
}
async fn create<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
    body: Result<Json<StrategyDraft>, JsonRejection>,
) -> Result<(StatusCode, Json<StrategyVersion>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.create(input(body)?).await.map_err(app_error)?),
    ))
}
async fn revise<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
    Path(id): Path<String>,
    body: Result<Json<RevisionRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<StrategyVersion>), ApiError> {
    let body = input(body)?;
    Ok((
        StatusCode::CREATED,
        Json(
            app.revise(id, body.expected_revision, body.draft)
                .await
                .map_err(app_error)?,
        ),
    ))
}
async fn evaluate<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
    body: Result<Json<StrategyEvaluationRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<StrategyEvaluation>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.evaluate(input(body)?).await.map_err(app_error)?),
    ))
}
async fn list<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({"items":app.list().await.map_err(app_error)?})))
}
async fn detail<
    R: StrategyRepository + Repository + HistoryRepository + AccountRepository + OrderBookRepository,
>(
    State(app): State<Arc<StrategyResearch<R>>>,
    Path(id): Path<String>,
) -> Result<Json<StrategyEvaluation>, ApiError> {
    Ok(Json(app.get(id).await.map_err(app_error)?))
}
#[cfg(test)]
#[path = "strategy_tests.rs"]
mod tests;
