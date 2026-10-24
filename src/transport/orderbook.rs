use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{
        orderbook::{ListingPreviewRequest, ListingPricePreview, OrderBookResearch},
        ports::*,
    },
    domain::money::Currency,
    domain::orderbook::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::{StatusCode, header},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

pub fn router<P: OrderBookProvider + 'static, R: OrderBookRepository + 'static>(
    state: Arc<OrderBookResearch<P, R>>,
) -> Router {
    Router::new()
        .route(
            "/api/steam/order-books",
            get(list::<P, R>).post(fetch::<P, R>),
        )
        .route("/api/steam/order-books/{id}", get(detail::<P, R>))
        .route(
            "/api/steam/order-books/{id}/valuation",
            post(valuation::<P, R>),
        )
        .route(
            "/api/steam/order-books/{id}/listing-preview",
            post(listing_preview::<P, R>),
        )
        .layer(DefaultBodyLimit::max(2048))
        .layer(axum::middleware::from_fn(same_origin))
        .with_state(state)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValuationRequest {
    #[serde(default = "Currency::cny")]
    display_currency: Currency,
}
async fn valuation<P: OrderBookProvider, R: OrderBookRepository>(
    State(app): State<Arc<OrderBookResearch<P, R>>>,
    Path(id): Path<String>,
    body: Result<Json<ValuationRequest>, JsonRejection>,
) -> Result<
    (
        [(header::HeaderName, &'static str); 1],
        Json<crate::application::orderbook::BookValuation>,
    ),
    ApiError,
> {
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(
            app.valuation(id, input(body)?.display_currency)
                .await
                .map_err(app_error)?,
        ),
    ))
}
async fn listing_preview<P: OrderBookProvider, R: OrderBookRepository>(
    State(app): State<Arc<OrderBookResearch<P, R>>>,
    Path(id): Path<String>,
    body: Result<Json<ListingPreviewRequest>, JsonRejection>,
) -> Result<
    (
        [(header::HeaderName, &'static str); 1],
        Json<ListingPricePreview>,
    ),
    ApiError,
> {
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(
            app.listing_preview(id, input(body)?)
                .await
                .map_err(app_error)?,
        ),
    ))
}
async fn list<P: OrderBookProvider, R: OrderBookRepository>(
    State(app): State<Arc<OrderBookResearch<P, R>>>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({"items":app.list().await.map_err(app_error)?})))
}
async fn fetch<P: OrderBookProvider, R: OrderBookRepository>(
    State(app): State<Arc<OrderBookResearch<P, R>>>,
    body: Result<Json<OrderBookRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<OrderBookSnapshot>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.fetch(input(body)?).await.map_err(app_error)?),
    ))
}
async fn detail<P: OrderBookProvider, R: OrderBookRepository>(
    State(app): State<Arc<OrderBookResearch<P, R>>>,
    Path(id): Path<String>,
) -> Result<Json<OrderBookSnapshot>, ApiError> {
    Ok(Json(app.get(id).await.map_err(app_error)?))
}
