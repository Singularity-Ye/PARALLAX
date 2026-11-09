use crate::application::{
    deals::SteamDealsService,
    gift::SteamGiftService,
    ports::{AccountRepository, CredentialStore},
};
use crate::domain::gift::GiftCheckRequest;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

pub struct DealsAndGiftState<A, C> {
    pub deals: Arc<SteamDealsService>,
    pub gift: Arc<SteamGiftService<A, C>>,
}

#[derive(Deserialize)]
pub struct AppIdQuery {
    pub appid: Option<u64>,
}

pub fn router<A, C>(deals: Arc<SteamDealsService>, gift: Arc<SteamGiftService<A, C>>) -> Router
where
    A: AccountRepository + 'static,
    C: CredentialStore + 'static,
{
    let state = Arc::new(DealsAndGiftState { deals, gift });

    Router::new()
        .route("/api/steam-deals/popular", get(get_popular_deals::<A, C>))
        .route("/api/steam-deals/compare", get(compare_game_deals::<A, C>))
        .route("/api/accounts/{account}/friends", get(get_friends::<A, C>))
        .route("/api/gift/editions", get(get_game_editions::<A, C>))
        .route("/api/gift/check", post(check_gift::<A, C>))
        .with_state(state)
}

async fn get_popular_deals<A, C>(
    State(state): State<Arc<DealsAndGiftState<A, C>>>,
) -> impl IntoResponse
where
    A: AccountRepository + 'static,
    C: CredentialStore + 'static,
{
    let items = state.deals.popular_games();
    (StatusCode::OK, Json(serde_json::json!({ "games": items })))
}

async fn compare_game_deals<A, C>(
    State(state): State<Arc<DealsAndGiftState<A, C>>>,
    Query(q): Query<AppIdQuery>,
) -> impl IntoResponse
where
    A: AccountRepository + 'static,
    C: CredentialStore + 'static,
{
    let appid = match q.appid {
        Some(id) if id > 0 => id,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "请提供有效的 appid 参数" }))),
    };

    match state.deals.compare_game(appid).await {
        Ok(res) => (StatusCode::OK, Json(serde_json::to_value(res).unwrap_or_default())),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        ),
    }
}

async fn get_friends<A, C>(
    State(state): State<Arc<DealsAndGiftState<A, C>>>,
    Path(account): Path<String>,
) -> impl IntoResponse
where
    A: AccountRepository + 'static,
    C: CredentialStore + 'static,
{
    match state.gift.get_friends(&account).await {
        Ok(friends) => (StatusCode::OK, Json(serde_json::json!({ "friends": friends }))),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err, "friends": [] })),
        ),
    }
}

async fn get_game_editions<A, C>(
    State(state): State<Arc<DealsAndGiftState<A, C>>>,
    Query(q): Query<AppIdQuery>,
) -> impl IntoResponse
where
    A: AccountRepository + 'static,
    C: CredentialStore + 'static,
{
    let appid = match q.appid {
        Some(id) if id > 0 => id,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "请提供有效的 appid 参数" }))),
    };

    match state.gift.get_editions(appid).await {
        Ok(res) => (StatusCode::OK, Json(serde_json::to_value(res).unwrap_or_default())),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        ),
    }
}

async fn check_gift<A, C>(
    State(state): State<Arc<DealsAndGiftState<A, C>>>,
    Json(req): Json<GiftCheckRequest>,
) -> impl IntoResponse
where
    A: AccountRepository + 'static,
    C: CredentialStore + 'static,
{
    match state.gift.check_gift_risk(req).await {
        Ok(res) => (StatusCode::OK, Json(serde_json::to_value(res).unwrap_or_default())),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        ),
    }
}
