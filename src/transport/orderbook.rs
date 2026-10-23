use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{orderbook::OrderBookResearch, ports::*},
    domain::orderbook::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
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
        .layer(DefaultBodyLimit::max(2048))
        .layer(axum::middleware::from_fn(same_origin))
        .with_state(state)
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
