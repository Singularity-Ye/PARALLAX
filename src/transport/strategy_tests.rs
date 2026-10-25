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

#[tokio::test]
async fn exit_valuation_http_persists_full_version_books_and_native_calculation_after_reopen() {
    use crate::domain::exit_valuation::tests::strategy_fixture;
    let path = std::env::temp_dir().join(format!("parallax-exit-{}.db", uuid::Uuid::new_v4()));
    let repo = Store::open(&path).unwrap();
    let (s, p, h, b, r) = strategy_fixture(now());
    repo.add_account(Account {
        id: h.account_id.clone(),
        label: "fixture".into(),
        steam_id: "76561198000000001".into(),
        created_at: now(),
    })
    .await
    .unwrap();
    repo.append_strategy(s, 0).await.unwrap();
    repo.insert_plan(p).await.unwrap();
    repo.insert_history(h).await.unwrap();
    repo.insert_order_book(b.clone()).await.unwrap();
    let app = router(StrategyResearch::new(repo.clone()));
    let (status, value) = call(app.clone(), "POST", "/api/strategy-evaluations", json!(r)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(value["algorithm"], "purchase_research_v2");
    assert_eq!(
        value["items"][0]["exit_valuation"]["stressed_net_cny"]["minor"],
        7759
    );
    assert_eq!(value["order_books"][0]["id"], b.id);
    assert_eq!(value["execution_allowed"], false);
    let id = value["id"].as_str().unwrap().to_string();
    assert!(
        repo.insert_evaluation(serde_json::from_value(value.clone()).unwrap())
            .await
            .is_err()
    );
    drop(app);
    drop(repo);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.evaluation(id).await.unwrap().unwrap()).unwrap(),
        value
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn exit_unknown_book_id_never_saves_a_research_record() {
    use crate::domain::exit_valuation::tests::strategy_fixture;
    let repo = Store::open(Path::new(":memory:")).unwrap();
    let (s, p, h, _, r) = strategy_fixture(now());
    repo.add_account(Account {
        id: h.account_id.clone(),
        label: "fixture".into(),
        steam_id: "76561198000000001".into(),
        created_at: now(),
    })
    .await
    .unwrap();
    repo.append_strategy(s, 0).await.unwrap();
    repo.insert_plan(p).await.unwrap();
    repo.insert_history(h).await.unwrap();
    let app = router(StrategyResearch::new(repo.clone()));
    assert_eq!(
        call(app, "POST", "/api/strategy-evaluations", json!(r))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert!(repo.evaluations().await.unwrap().is_empty());
}

#[tokio::test]
async fn holding_v3_http_persists_objective_and_evidence_scope_without_platform_access() {
    use crate::domain::strategy::ResearchObjective;
    let path = std::env::temp_dir().join(format!(
        "parallax-holding-objective-{}.db",
        uuid::Uuid::new_v4()
    ));
    let store = Store::open(&path).unwrap();
    let (mut s, p, h, mut r) = fixture(now());
    s.draft.schema_version = 3;
    s.draft.objective = ResearchObjective::Holding;
    s.draft.rules.remove(0);
    store
        .add_account(Account {
            id: h.account_id.clone(),
            label: "fixture".into(),
            steam_id: "76561198000000001".into(),
            created_at: now(),
        })
        .await
        .unwrap();
    store.insert_plan(p).await.unwrap();
    store.insert_history(h).await.unwrap();
    let app = router(StrategyResearch::new(store.clone()));
    let (status, v) = call(app.clone(), "POST", "/api/strategies", json!(s.draft)).await;
    assert_eq!(status, StatusCode::CREATED);
    r.strategy_id = v["strategy_id"].as_str().unwrap().into();
    let (status, e) = call(app.clone(), "POST", "/api/strategy-evaluations", json!(r)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(e["algorithm"], "purchase_research_v3");
    assert_eq!(e["items"][0]["review"]["objective"], "holding");
    assert_eq!(e["items"][0]["review"]["holding_value_established"], false);
    assert_eq!(e["execution_allowed"], false);
    let id = e["id"].as_str().unwrap().to_string();
    drop(app);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.evaluation(id).await.unwrap().unwrap()).unwrap(),
        e
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}
