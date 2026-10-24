use super::*;
use crate::{
    adapters::sqlite::Store,
    domain::{accounts::Account, selection::now, strategy::tests::fixture},
};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use std::path::Path;
use tower::ServiceExt;
async fn call(app: Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 5_000_000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn http_version_conflict_and_immutable_research_roundtrip_after_reopen() {
    let path = std::env::temp_dir().join(format!("parallax-strategy-{}.db", uuid::Uuid::new_v4()));
    let store = Store::open(&path).unwrap();
    let (s, p, h, mut req) = fixture(now());
    store
        .add_account(Account {
            id: h.account_id.clone(),
            label: "fixture".into(),
            steam_id: "76561198000000001".into(),
            created_at: now(),
        })
        .await
        .unwrap();
    store.insert_plan(p.clone()).await.unwrap();
    store.insert_history(h.clone()).await.unwrap();
    let app = router(StrategyResearch::new(store.clone()));
    let (status, v) = call(app.clone(), "POST", "/api/strategies", json!(s.draft)).await;
    assert_eq!(status, StatusCode::CREATED);
    let saved: StrategyVersion = serde_json::from_value(v).unwrap();
    req.strategy_id = saved.strategy_id.clone();
    let (status, e) = call(app.clone(), "POST", "/api/strategy-evaluations", json!(req)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(e["status"], "pass");
    assert_eq!(e["execution_allowed"], false);
    let id = e["id"].as_str().unwrap().to_owned();
    let url = format!("/api/strategies/{}/versions", saved.strategy_id);
    let mut changed = saved.draft.clone();
    changed.name = "新版本".into();
    assert_eq!(
        call(
            app.clone(),
            "POST",
            &url,
            json!({"expected_revision":1,"draft":changed})
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        call(
            app.clone(),
            "POST",
            &url,
            json!({"expected_revision":1,"draft":changed})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        call(app.clone(), "GET", &url, Value::Null).await.1["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let (status, old) = call(
        app.clone(),
        "GET",
        &format!("/api/strategies/{}/versions/1", saved.strategy_id),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(old["draft"]["name"], s.draft.name);
    assert_eq!(
        call(app.clone(), "GET", "/api/strategies", Value::Null)
            .await
            .1["items"][0]["revision"],
        2
    );
    assert_eq!(
        call(app.clone(), "GET", "/api/strategy-evaluations", Value::Null)
            .await
            .1["items"][0]["id"],
        id
    );
    assert!(
        store
            .insert_evaluation(serde_json::from_value(e.clone()).unwrap())
            .await
            .is_err()
    );
    drop(app);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    let stored = reopened.evaluation(id).await.unwrap().unwrap();
    assert_eq!(serde_json::to_value(stored).unwrap(), e);
    assert_eq!(
        serde_json::to_value(reopened.get_plan(p.id.clone()).await.unwrap().unwrap()).unwrap(),
        json!(p)
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn concurrent_compare_and_append_works_across_store_connections() {
    let path =
        std::env::temp_dir().join(format!("parallax-strategy-cas-{}.db", uuid::Uuid::new_v4()));
    let a = Store::open(&path).unwrap();
    let b = Store::open(&path).unwrap();
    let (s, _, _, _) = fixture(now());
    assert!(a.append_strategy(s.clone(), 0).await.unwrap());
    let mut s = s;
    s.revision = 2;
    let (left, right) = tokio::join!(
        a.append_strategy(s.clone(), 1),
        b.append_strategy(s.clone(), 1)
    );
    assert_ne!(left.unwrap(), right.unwrap());
    assert_eq!(a.strategy_versions(s.strategy_id).await.unwrap().len(), 2);
    drop(a);
    drop(b);
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn wrong_account_invalid_binding_and_missing_inputs_do_not_save_results() {
    let store = Store::open(Path::new(":memory:")).unwrap();
    let (s, p, h, mut req) = fixture(now());
    store.append_strategy(s, 0).await.unwrap();
    store.insert_plan(p).await.unwrap();
    store.insert_history(h.clone()).await.unwrap();
    store
        .add_account(Account {
            id: "other".into(),
            label: "fixture".into(),
            steam_id: "76561198000000002".into(),
            created_at: now(),
        })
        .await
        .unwrap();
    let app = router(StrategyResearch::new(store.clone()));
    req.account_id = Some("other".into());
    assert_eq!(
        call(app.clone(), "POST", "/api/strategy-evaluations", json!(req))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    req.account_id = None;
    assert_eq!(
        call(app.clone(), "POST", "/api/strategy-evaluations", json!(req))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    req.history_snapshot_ids.clear();
    req.plan_id = "missing".into();
    assert_eq!(
        call(app, "POST", "/api/strategy-evaluations", json!(req))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert!(store.evaluations().await.unwrap().is_empty());
}
#[tokio::test]
async fn transport_rejects_cross_site_oversize_and_unrecognized_payloads() {
    let app = router(StrategyResearch::new(
        Store::open(Path::new(":memory:")).unwrap(),
    ));
    let (s, _, _, _) = fixture(now());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/strategies")
                .header("host", "127.0.0.1:38480")
                .header("origin", "https://example.com")
                .header("content-type", "application/json")
                .body(Body::from(json!(s.draft).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let mut oversized = s.draft.clone();
    oversized.description = "x".repeat(17000);
    assert_eq!(
        call(app.clone(), "POST", "/api/strategies", json!(oversized))
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let mut payload = json!(s.draft);
    payload["secret"] = json!("not-accepted");
    assert_eq!(
        call(app.clone(), "POST", "/api/strategies", payload)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        call(app, "GET", "/api/strategy-evaluations/missing", Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}
