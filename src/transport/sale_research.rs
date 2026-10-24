use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::sale_research::{SaleResearchService, SaleStore},
    domain::sale_research::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn router<R: SaleStore + 'static>(app: Arc<SaleResearchService<R>>) -> Router {
    Router::new()
        .route(
            "/api/accounts/{account}/sale-research",
            get(list::<R>).post(create::<R>),
        )
        .route(
            "/api/accounts/{account}/sale-research/{id}",
            get(detail::<R>),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut res = next.run(req).await;
                res.headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                res
            },
        ))
        .with_state(app)
}
async fn create<R: SaleStore>(
    State(app): State<Arc<SaleResearchService<R>>>,
    Path(account): Path<String>,
    body: Result<Json<SaleResearchRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SaleResearch>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.create(account, input(body)?).await.map_err(app_error)?),
    ))
}
async fn list<R: SaleStore>(
    State(app): State<Arc<SaleResearchService<R>>>,
    Path(account): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":app.list(account).await.map_err(app_error)?}),
    ))
}
async fn detail<R: SaleStore>(
    State(app): State<Arc<SaleResearchService<R>>>,
    Path((account, id)): Path<(String, String)>,
) -> Result<Json<SaleResearch>, ApiError> {
    Ok(Json(app.get(account, id).await.map_err(app_error)?))
}
