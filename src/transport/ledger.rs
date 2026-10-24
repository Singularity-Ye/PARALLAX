use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::ledger::{LedgerStore, PurchaseLedger},
    domain::{holdings::Holdings, ledger::*},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldingsRequest {
    inventory_snapshot_id: Option<String>,
}
pub fn router<R: LedgerStore + 'static>(app: Arc<PurchaseLedger<R>>) -> Router {
    Router::new()
        .route(
            "/api/accounts/{account}/purchase-lots",
            get(list::<R>).post(create::<R>),
        )
        .route(
            "/api/accounts/{account}/purchase-lots/{id}",
            get(detail::<R>),
        )
        .route(
            "/api/accounts/{account}/purchase-lots/{id}/events",
            get(events::<R>).post(append::<R>),
        )
        .route(
            "/api/accounts/{account}/holdings-preview",
            axum::routing::post(holdings::<R>),
        )
        .layer(DefaultBodyLimit::max(64 * 1024))
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
async fn create<R: LedgerStore>(
    State(app): State<Arc<PurchaseLedger<R>>>,
    Path(account): Path<String>,
    body: Result<Json<LotRequest>, JsonRejection>,
) -> Result<Json<LedgerWrite>, ApiError> {
    Ok(Json(
        app.create(account, input(body)?).await.map_err(app_error)?,
    ))
}
async fn list<R: LedgerStore>(
    State(app): State<Arc<PurchaseLedger<R>>>,
    Path(account): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.list(account).await.map_err(app_error)?}),
    ))
}
async fn detail<R: LedgerStore>(
    State(app): State<Arc<PurchaseLedger<R>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<PurchaseLot>, ApiError> {
    Ok(Json(app.get(account, id).await.map_err(app_error)?))
}
async fn events<R: LedgerStore>(
    State(app): State<Arc<PurchaseLedger<R>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.events(account,id).await.map_err(app_error)?}),
    ))
}
async fn append<R: LedgerStore>(
    State(app): State<Arc<PurchaseLedger<R>>>,
    Path((account, id)): Path<(String, String)>,
    body: Result<Json<LotEventRequest>, JsonRejection>,
) -> Result<Json<LedgerWrite>, ApiError> {
    Ok(Json(
        app.append(account, id, input(body)?)
            .await
            .map_err(app_error)?,
    ))
}
async fn holdings<R: LedgerStore>(
    State(app): State<Arc<PurchaseLedger<R>>>,
    Path(account): Path<String>,
    body: Result<Json<HoldingsRequest>, JsonRejection>,
) -> Result<Json<Holdings>, ApiError> {
    Ok(Json(
        app.holdings(account, input(body)?.inventory_snapshot_id)
            .await
            .map_err(app_error)?,
    ))
}
#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
