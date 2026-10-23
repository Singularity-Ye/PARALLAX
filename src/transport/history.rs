use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{history::HistoryResearch, ports::*},
    domain::history::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use serde_json::{Value, json};
use std::sync::Arc;

pub fn router<R, C, V, P>(state: Arc<HistoryResearch<R, C, V, P>>) -> Router
where
    R: AccountRepository + Repository + HistoryRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    P: HistoryProvider + 'static,
{
    Router::new()
        .route(
            "/api/accounts/{account}/price-histories",
            get(list::<R, C, V, P>).post(fetch::<R, C, V, P>),
        )
        .route(
            "/api/accounts/{account}/price-histories/{id}",
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
    R: AccountRepository + Repository + HistoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: HistoryProvider,
>(
    State(app): State<Arc<HistoryResearch<R, C, V, P>>>,
    Path(account): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.list(&account).await.map_err(app_error)?}),
    ))
}
async fn fetch<
    R: AccountRepository + Repository + HistoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: HistoryProvider,
>(
    State(app): State<Arc<HistoryResearch<R, C, V, P>>>,
    Path(account): Path<String>,
    body: Result<Json<HistoryRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<HistorySnapshot>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.fetch(&account, input(body)?).await.map_err(app_error)?),
    ))
}
async fn detail<
    R: AccountRepository + Repository + HistoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: HistoryProvider,
>(
    State(app): State<Arc<HistoryResearch<R, C, V, P>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<HistorySnapshot>, ApiError> {
    Ok(Json(app.get(&account, id).await.map_err(app_error)?))
}
