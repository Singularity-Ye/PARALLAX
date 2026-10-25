use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{checkout::CheckoutPreparation, ports::CheckoutRepository},
    domain::{checkout::*, funding::*},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    routing::get,
};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn router<R: CheckoutRepository + 'static>(app: Arc<CheckoutPreparation<R>>) -> Router {
    Router::new()
        .route(
            "/api/accounts/{account}/cash-funding",
            get(funds::<R>).post(observe::<R>),
        )
        .route(
            "/api/accounts/{account}/purchase-intents",
            get(list::<R>).post(prepare::<R>),
        )
        .route(
            "/api/accounts/{account}/purchase-intents/{id}",
            get(detail::<R>),
        )
        .route(
            "/api/accounts/{account}/purchase-intents/{id}/cancel",
            axum::routing::post(cancel::<R>),
        )
        .route(
            "/api/accounts/{account}/purchase-intents/{id}/events",
            get(events::<R>),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut res = next.run(req).await;
                res.headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                res
            },
        ))
        .with_state(app)
}
async fn funds<R: CheckoutRepository>(
    State(app): State<Arc<CheckoutPreparation<R>>>,
    Path(account): Path<String>,
) -> Result<Json<FundingView>, ApiError> {
    Ok(Json(app.funds(account).await.map_err(app_error)?))
}
async fn observe<R: CheckoutRepository>(
    State(app): State<Arc<CheckoutPreparation<R>>>,
    Path(account): Path<String>,
    body: Result<Json<FundingRequest>, JsonRejection>,
) -> Result<Json<CheckoutWrite<FundingSnapshot>>, ApiError> {
    Ok(Json(
        app.observe(account, input(body)?)
            .await
            .map_err(app_error)?,
    ))
}
async fn prepare<R: CheckoutRepository>(
    State(app): State<Arc<CheckoutPreparation<R>>>,
    Path(account): Path<String>,
    body: Result<Json<PrepareRequest>, JsonRejection>,
) -> Result<Json<CheckoutWrite<PurchaseIntent>>, ApiError> {
    Ok(Json(
        app.prepare(account, input(body)?)
            .await
            .map_err(app_error)?,
    ))
}
async fn list<R: CheckoutRepository>(
    State(app): State<Arc<CheckoutPreparation<R>>>,
    Path(account): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.list(account).await.map_err(app_error)?}),
    ))
}
async fn detail<R: CheckoutRepository>(
    State(app): State<Arc<CheckoutPreparation<R>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<PurchaseIntent>, ApiError> {
    Ok(Json(app.get(account, id).await.map_err(app_error)?))
}
async fn cancel<R: CheckoutRepository>(
    State(app): State<Arc<CheckoutPreparation<R>>>,
    Path((account, id)): Path<(String, String)>,
    body: Result<Json<CancelRequest>, JsonRejection>,
) -> Result<Json<CheckoutWrite<PurchaseIntent>>, ApiError> {
    Ok(Json(
        app.cancel(account, id, input(body)?)
            .await
            .map_err(app_error)?,
    ))
}
async fn events<R: CheckoutRepository>(
    State(app): State<Arc<CheckoutPreparation<R>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.events(account,id).await.map_err(app_error)?}),
    ))
}
#[cfg(test)]
#[path = "checkout_tests.rs"]
mod tests;
