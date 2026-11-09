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
    #[serde(default)]
    label: String,
    #[serde(default)]
    steam_id: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateAccount {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    steam_id: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
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
        .route(
            "/api/accounts/{id}",
            axum::routing::put(update::<R, C, V>).delete(remove::<R, C, V>),
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
    let has_pwd = body
        .password
        .as_ref()
        .is_some_and(|p| !p.trim().is_empty());
    let account = app
        .create_flexible(body.label, body.steam_id, body.username.clone(), has_pwd)
        .await
        .map_err(app_error)?;
    #[cfg(windows)]
    if let (Some(u), Some(p)) = (
        body.username.as_deref().map(str::trim).filter(|s| !s.is_empty()),
        body.password.as_deref().filter(|s| !s.is_empty()),
    ) {
        let _ = crate::adapters::steam_password::save_password(&account.id, u, p).await;
    }
    Ok((StatusCode::CREATED, Json(account)))
}
async fn update<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    Path(id): Path<String>,
    body: Result<Json<UpdateAccount>, JsonRejection>,
) -> Result<Json<Account>, ApiError> {
    let body = input(body)?;
    let has_new_pwd = body.password.as_deref().is_some_and(|p| !p.is_empty());
    let account = app
        .update(
            &id,
            body.label,
            body.steam_id,
            body.username.clone(),
            if has_new_pwd { Some(true) } else { None },
        )
        .await
        .map_err(app_error)?;
    #[cfg(windows)]
    if has_new_pwd {
        let prof = app.profile_meta(&id).await.unwrap_or_default();
        let uname = body
            .username
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&prof.username);
        if let Some(p) = body.password.as_deref().filter(|s| !s.is_empty()) {
            if !uname.is_empty() {
                let _ = crate::adapters::steam_password::save_password(&id, uname, p).await;
            }
        }
    }
    Ok(Json(account))
}
async fn remove<R: AccountRepository, C: CredentialStore, V: SessionVerifier>(
    State(app): State<Arc<Accounts<R, C, V>>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    app.delete(&id).await.map_err(app_error)?;
    #[cfg(windows)]
    {
        let _ = crate::adapters::steam_password::remove_password(&id).await;
    }
    Ok(Json(json!({"deleted": true, "id": id})))
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
    let mut prof = app.profile_meta(&id).await.unwrap_or_default();

    #[cfg(windows)]
    {
        if !prof.has_saved_password {
            if let Ok(Some((saved_u, _))) = crate::adapters::steam_password::load_password(&id).await {
                prof.has_saved_password = true;
                if prof.username.is_empty() {
                    prof.username = saved_u;
                }
            }
        }
        let need_fetch = prof.avatar_url.is_empty()
            || prof.display_name.is_empty()
            || prof.currency_code.is_empty();
        if need_fetch && steam_id.len() == 17 && steam_id != "00000000000000000" {
            let cookies = app.stored_steam_cookies(&id).await;
            let fetched =
                crate::adapters::steam_password::fetch_profile_and_wallet(&steam_id, cookies.as_deref()).await;
            let mut changed = false;
            if !fetched.display_name.is_empty() && prof.display_name != fetched.display_name {
                prof.display_name = fetched.display_name;
                changed = true;
            }
            if !fetched.avatar_url.is_empty() && prof.avatar_url != fetched.avatar_url {
                prof.avatar_url = fetched.avatar_url;
                changed = true;
            }
            if !fetched.currency_code.is_empty() {
                prof.currency_code = fetched.currency_code;
                prof.region_code = fetched.region_code;
                prof.wallet_balance_display = fetched.wallet_balance_display;
                prof.wallet_balance_raw = fetched.wallet_balance_raw;
                prof.wallet_currency_id = fetched.wallet_currency_id;
                changed = true;
            }
            if changed {
                let _ = app.save_profile_meta(&id, prof.clone()).await;
            }
        }
    }

    if prof.avatar_url.is_empty() || prof.currency_code.is_empty() {
        let mut seed_json: Option<Value> = None;
        for candidate_path in [
            std::path::Path::new("../AetherSwap_reference/config/accounts.json"),
            std::path::Path::new("d:/Yhx06/Documents/仙术工坊——项目集/AetherSwap_reference/config/accounts.json"),
        ] {
            if let Ok(raw) = std::fs::read_to_string(candidate_path) {
                if let Ok(parsed) = serde_json::from_str::<Value>(&raw) {
                    seed_json = Some(parsed);
                    break;
                }
            }
        }
        if seed_json.is_none() {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(800))
                .build()
                .unwrap_or_default();
            if let Ok(resp) = client
                .get("http://127.0.0.1:29472/api/accounts")
                .send()
                .await
            {
                if resp.status().is_success() {
                    seed_json = resp.json::<Value>().await.ok();
                }
            }
        }
        if let Some(data) = seed_json {
            if let Some(arr) = data.get("accounts").and_then(|v| v.as_array()) {
                for acc in arr {
                    if acc.get("steam_id").and_then(|v| v.as_str()) == Some(&steam_id) {
                        if let Some(av) = acc.get("avatar_url").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                            if prof.avatar_url.is_empty() {
                                prof.avatar_url = av.to_string();
                            }
                        }
                        if let Some(dn) = acc.get("display_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                            if prof.display_name.is_empty() {
                                prof.display_name = dn
                                    .replace("&gt;", ">")
                                    .replace("&lt;", "<")
                                    .replace("&amp;", "&");
                            }
                        }
                        if let Some(un) = acc.get("username").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                            if prof.username.is_empty() {
                                prof.username = un.to_string();
                            }
                        }
                        if let Some(cc) = acc.get("currency_code").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                            if prof.currency_code.is_empty() {
                                prof.currency_code = cc.to_string();
                            }
                        }
                        if let Some(rc) = acc.get("region_code").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                            if prof.region_code.is_empty() {
                                prof.region_code = rc.to_string();
                            }
                        }
                        let _ = app.save_profile_meta(&id, prof.clone()).await;
                        break;
                    }
                }
            }
        }
    }

    if prof.display_name.contains("&gt;") || prof.display_name.contains("&lt;") || prof.display_name.contains("&amp;") {
        prof.display_name = prof
            .display_name
            .replace("&gt;", ">")
            .replace("&lt;", "<")
            .replace("&amp;", "&");
        let _ = app.save_profile_meta(&id, prof.clone()).await;
    }

    let persona = if !prof.display_name.is_empty() {
        prof.display_name.clone()
    } else {
        account.label.clone()
    };
    let avatar_opt = if prof.avatar_url.is_empty() {
        None
    } else {
        Some(prof.avatar_url.clone())
    };

    Ok(Json(json!({
        "account_id": id,
        "steam_id": steam_id,
        "username": prof.username,
        "persona_name": persona,
        "avatar_url": avatar_opt,
        "currency_code": prof.currency_code,
        "region_code": prof.region_code,
        "wallet_balance_display": prof.wallet_balance_display,
        "wallet_balance_raw": prof.wallet_balance_raw,
        "has_saved_password": prof.has_saved_password,
    })))
}

