use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{
        accounts::Accounts,
        ports::{AccountRepository, CredentialStore, SessionVerifier},
    },
    domain::accounts::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateAccount {
    label: String,
    steam_id: String,
}
// Never derive Debug or Serialize on request bodies containing secrets.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportSession {
    cookies: String,
    #[serde(default)]
    user_agent: String,
}

pub fn router<R, C, V>(state: Arc<Accounts<R, C, V>>) -> Router
where
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
{
    Router::new()
        .route(
            "/api/accounts",
            get(list::<R, C, V>).post(create::<R, C, V>),
        )
        .route("/api/accounts/{id}/sessions", get(sessions::<R, C, V>))
        .route(
            "/api/accounts/{id}/sessions/{platform}",
            post(import::<R, C, V>).delete(revoke::<R, C, V>),
        )
        .route(
            "/api/accounts/{id}/sessions/{platform}/verify",
            post(verify::<R, C, V>),
        )
        .layer(DefaultBodyLimit::max(8 * 1024))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |request: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(request).await;
                response
                    .headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                response
            },
        ))
        .with_state(state)
}
async fn list<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({"items":app.list().await.map_err(app_error)?})))
}
async fn create<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    body: Result<Json<CreateAccount>, JsonRejection>,
) -> Result<(StatusCode, Json<Account>), ApiError> {
    let body = input(body)?;
    Ok((
        StatusCode::CREATED,
        Json(
            app.create(body.label, body.steam_id)
                .await
                .map_err(app_error)?,
        ),
    ))
}
async fn sessions<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.sessions(&id).await.map_err(app_error)?}),
    ))
}
async fn import<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    Path((id, platform)): Path<(String, Platform)>,
    body: Result<Json<ImportSession>, JsonRejection>,
) -> Result<Json<SessionView>, ApiError> {
    let body = body.map(|Json(value)| value).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"登录态请求格式不正确；需要 cookies 和可选 user_agent 字符串"})),
        )
    })?;
    Ok(Json(
        app.import(
            &id,
            platform,
            Credentials {
                cookies: body.cookies,
                user_agent: body.user_agent,
            },
        )
        .await
        .map_err(app_error)?,
    ))
}
async fn verify<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    Path((id, platform)): Path<(String, Platform)>,
) -> Result<Json<SessionView>, ApiError> {
    Ok(Json(app.verify(&id, platform).await.map_err(app_error)?))
}
async fn revoke<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    Path((id, platform)): Path<(String, Platform)>,
) -> Result<Json<SessionView>, ApiError> {
    Ok(Json(app.revoke(&id, platform).await.map_err(app_error)?))
}
