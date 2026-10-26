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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn proxy_api_reports_limits_and_rejects_cross_site_or_unknown_nodes() {
        let app = router();
        let response = app
            .clone()
            .oneshot(
                Request::get("/api/network/proxies")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(value["browser_routed"], false);
        assert_eq!(value["automatic_rotation"], false);
        assert_eq!(value["requires_restart"], true);
        assert!(value.get("url").is_none());
        for (header, value, expected) in [
            ("origin", "https://example.invalid", StatusCode::FORBIDDEN),
            ("sec-fetch-site", "cross-site", StatusCode::FORBIDDEN),
            ("origin", "http://127.0.0.1:38480", StatusCode::BAD_REQUEST),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/network/proxies/nonexistent-fixture/probe")
                        .header("host", "127.0.0.1:38480")
                        .header(header, value)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
    }
}
