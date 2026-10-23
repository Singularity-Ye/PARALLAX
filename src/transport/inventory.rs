use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{inventory::InventoryResearch, ports::*},
    domain::inventory::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use serde_json::{Value, json};
use std::sync::Arc;

pub fn router<R, C, V, P>(state: Arc<InventoryResearch<R, C, V, P>>) -> Router
where
    R: AccountRepository + InventoryRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    P: InventoryProvider + 'static,
{
    Router::new()
        .route(
            "/api/accounts/{account}/inventories",
            get(list::<R, C, V, P>).post(fetch::<R, C, V, P>),
        )
        .route(
            "/api/accounts/{account}/inventories/{id}",
            get(detail::<R, C, V, P>),
        )
        .layer(DefaultBodyLimit::max(2048))
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
async fn list<
    R: AccountRepository + InventoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: InventoryProvider,
>(
    State(app): State<Arc<InventoryResearch<R, C, V, P>>>,
    Path(account): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.list(&account).await.map_err(app_error)?}),
    ))
}
async fn fetch<
    R: AccountRepository + InventoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: InventoryProvider,
>(
    State(app): State<Arc<InventoryResearch<R, C, V, P>>>,
    Path(account): Path<String>,
    body: Result<Json<InventoryRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<InventorySnapshot>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.fetch(&account, input(body)?).await.map_err(app_error)?),
    ))
}
async fn detail<
    R: AccountRepository + InventoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: InventoryProvider,
>(
    State(app): State<Arc<InventoryResearch<R, C, V, P>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<InventorySnapshot>, ApiError> {
    Ok(Json(app.get(&account, id).await.map_err(app_error)?))
}