#[cfg(windows)]
pub fn steam_password_router(
    accounts: Arc<
        Accounts<
            crate::adapters::sqlite::Store,
            crate::adapters::credentials::NativeVault,
            crate::adapters::session::OnlineVerifier,
        >,
    >,
    guard: Arc<
        crate::application::steam_guard::SteamGuard<
            crate::adapters::sqlite::Store,
            crate::adapters::credentials::NativeVault,
        >,
    >,
) -> Router {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Login {
        #[serde(default)]
        username: String,
        #[serde(default)]
        password: String,
        #[serde(default)]
        guard_code: Option<String>,
        #[serde(default)]
        remember_password: Option<bool>,
    }
    let gate = Arc::new(tokio::sync::Mutex::new(None::<std::time::Instant>));
    Router::new()
        .route("/api/accounts/{id}/sessions/steam/password", post(move |Path(id): Path<String>, body: Result<Json<Login>, JsonRejection>| {
            let (accounts, guard, gate) = (accounts.clone(), guard.clone(), gate.clone());
            async move {
                let Json(Login { username, password, guard_code, remember_password }) = body.map_err(|_| (StatusCode::BAD_REQUEST, Json(json!({"error":"登录请求格式错误"}))))?;
                let mut username = username.trim().to_string();
                let mut password = password;

                // If password was omitted, automatically load saved credentials from Windows Credential Manager!
                if password.is_empty() {
                    if let Ok(Some((saved_u, saved_p))) = crate::adapters::steam_password::load_password(&id).await {
                        if username.is_empty() {
                            username = saved_u;
                        }
                        password = saved_p;
                    }
                }
                if username.is_empty() || username.len() > 128 || password.is_empty() || password.len() > 245 {
                    return Err((StatusCode::BAD_REQUEST, Json(json!({"error":"尚未保存该账户的 Steam 密码；请在下方填写一次 Steam 用户名与密码（将自动加密保存，后续可一键免密重登）"}))));
                }
                let account = accounts.account(&id).await.map_err(app_error)?;
                let mut attempt = gate.try_lock().map_err(|_| (StatusCode::CONFLICT, Json(json!({"error":"已有 Steam 登录正在进行，请稍候"}))))?;
                if attempt.is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(5)) {
                    return Err((StatusCode::TOO_MANY_REQUESTS, Json(json!({"error":"请等待 5 秒后再尝试登录"}))));
                }
                *attempt = Some(std::time::Instant::now());

                // Save username & password to Windows Credential Manager by default so user never has to re-type
                if remember_password.unwrap_or(true) {
                    let _ = crate::adapters::steam_password::save_password(&id, &username, &password).await;
                }

                let id_for_guard = id.clone();
                let guard_ref = guard.clone();
                let login_fut = crate::adapters::steam_password::login_smart(
                    &username,
                    &password,
                    &account.steam_id,
                    guard_code.as_deref(),
                    move |steam_time| async move {
                        guard_ref
                            .code_at(&id_for_guard, steam_time)
                            .await
                            .ok()
                            .map(|value| value.code)
                    },
                );
                let (secret, discovered_sid) = match tokio::time::timeout(std::time::Duration::from_secs(60), login_fut).await {
                    Ok(Ok(res)) => res,
                    Ok(Err(err)) => {
                        return Err((StatusCode::BAD_GATEWAY, Json(json!({"error": err.to_string()}))));
                    }
                    Err(_) => {
                        return Err((StatusCode::BAD_GATEWAY, Json(json!({"error": "Steam 登录握手超时（60秒），请检查网络或手机令牌确认"}))));
                    }
                };

                let cookies_copy = secret.cookies.clone();
                let mut view = accounts
                    .save_verified_login(&id, Platform::Steam, secret, Some(discovered_sid.clone()))
                    .await
                    .map_err(app_error)?;

                // Immediately fetch Steam Avatar, Persona Name, Wallet Balance, Currency & Region!
                let fetched = crate::adapters::steam_password::fetch_profile_and_wallet(&discovered_sid, Some(&cookies_copy)).await;
                let mut prof = accounts.profile_meta(&id).await.unwrap_or_default();
                prof.username = username.clone();
                prof.has_saved_password = remember_password.unwrap_or(true);
                if !fetched.display_name.is_empty() {
                    prof.display_name = fetched.display_name.clone();
                }
                if !fetched.avatar_url.is_empty() {
                    prof.avatar_url = fetched.avatar_url;
                }
                if !fetched.currency_code.is_empty() {
                    prof.currency_code = fetched.currency_code;
                    prof.region_code = fetched.region_code;
                    prof.wallet_balance_display = fetched.wallet_balance_display;
                    prof.wallet_balance_raw = fetched.wallet_balance_raw;
                    prof.wallet_currency_id = fetched.wallet_currency_id;
                }
                let _ = accounts.save_profile_meta(&id, prof.clone()).await;

                // If account label was just the username or placeholder and we discovered a Steam nickname, update label too
                if !prof.display_name.is_empty() && (account.label == username || account.steam_id == "00000000000000000") {
                    let _ = accounts.update(&id, Some(prof.display_name.clone()), Some(discovered_sid), None, None).await;
                }

                let mut extra_info = Vec::new();
                if !prof.display_name.is_empty() {
                    extra_info.push(format!("昵称: {}", prof.display_name));
                }
                if !prof.currency_code.is_empty() {
                    extra_info.push(format!("币种: {} ({})", prof.currency_code, prof.region_code));
                }
                if !prof.wallet_balance_display.is_empty() {
                    extra_info.push(format!("余额: {}", prof.wallet_balance_display));
                }
                if !extra_info.is_empty() {
                    view.message = format!("Steam 登录成功 · {}", extra_info.join(" · "));
                }
                Ok(Json(view))
            }
        }))
        .layer(DefaultBodyLimit::max(2048))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(|request: axum::extract::Request, next: axum::middleware::Next| async move {
            let mut response = next.run(request).await;
            response.headers_mut().insert(axum::http::header::CACHE_CONTROL, axum::http::HeaderValue::from_static("no-store"));
            response
        }))
}
