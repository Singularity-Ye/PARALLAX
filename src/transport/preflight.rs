use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{
        ports::{PreflightRepository, StrategyRepository},
        preflight::Preflights,
    },
    domain::preflight::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn router<R: PreflightRepository + StrategyRepository + 'static>(
    state: Arc<Preflights<R>>,
) -> Router {
    Router::new()
        .route("/api/preflights", get(list::<R>).post(create::<R>))
        .route("/api/preflights/{id}", get(detail::<R>))
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut res = next.run(req).await;
                res.headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                res
            },
        ))
        .with_state(state)
}
async fn create<R: PreflightRepository + StrategyRepository>(
    State(app): State<Arc<Preflights<R>>>,
    body: Result<Json<PreflightRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Preflight>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.create(input(body)?).await.map_err(app_error)?),
    ))
}
async fn list<R: PreflightRepository + StrategyRepository>(
    State(app): State<Arc<Preflights<R>>>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({"items":app.list().await.map_err(app_error)?})))
}
async fn detail<R: PreflightRepository + StrategyRepository>(
    State(app): State<Arc<Preflights<R>>>,
    Path(id): Path<String>,
) -> Result<Json<Preflight>, ApiError> {
    Ok(Json(app.get(id).await.map_err(app_error)?))
}
#[cfg(test)]
#[path = "preflight_tests.rs"]
mod tests;
