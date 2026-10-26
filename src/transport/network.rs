use super::http::{ApiError, same_origin};
use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    routing::{get, post},
};
pub fn router() -> Router {
    Router::new()
        .route(
            "/api/network/proxies",
            get(|| async { Json(crate::adapters::network::view()) }),
        )
        .route("/api/network/proxies/{id}/probe", post(probe))
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
}
async fn probe(Path(id): Path<String>) -> Result<Json<serde_json::Value>, ApiError> {
    static GATE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
    let _gate = GATE.try_acquire().map_err(|_| {
        (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error":"已有代理测试进行中"})),
        )
    })?;
    crate::adapters::network::probe(&id)
        .await
        .map(Json)
        .map_err(|message| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":message})),
            )
        })
}
