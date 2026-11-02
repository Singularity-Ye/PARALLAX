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
        .route("/api/accounts/{id}/profile", get(profile::<R, C, V>))
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

async fn profile<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let account = app.account(&id).await.map_err(app_error)?;
    let steam_id = account.steam_id.trim().to_string();

    let account_id_32 = steam_id
        .parse::<u64>()
        .ok()
        .and_then(|sid| sid.checked_sub(76561197960265728));

    let mut avatar_url: Option<String> = None;
    let mut persona_name: Option<String> = None;

    if let Some(acc_id) = account_id_32 {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
            .unwrap_or_default();

        let steam_url = format!("https://steamcommunity.com/miniprofile/{acc_id}/json");
        if let Ok(resp) = client
            .get(&steam_url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<Value>().await {
                    if let Some(name) = data.get("persona_name").and_then(|v| v.as_str()) {
                        persona_name = Some(name.to_string());
                    }
                    if let Some(mut av) = data
                        .get("avatar_url")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                    {
                        if !av.starts_with("http") {
                            av = format!("https://avatars.steamstatic.com/{av}");
                        }
                        av = av.replace("_medium", "_full");
                        avatar_url = Some(av);
                    }
                }
            }
        }
    }

    if avatar_url.is_none() {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(800))
            .build()
            .unwrap_or_default();
        if let Ok(resp) = client.get("http://127.0.0.1:29472/api/accounts").send().await {
            if resp.status().is_success() {
                if let Ok(data) = resp.json::<Value>().await {
                    if let Some(arr) = data.get("accounts").and_then(|v| v.as_array()) {
                        for acc in arr {
                            if acc.get("steam_id").and_then(|v| v.as_str()) == Some(&steam_id) {
                                if let Some(av) = acc.get("avatar_url").and_then(|v| v.as_str()) {
                                    if !av.is_empty() {
                                        avatar_url = Some(av.to_string());
                                    }
                                }
                                if let Some(dn) = acc.get("display_name").and_then(|v| v.as_str()) {
                                    if !dn.is_empty() && persona_name.is_none() {
                                        persona_name = Some(dn.to_string());
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(json!({
        "account_id": id,
        "steam_id": steam_id,
        "persona_name": persona_name.unwrap_or_else(|| account.label.clone()),
        "avatar_url": avatar_url,
    })))
}
