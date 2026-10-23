use crate::{
    adapters::{market::Provider, sqlite::Store},
    application::{AppError, Application},
    domain::{
        planning::{PlanPreview, PlanRequest},
        selection::{ScanReport, ScanRequest},
    },
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use std::sync::Arc;

pub type AppState = Arc<Application<Provider, Store>>;
pub(super) type ApiError = (StatusCode, Json<serde_json::Value>);
fn error(status: StatusCode, message: &str) -> ApiError {
    (status, Json(json!({"error":message})))
}
pub(super) fn app_error(err: AppError) -> ApiError {
    match err {
        AppError::Invalid(message) => error(StatusCode::BAD_REQUEST, &message),
        AppError::Missing(message) => error(StatusCode::NOT_FOUND, message),
        AppError::Conflict(message) => error(StatusCode::CONFLICT, message),
        AppError::Market(err) => {
            if let crate::application::ports::MarketReadError::RateLimited(retry_at) = err {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({"error":err.message(),"retry_at":retry_at})),
                )
            } else {
                error(StatusCode::BAD_GATEWAY, err.message())
            }
        }
        AppError::Storage(err) => {
            eprintln!("storage error: {err}");
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "记录存取失败，请检查本地日志；当前操作未确认完成",
            )
        }
    }
}
pub(super) fn input<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value)
        .map_err(|err| error(err.status(), &err.body_text()))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { Html(include_str!("../../web/index.html")) }))
        .route("/app.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../../web/app.js")) }))
        .route("/accounts.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../../web/accounts.js")) }))
        .route("/orderbook.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../../web/orderbook.js")) }))
        .route("/plan-inputs.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../../web/plan-inputs.js")) }))
        .route("/lib/api.js", get(|| async { ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8")], include_str!("../../web/lib/api.js")) }))
        .route("/style.css", get(|| async { ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("../../web/style.css")) }))
        .route("/api/health", get(|| async { Json(json!({"name":"Parallax","version":env!("CARGO_PKG_VERSION"), "api_version":"1",
            "capabilities":{"scan":true,"live_ranking":true,"plan_preview":true,"manual_sessions":true,"browser_login":true,"automatic_login":false,"live_execution":false,"history_analysis":false,"steam_order_books":true}})) }))
        .route("/api/scans", get(list).post(start))
        .route("/api/scans/{id}", get(detail))
        .route("/api/scans/{id}/cancel", post(cancel))
        .route("/api/plans", get(plans).post(create_plan).layer(DefaultBodyLimit::max(2 * 1024 * 1024)))
        .route("/api/plans/{id}", get(plan))
        .route("/api/trades", post(|| async { error(StatusCode::NOT_IMPLEMENTED, "真实交易执行尚未迁移；未创建订单") }))
        .fallback(|| async { error(StatusCode::NOT_FOUND, "接口或资源不存在") })
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(axum::middleware::from_fn(same_origin))
        .with_state(state)
}
pub(super) async fn same_origin(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if request.method() != axum::http::Method::GET {
        if let Some(origin) = request.headers().get(header::ORIGIN) {
            let host = request
                .headers()
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if origin.to_str().ok() != Some(format!("http://{host}").as_str()) {
                return error(StatusCode::FORBIDDEN, "拒绝跨站操作").into_response();
            }
        }
        if request
            .headers()
            .get("sec-fetch-site")
            .is_some_and(|v| v == "cross-site")
        {
            return error(StatusCode::FORBIDDEN, "拒绝跨站操作").into_response();
        }
    }
    next.run(request).await
}
async fn list(State(app): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(json!({"items":app.scans().await.map_err(app_error)?})))
}
async fn detail(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ScanReport>, ApiError> {
    Ok(Json(app.scan(id).await.map_err(app_error)?))
}
async fn start(
    State(app): State<AppState>,
    body: Result<Json<ScanRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ScanReport>), ApiError> {
    Ok((
        StatusCode::ACCEPTED,
        Json(app.start(input(body)?).await.map_err(app_error)?),
    ))
}
async fn cancel(
    State(app): State<AppState>,
    Path(id): Path<String>,
    body: Result<Json<serde_json::Value>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    input(body)?;
    app.cancel(&id).await.map_err(app_error)?;
    Ok(Json(
        json!({"status":"cancellation_requested","message":"将在当前请求返回后停止，最长等待约 20 秒"}),
    ))
}
async fn plans(State(app): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    Ok(Json(json!({"items":app.plans().await.map_err(app_error)?})))
}
async fn plan(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PlanPreview>, ApiError> {
    Ok(Json(app.plan(id).await.map_err(app_error)?))
}
async fn create_plan(
    State(app): State<AppState>,
    body: Result<Json<PlanRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<PlanPreview>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(app.create_plan(input(body)?).await.map_err(app_error)?),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    fn app() -> AppState {
        Application::new(
            Provider::new().unwrap(),
            Store::open(std::path::Path::new(":memory:")).unwrap(),
        )
    }
    async fn request(router: Router, uri: &str, body: &str) -> Response {
        router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn invalid_input_and_live_execution_are_rejected() {
        let router = router(app());
        assert_eq!(
            request(router.clone(), "/api/scans", r#"{"max_items":0}"#)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            request(router.clone(), "/api/trades", "{}").await.status(),
            StatusCode::NOT_IMPLEMENTED
        );
        let malformed = request(router.clone(), "/api/scans", "{").await;
        assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(malformed.into_body(), 100_000)
            .await
            .unwrap();
        assert!(serde_json::from_slice::<serde_json::Value>(&body).unwrap()["error"].is_string());
        let cross_site = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/scans")
                    .header("origin", "https://untrusted.test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(cross_site.status(), StatusCode::FORBIDDEN);
    }
    #[tokio::test]
    async fn scan_to_plan_is_persisted_and_retrievable() {
        use crate::application::ports::AccountRepository;
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let app = Application::new(Provider::new().unwrap(), store.clone());
        let router = router(app.clone());
        let response = request(router.clone(), "/api/scans", "{}").await;
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = axum::body::to_bytes(response.into_body(), 100_000)
            .await
            .unwrap();
        let initial: ScanReport = serde_json::from_slice(&body).unwrap();
        let finished = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let report = app.scan(initial.id.clone()).await.unwrap();
                if report.status != "running" {
                    break report;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(finished.status, "completed");
        assert_eq!(finished.coverage.accepted, 7);
        let payload = json!({"scan_id":initial.id,"budget_cents":10000,"target_wallet_cents":null,
            "max_units_per_item":2,"max_positions":3,"max_quote_age_secs":300})
        .to_string();
        let response = request(router.clone(), "/api/plans", &payload).await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), 100_000)
            .await
            .unwrap();
        let plan: PlanPreview = serde_json::from_slice(&body).unwrap();
        assert!(!plan.lines.is_empty());
        assert!(plan.cash_total_cents <= 10000);
        assert_eq!(plan.status, "preview_only");
        assert_eq!(plan.algorithm, "ratio_greedy_v2");
        assert_eq!(
            app.plan(plan.id.clone()).await.unwrap().cash_total_cents,
            plan.cash_total_cents
        );
        assert_eq!(app.plans().await.unwrap().len(), 1);
        let mut constrained = serde_json::from_str::<serde_json::Value>(&payload).unwrap();
        constrained["position_snapshot"] = json!({"account_id":"fixture-account","observed_at":crate::domain::selection::now(),"reserved_cash_cents":200,
            "entries":finished.candidates.iter().filter(|c|c.passed).map(|c|json!({"key":c.item.key,"held_units":2,"pending_buy_units":0,"available_units":0,"daily_volume":null})).collect::<Vec<_>>()});
        let missing = request(router.clone(), "/api/plans", &constrained.to_string()).await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert_eq!(app.plans().await.unwrap().len(), 1);
        store
            .add_account(crate::domain::accounts::Account {
                id: "fixture-account".into(),
                label: "fixture".into(),
                steam_id: "76561198000000001".into(),
                created_at: crate::domain::selection::now(),
            })
            .await
            .unwrap();
        let saved = request(router.clone(), "/api/plans", &constrained.to_string()).await;
        assert_eq!(saved.status(), StatusCode::CREATED);
        let saved: PlanPreview = serde_json::from_slice(
            &axum::body::to_bytes(saved.into_body(), 100_000)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(saved.lines.is_empty());
        assert_eq!(
            (
                saved.cash_total_cents,
                saved.reserved_cash_cents,
                saved.remaining_budget_cents
            ),
            (0, 200, 9800)
        );
        assert_eq!(
            app.plan(saved.id).await.unwrap().position_source.as_deref(),
            Some("manual_unverified")
        );
        let response = router
            .oneshot(
                Request::builder()
                    .uri(format!("/api/plans/{}", plan.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
