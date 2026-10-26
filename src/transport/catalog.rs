use crate::{
    application::{
        catalog::Catalog,
        ports::{CatalogError, CatalogProvider},
    },
    domain::catalog::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
};
use std::sync::Arc;
pub fn router<P: CatalogProvider + 'static>(service: Arc<Catalog<P>>) -> Router {
    Router::new()
        .route("/api/catalog/search", post(search::<P>))
        .with_state(service)
        .layer(DefaultBodyLimit::max(1024))
        .layer(axum::middleware::from_fn(super::http::same_origin))
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
async fn search<P: CatalogProvider>(
    State(service): State<Arc<Catalog<P>>>,
    body: Result<Json<CatalogRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<CatalogPage>, super::http::ApiError> {
    let request = super::http::input(body)?;
    service.search(request).await.map(Json).map_err(|error| {
        let (status, message, retry_at) = match error {
            CatalogError::Invalid(m) => (StatusCode::BAD_REQUEST, m, None),
            CatalogError::Busy => (StatusCode::CONFLICT, "已有检索进行中，请等待".into(), None),
            CatalogError::RateLimited(t) => (
                StatusCode::TOO_MANY_REQUESTS,
                "检索冷却中，请稍后重试".into(),
                Some(t),
            ),
            CatalogError::Upstream(m) => (StatusCode::BAD_GATEWAY, m, None),
        };
        (
            status,
            Json(serde_json::json!({"error":message,"retry_at":retry_at})),
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
    #[derive(Clone)]
    struct Fixture(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl CatalogProvider for Fixture {
        async fn search(&self, r: &CatalogRequest) -> Result<CatalogPage, CatalogError> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(CatalogPage {
                query: r.query.clone(),
                start: r.start,
                received: 0,
                upstream_total: 0,
                next_start: None,
                items: vec![],
                observed_at: 1,
                source: "fixture".into(),
                scope: "single_page".into(),
            })
        }
    }
    #[tokio::test]
    async fn catalog_http_validates_before_fetch_and_throttles_without_network() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let app = router(Catalog::new(Fixture(calls.clone())));
        for (body, origin, status) in [
            (
                r#"{"query":"Case"}"#,
                "https://example.invalid",
                StatusCode::FORBIDDEN,
            ),
            (
                r#"{"query":" "}"#,
                "http://localhost",
                StatusCode::BAD_REQUEST,
            ),
            (
                r#"{"query":"Case","start":1001}"#,
                "http://localhost",
                StatusCode::BAD_REQUEST,
            ),
            (
                r#"{"query":"Case","extra":true}"#,
                "http://localhost",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (r#"{"query":"Case"}"#, "http://localhost", StatusCode::OK),
            (
                r#"{"query":"Case","start":10}"#,
                "http://localhost",
                StatusCode::TOO_MANY_REQUESTS,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/api/catalog/search")
                        .header("host", "localhost")
                        .header("origin", origin)
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status);
            assert_eq!(response.headers()["cache-control"], "no-store");
            if status == StatusCode::TOO_MANY_REQUESTS {
                let value: serde_json::Value =
                    serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap())
                        .unwrap();
                assert!(value["retry_at"].as_u64().is_some());
            }
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
