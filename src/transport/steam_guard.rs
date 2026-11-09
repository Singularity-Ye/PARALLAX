use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{
        accounts::Accounts,
        ports::{AccountRepository, CredentialStore, GuardSecretStore, SessionVerifier},
        steam_guard::SteamGuard,
    },
    domain::steam_guard::*,
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
struct Import {
    #[serde(default)]
    shared_secret: Option<String>,
    #[serde(default)]
    identity_secret: Option<String>,
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    mafile_json: Option<String>,
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
        .layer(DefaultBodyLimit::max(64 * 1024))
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
    let mut body = input(body)?;
    if let Some(raw_json) = body
        .mafile_json
        .as_deref()
        .map(str::trim)
        .filter(|j| !j.is_empty())
    {
        if let Ok(parsed) = serde_json::from_str::<Value>(raw_json) {
            if body.shared_secret.as_deref().unwrap_or_default().trim().is_empty() {
                if let Some(ss) = parsed["shared_secret"].as_str() {
                    body.shared_secret = Some(ss.to_owned());
                }
            }
            if body.identity_secret.as_deref().unwrap_or_default().trim().is_empty() {
                if let Some(is) = parsed["identity_secret"].as_str() {
                    body.identity_secret = Some(is.to_owned());
                }
            }
            if body.device_id.as_deref().unwrap_or_default().trim().is_empty() {
                if let Some(dev) = parsed["device_id"].as_str() {
                    body.device_id = Some(dev.to_owned());
                }
            }
        }
    }
    // Also detect if user pasted full maFile JSON directly into the shared_secret input box!
    if let Some(raw_ss) = body.shared_secret.clone() {
        let trimmed = raw_ss.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                body.shared_secret = parsed["shared_secret"].as_str().map(str::to_owned);
                if body.identity_secret.as_deref().unwrap_or_default().trim().is_empty() {
                    body.identity_secret = parsed["identity_secret"].as_str().map(str::to_owned);
                }
                if body.device_id.as_deref().unwrap_or_default().trim().is_empty() {
                    body.device_id = parsed["device_id"].as_str().map(str::to_owned);
                }
            }
        }
    }
    if body
        .shared_secret
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
        && body
            .identity_secret
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "请填写 shared_secret、identity_secret 或粘贴完整的 .maFile JSON 内容"})),
        ));
    }
    Ok(Json(
        s.import_full(
            &id,
            body.shared_secret.as_deref(),
            body.identity_secret.as_deref(),
            body.device_id.as_deref(),
        )
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
) -> Result<Json<Value>, ApiError> {
    s.remove(&id).await.map_err(app_error)?;
    Ok(Json(
        json!({"removed":true,"scope":"local_secret_only"}),
    ))
}

struct LiveMarketState<R, C, SV, GV> {
    accounts: Arc<Accounts<R, C, SV>>,
    guard: Arc<SteamGuard<R, GV>>,
    fx: Option<Arc<dyn crate::application::fx::FxResolver>>,
}
impl<R, C, SV, GV> Clone for LiveMarketState<R, C, SV, GV> {
    fn clone(&self) -> Self {
        Self {
            accounts: self.accounts.clone(),
            guard: self.guard.clone(),
            fx: self.fx.clone(),
        }
    }
}

#[derive(Deserialize)]
struct ConfirmRequest {
    #[serde(default)]
    ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SellItemRequest {
    asset_id: String,
    /// Buyer-pays price already expressed in the wallet's own minor units.
    #[serde(default)]
    buyer_pays_minor: Option<i64>,
    /// Buyer-pays price in CNY cents (strategy basis); converted to the live wallet currency server-side.
    #[serde(default)]
    buyer_pays_cny_cents: Option<i64>,
}

#[derive(Deserialize)]
struct DelistItemRequest {
    listing_id: String,
}

pub fn live_market_router<R, C, SV, GV>(
    accounts: Arc<Accounts<R, C, SV>>,
    guard: Arc<SteamGuard<R, GV>>,
    fx: Option<Arc<dyn crate::application::fx::FxResolver>>,
) -> Router
where
    R: AccountRepository + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    let state = LiveMarketState { accounts, guard, fx };
    Router::new()
        .route(
            "/api/accounts/{id}/steam-guard/confirmations",
            get(list_confirmations::<R, C, SV, GV>),
        )
        .route(
            "/api/accounts/{id}/steam-guard/confirm",
            post(accept_confirmations::<R, C, SV, GV>),
        )
        .route(
            "/api/accounts/{id}/steam-market/listings",
            get(get_market_listings::<R, C, SV, GV>),
        )
        .route(
            "/api/accounts/{id}/steam-market/sell",
            post(sell_market_item::<R, C, SV, GV>),
        )
        .route(
            "/api/accounts/{id}/steam-market/delist",
            post(delist_market_item::<R, C, SV, GV>),
        )
        .layer(DefaultBodyLimit::max(32 * 1024))
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
        .with_state(state)
}

async fn list_confirmations<R, C, SV, GV>(
    State(st): State<LiveMarketState<R, C, SV, GV>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError>
where
    R: AccountRepository,
    C: CredentialStore,
    SV: SessionVerifier,
    GV: GuardSecretStore,
{
    let (account, cred) = st.accounts.steam_credentials(&id).await.map_err(app_error)?;
    let (_, identity_secret, device_id) =
        st.guard.identity_credentials(&id).await.map_err(app_error)?;
    let items = crate::adapters::steam_market::get_mobile_confirmations(
        &identity_secret,
        &device_id,
        &account.steam_id,
        &cred.cookies,
    )
    .await
    .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;
    Ok(Json(json!({
        "count": items.len(),
        "items": items,
    })))
}

async fn accept_confirmations<R, C, SV, GV>(
    State(st): State<LiveMarketState<R, C, SV, GV>>,
    Path(id): Path<String>,
    body: Result<Json<ConfirmRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError>
where
    R: AccountRepository,
    C: CredentialStore,
    SV: SessionVerifier,
    GV: GuardSecretStore,
{
    let req = body.map(|Json(v)| v).unwrap_or(ConfirmRequest { ids: None });
    let (account, cred) = st.accounts.steam_credentials(&id).await.map_err(app_error)?;
    let (_, identity_secret, device_id) =
        st.guard.identity_credentials(&id).await.map_err(app_error)?;
    let mut items = crate::adapters::steam_market::get_mobile_confirmations(
        &identity_secret,
        &device_id,
        &account.steam_id,
        &cred.cookies,
    )
    .await
    .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;
    if let Some(filter_ids) = req.ids.as_ref().filter(|v| !v.is_empty()) {
        items.retain(|c| filter_ids.iter().any(|fid| fid == &c.id));
    }
    let confirmed = crate::adapters::steam_market::accept_mobile_confirmations(
        &identity_secret,
        &device_id,
        &account.steam_id,
        &cred.cookies,
        &items,
    )
    .await
    .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;
    Ok(Json(json!({
        "success": true,
        "confirmed": confirmed,
    })))
}

async fn get_market_listings<R, C, SV, GV>(
    State(st): State<LiveMarketState<R, C, SV, GV>>,
    Path(id): Path<String>,
) -> Result<Json<crate::adapters::steam_market::MarketListingsSnapshot>, ApiError>
where
    R: AccountRepository,
    C: CredentialStore,
    SV: SessionVerifier,
    GV: GuardSecretStore,
{
    let (_, cred) = st.accounts.steam_credentials(&id).await.map_err(app_error)?;
    let snap = crate::adapters::steam_market::fetch_my_listings(&cred.cookies)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;
    Ok(Json(snap))
}

async fn sell_market_item<R, C, SV, GV>(
    State(st): State<LiveMarketState<R, C, SV, GV>>,
    Path(id): Path<String>,
    body: Result<Json<SellItemRequest>, JsonRejection>,
) -> Result<Json<crate::adapters::steam_market::SellItemResponse>, ApiError>
where
    R: AccountRepository,
    C: CredentialStore,
    SV: SessionVerifier,
    GV: GuardSecretStore,
{
    let req = input(body)?;
    let bad = |msg: String| (StatusCode::BAD_REQUEST, Json(json!({"error": msg})));
    let (account, cred) = st.accounts.steam_credentials(&id).await.map_err(app_error)?;

    // Settlement currency: re-read the live wallet right before listing (the cache may be stale
    // after a region change), fall back to the cached profile, and refuse if neither is known.
    let cached = st.accounts.profile_meta(&id).await.unwrap_or_default().currency_code;
    #[cfg(windows)]
    let live = crate::adapters::steam_password::fetch_profile_and_wallet(
        &account.steam_id,
        Some(&cred.cookies),
    )
    .await
    .currency_code;
    #[cfg(not(windows))]
    let live = String::new();
    let currency = if !live.is_empty() { live } else { cached }.to_uppercase();
    if currency.is_empty() {
        return Err(bad("无法确认 Steam 钱包结算币种，已拒绝上架以防跨币种误卖；请先在账号页点击验证刷新钱包信息".into()));
    }

    let (buyer_pays_minor, fx_note) = match (req.buyer_pays_minor, req.buyer_pays_cny_cents) {
        (Some(native), None) => (native, None),
        (None, Some(cny)) if currency == "CNY" => (cny, None),
        (None, Some(cny)) => {
            let fx = st
                .fx
                .as_ref()
                .ok_or_else(|| bad("服务未配置汇率源，无法把人民币壁垒价换算为钱包币种".into()))?;
            let snap = fx.rates().await.map_err(app_error)?;
            let rate = |code: &str| snap.rates.get(code).map(|r| (r.units_per_cny_1e8, r.date.clone()));
            let (units, date) = rate(&currency).ok_or_else(|| {
                bad(format!("汇率快照不含钱包币种 {currency}，已拒绝上架以防误卖"))
            })?;
            // Round to nearest minor unit; all supported currencies use hundredths.
            let scale = crate::domain::fx::SCALE as u128;
            let native = ((cny as u128) * (units as u128) + scale / 2) / scale;
            (
                native as i64,
                Some(format!(
                    "CNY {:.2} → {currency} {:.2}（{} 日参考汇率）",
                    cny as f64 / 100.0,
                    native as f64 / 100.0,
                    date
                )),
            )
        }
        _ => {
            return Err(bad(
                "buyer_pays_minor（钱包币种）与 buyer_pays_cny_cents（人民币）需且只需填写一个".into(),
            ));
        }
    };
    if !(3..=100_000_000).contains(&buyer_pays_minor) {
        return Err(bad(format!("换算后的挂牌价 {buyer_pays_minor} 超出合理范围，已拒绝上架")));
    }

    let identity = st
        .guard
        .identity_credentials(&id)
        .await
        .ok()
        .map(|(_, id_sec, dev)| (id_sec, dev));
    let mut res = crate::adapters::steam_market::sell_item_on_market(
        &account.steam_id,
        &cred.cookies,
        &req.asset_id,
        buyer_pays_minor,
        &currency,
        identity,
    )
    .await
    .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;
    res.currency_code = currency;
    if let Some(note) = fx_note {
        res.message = format!("{} · {note}", res.message);
    }
    Ok(Json(res))
}

async fn delist_market_item<R, C, SV, GV>(
    State(st): State<LiveMarketState<R, C, SV, GV>>,
    Path(id): Path<String>,
    body: Result<Json<DelistItemRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError>
where
    R: AccountRepository,
    C: CredentialStore,
    SV: SessionVerifier,
    GV: GuardSecretStore,
{
    let req = input(body)?;
    let (_, cred) = st.accounts.steam_credentials(&id).await.map_err(app_error)?;
    crate::adapters::steam_market::remove_market_listing(&cred.cookies, &req.listing_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;
    Ok(Json(json!({
        "success": true,
        "listing_id": req.listing_id,
    })))
}
