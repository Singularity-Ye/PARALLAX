use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{buff_orders::BuffOrderResearch, ports::*},
    domain::buff_orders::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn router<R, C, V, P>(state: Arc<BuffOrderResearch<R, C, V, P>>) -> Router
where
    R: AccountRepository + BuffOrderRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    P: BuffOrderProvider + 'static,
{
    Router::new()
        .route(
            "/api/accounts/{account}/buff-order-observations",
            get(list::<R, C, V, P>).post(fetch::<R, C, V, P>),
        )
        .route(
            "/api/accounts/{account}/buff-order-observations/{id}",
            get(detail::<R, C, V, P>),
        )
        .route(
            "/api/accounts/{account}/buff-order-observations/{id}/lookup",
            post(lookup::<R, C, V, P>),
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
    R: AccountRepository + BuffOrderRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: BuffOrderProvider,
>(
    State(app): State<Arc<BuffOrderResearch<R, C, V, P>>>,
    Path(account): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.list(&account).await.map_err(app_error)?}),
    ))
}
async fn fetch<
    R: AccountRepository + BuffOrderRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: BuffOrderProvider,
>(
    State(app): State<Arc<BuffOrderResearch<R, C, V, P>>>,
    Path(account): Path<String>,
    body: Result<Json<OrderHistoryRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<OrderHistorySnapshot>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.fetch(&account, input(body)?).await.map_err(app_error)?),
    ))
}
async fn detail<
    R: AccountRepository + BuffOrderRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: BuffOrderProvider,
>(
    State(app): State<Arc<BuffOrderResearch<R, C, V, P>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<OrderHistorySnapshot>, ApiError> {
    Ok(Json(app.get(&account, id).await.map_err(app_error)?))
}
async fn lookup<
    R: AccountRepository + BuffOrderRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: BuffOrderProvider,
>(
    State(app): State<Arc<BuffOrderResearch<R, C, V, P>>>,
    Path((account, id)): Path<(String, String)>,
    body: Result<Json<OrderLookupRequest>, JsonRejection>,
) -> Result<Json<OrderLookup>, ApiError> {
    Ok(Json(
        app.lookup(&account, id, input(body)?)
            .await
            .map_err(app_error)?,
    ))
}
