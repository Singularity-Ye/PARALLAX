use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{
        ports::{AccountRepository, GuardSecretStore},
        steam_guard::SteamGuard,
    },
    domain::steam_guard::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    routing::{get, post},
};
use std::sync::Arc;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    shared_secret: String,
}
pub fn router<R: AccountRepository + 'static, V: GuardSecretStore + 'static>(
    s: Arc<SteamGuard<R, V>>,
) -> Router {
    Router::new()
        .route(
            "/api/accounts/{id}/steam-guard",
            get(status::<R, V>)
                .post(import::<R, V>)
                .delete(remove::<R, V>),
        )
        .route("/api/accounts/{id}/steam-guard/code", post(code::<R, V>))
        .layer(DefaultBodyLimit::max(512))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |r: axum::extract::Request, n: axum::middleware::Next| async move {
                let mut response = n.run(r).await;
                response
                    .headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                response
            },
        ))
        .with_state(s)
}
async fn status<R: AccountRepository, V: GuardSecretStore>(
    State(s): State<Arc<SteamGuard<R, V>>>,
    Path(id): Path<String>,
) -> Result<Json<GuardStatus>, ApiError> {
    Ok(Json(s.status(&id).await.map_err(app_error)?))
}
async fn import<R: AccountRepository, V: GuardSecretStore>(
    State(s): State<Arc<SteamGuard<R, V>>>,
    Path(id): Path<String>,
    body: Result<Json<Import>, JsonRejection>,
) -> Result<Json<GuardStatus>, ApiError> {
    let body = input(body)?;
    Ok(Json(
        s.import(&id, &body.shared_secret)
            .await
            .map_err(app_error)?,
    ))
}
async fn code<R: AccountRepository, V: GuardSecretStore>(
    State(s): State<Arc<SteamGuard<R, V>>>,
    Path(id): Path<String>,
) -> Result<Json<GuardCode>, ApiError> {
    Ok(Json(s.code(&id).await.map_err(app_error)?))
}
async fn remove<R: AccountRepository, V: GuardSecretStore>(
    State(s): State<Arc<SteamGuard<R, V>>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    s.remove(&id).await.map_err(app_error)?;
    Ok(Json(
        serde_json::json!({"removed":true,"scope":"local_secret_only"}),
    ))
}
