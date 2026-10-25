use super::http::{ApiError, app_error, input, same_origin};
use crate::{
    application::{ports::*, watchlist::WatchService},
    domain::watchlist::*,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    routing::get,
};
use serde_json::{Value, json};
use std::sync::Arc;
pub fn router<R: AccountRepository + HoldingResearchRepository + WatchRepository + 'static>(
    state: Arc<WatchService<R>>,
) -> Router {
    Router::new()
        .route(
            "/api/accounts/{account}/watchlist",
            get(list::<R>).post(create::<R>),
        )
        .route(
            "/api/accounts/{account}/watchlist/{id}/observations",
            get(observations::<R>).post(observe::<R>),
        )
        .layer(DefaultBodyLimit::max(4096))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |req: axum::extract::Request, next: axum::middleware::Next| async move {
                let mut r = next.run(req).await;
                r.headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                r
            },
        ))
        .with_state(state)
}
async fn list<R: AccountRepository + HoldingResearchRepository + WatchRepository>(
    State(s): State<Arc<WatchService<R>>>,
    Path(a): Path<String>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(json!({"items":s.list(&a).await.map_err(app_error)?})))
}
async fn create<R: AccountRepository + HoldingResearchRepository + WatchRepository>(
    State(s): State<Arc<WatchService<R>>>,
    Path(a): Path<String>,
    b: Result<Json<WatchRequest>, JsonRejection>,
) -> Result<Json<WatchItem>, ApiError> {
    Ok(Json(s.create(&a, input(b)?).await.map_err(app_error)?))
}
async fn observations<R: AccountRepository + HoldingResearchRepository + WatchRepository>(
    State(s): State<Arc<WatchService<R>>>,
    Path((a, id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"items":s.observations(&a,&id).await.map_err(app_error)?}),
    ))
}
async fn observe<R: AccountRepository + HoldingResearchRepository + WatchRepository>(
    State(s): State<Arc<WatchService<R>>>,
    Path((a, id)): Path<(String, String)>,
    b: Result<Json<ObserveRequest>, JsonRejection>,
) -> Result<Json<WatchObservation>, ApiError> {
    Ok(Json(
        s.observe(&a, &id, input(b)?).await.map_err(app_error)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::sqlite::Store;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn watch_http_enforces_origin_body_and_missing_account() {
        let db = Store::open(std::path::Path::new(":memory:")).unwrap();
        let app = router(WatchService::new(db));
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/accounts/missing/watchlist")
                    .header("origin", "https://other.invalid")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"item_key":"Case","reason":"observe"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = app
            .clone()
            .oneshot(
                Request::get("/api/accounts/missing/watchlist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let response = app
            .oneshot(
                Request::post("/api/accounts/missing/watchlist")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"item_key":"Case","reason":"observe","unknown":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
