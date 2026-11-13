use super::http::{ApiError, app_error, input, same_origin};
use crate::application::assets::{CleanupRequest, IconError, ItemIconCache};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Query, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct IconQuery {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

pub fn router(cache: Arc<ItemIconCache>) -> Router {
    let icon_routes = Router::new()
        .route("/api/assets/icon", get(get_icon))
        .with_state(cache.clone());

    let maintenance_routes = Router::new()
        .route("/api/maintenance/stats", get(stats))
        .route("/api/maintenance/cleanup", post(cleanup))
        .layer(DefaultBodyLimit::max(4096))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut response = next.run(req).await;
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            },
        ))
        .with_state(cache);

    icon_routes.merge(maintenance_routes)
}

async fn get_icon(
    State(cache): State<Arc<ItemIconCache>>,
    Query(query): Query<IconQuery>,
) -> Response {
    match cache
        .resolve_icon(&query.name, query.url.as_deref())
        .await
    {
        Ok(icon) => {
            let ct = HeaderValue::from_str(&icon.content_type)
                .unwrap_or_else(|_| HeaderValue::from_static("image/png"));
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, ct),
                    (
                        header::CACHE_CONTROL,
                        HeaderValue::from_static("private, max-age=86400"),
                    ),
                    (
                        header::X_CONTENT_TYPE_OPTIONS,
                        HeaderValue::from_static("nosniff"),
                    ),
                ],
                icon.bytes,
            )
                .into_response()
        }
        Err(IconError::Invalid(msg)) => (
            StatusCode::BAD_REQUEST,
            [(header::CACHE_CONTROL, "no-store")],
            Json(json!({"error": msg})),
        )
            .into_response(),
        Err(IconError::NotFound) => (
            StatusCode::NOT_FOUND,
            [(header::CACHE_CONTROL, "no-store")],
            "not found",
        )
            .into_response(),
        Err(IconError::Internal(err)) => {
            eprintln!("Asset icon cache error: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CACHE_CONTROL, "no-store")],
                "internal error",
            )
                .into_response()
        }
    }
}

async fn stats(State(cache): State<Arc<ItemIconCache>>) -> Result<Json<Value>, ApiError> {
    let stats = cache.stats().await.map_err(app_error)?;
    Ok(Json(serde_json::to_value(stats).unwrap_or_else(|_| json!({}))))
}

async fn cleanup(
    State(cache): State<Arc<ItemIconCache>>,
    body: Result<Json<CleanupRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let req = input(body)?;
    let report = cache.cleanup(req).await.map_err(app_error)?;
    Ok(Json(serde_json::to_value(report).unwrap_or_else(|_| json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::sqlite::Store;
    use axum::{body::{Body, to_bytes}, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn maintenance_endpoints_enforce_same_origin_and_report_stats() {
        let root = std::env::temp_dir().join(format!("parallax-http-assets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db_path = root.join("parallax.db");
        let store = Store::open(&db_path).unwrap();
        let cache = ItemIconCache::new(store, &root, db_path).unwrap();
        let app = router(cache);

        let demo_icon = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/assets/icon?name=demo-0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(demo_icon.status(), StatusCode::NOT_FOUND);

        let stats_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/maintenance/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stats_res.status(), StatusCode::OK);
        assert_eq!(stats_res.headers()["cache-control"], "no-store");

        let cross = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/maintenance/cleanup")
                    .header("host", "127.0.0.1:38480")
                    .header("origin", "https://untrusted.test")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"clear_icons":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cross.status(), StatusCode::FORBIDDEN);

        let clean = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/maintenance/cleanup")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"clear_icons":true,"clear_logs":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(clean.status(), StatusCode::OK);
        let bytes = to_bytes(clean.into_body(), 65536).await.unwrap();
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["cleared_icon_files"], 0);

        let _ = std::fs::remove_dir_all(root);
    }
}
