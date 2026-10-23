use super::http::{ApiError, app_error, same_origin};
use crate::{
    application::{
        browser_login::{BrowserLogins, LoginView},
        ports::*,
    },
    domain::accounts::Platform,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::Arc;

pub fn router<R, C, V, B>(state: Arc<BrowserLogins<R, C, V, B>>) -> Router
where
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    B: BrowserAuth + 'static,
{
    Router::new()
        .route("/api/browser-logins", get(current::<R, C, V, B>))
        .route(
            "/api/accounts/{account}/sessions/{platform}/browser",
            post(start::<R, C, V, B>),
        )
        .route(
            "/api/browser-logins/{id}/finish",
            post(finish::<R, C, V, B>),
        )
        .route(
            "/api/browser-logins/{id}",
            axum::routing::delete(cancel::<R, C, V, B>),
        )
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
async fn current<
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    B: BrowserAuth + 'static,
>(
    State(app): State<Arc<BrowserLogins<R, C, V, B>>>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"item":app.current().await.map_err(app_error)?}),
    ))
}
async fn start<
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    B: BrowserAuth + 'static,
>(
    State(app): State<Arc<BrowserLogins<R, C, V, B>>>,
    Path((account, platform)): Path<(String, Platform)>,
) -> Result<(StatusCode, Json<LoginView>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.start(&account, platform).await.map_err(app_error)?),
    ))
}
async fn finish<
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    B: BrowserAuth + 'static,
>(
    State(app): State<Arc<BrowserLogins<R, C, V, B>>>,
    Path(id): Path<String>,
) -> Result<Json<LoginView>, ApiError> {
    Ok(Json(app.finish(&id).await.map_err(app_error)?))
}
async fn cancel<
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    B: BrowserAuth + 'static,
>(
    State(app): State<Arc<BrowserLogins<R, C, V, B>>>,
    Path(id): Path<String>,
) -> Result<Json<LoginView>, ApiError> {
    Ok(Json(app.cancel(&id).await.map_err(app_error)?))
}
