use super::*;
use crate::{
    adapters::sqlite::Store,
    application::ports::*,
    domain::{accounts::*, preflight::tests::inspection_fixture, selection::now},
};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use tower::ServiceExt;
async fn call(
    app: Router,
    method: &str,
    path: &str,
    body: Value,
    origin: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(origin) = origin {
        req = req
            .header("origin", origin)
            .header("host", "127.0.0.1:38480");
    }
    let res = app
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(res.headers()["cache-control"], "no-store");
    let status = res.status();
    (
        status,
        serde_json::from_slice(&to_bytes(res.into_body(), 5_000_000).await.unwrap()).unwrap(),
    )
}
#[tokio::test]
async fn inspection_survives_restart_and_session_rotation_without_secret_or_overwrite() {
    let path = std::env::temp_dir().join(format!("parallax-preflight-{}.db", uuid::Uuid::new_v4()));
    let store = Store::open(&path).unwrap();
    let (req, e, c) = inspection_fixture(now());
    store.add_account(c.account.clone()).await.unwrap();
    store.insert_evaluation(e.clone()).await.unwrap();
    for view in &c.sessions {
        store
            .save_session(
                c.account.id.clone(),
                StoredSession {
                    view: view.clone(),
                    credential_key: "private-vault-locator".into(),
                },
            )
            .await
            .unwrap();
    }
    let app = router(Preflights::new(store.clone()));
    let (status, p) = call(app.clone(), "POST", "/api/preflights", json!(req), None).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(p["evidence_status"], "pass");
    assert_eq!(p["execution_allowed"], false);
    assert!(!p.to_string().contains("private-vault-locator"));
    let id = p["id"].as_str().unwrap().to_owned();
    assert!(
        store
            .insert_preflight(serde_json::from_value(p.clone()).unwrap())
            .await
            .is_err()
    );
    let mut view = c.sessions[1].clone();
    view.generation = "rotated".into();
    store
        .save_session(
            c.account.id.clone(),
            StoredSession {
                view,
                credential_key: "other-private-locator".into(),
            },
        )
        .await
        .unwrap();
    let (_, changed) = call(app.clone(), "POST", "/api/preflights", json!(req), None).await;
    assert_eq!(changed["status"], "fail");
    assert_eq!(
        call(app.clone(), "GET", "/api/preflights", Value::Null, None)
            .await
            .1["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    drop(app);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    let app = router(Preflights::new(reopened.clone()));
    assert_eq!(
        call(
            app,
            "GET",
            &format!("/api/preflights/{id}"),
            Value::Null,
            None
        )
        .await
        .1,
        p
    );
    assert_eq!(
        serde_json::to_value(reopened.evaluation(e.id.clone()).await.unwrap().unwrap()).unwrap(),
        json!(e)
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn invalid_binding_missing_records_and_cross_origin_do_not_persist() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let (mut req, e, c) = inspection_fixture(now());
    store.insert_evaluation(e).await.unwrap();
    store.add_account(c.account).await.unwrap();
    let app = router(Preflights::new(store.clone()));
    assert_eq!(
        call(
            app.clone(),
            "POST",
            "/api/preflights",
            json!(req),
            Some("https://external.example")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    req.selections[0].quantity = 3;
    assert_eq!(
        call(app.clone(), "POST", "/api/preflights", json!(req), None)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    req.evaluation_id = "missing".into();
    assert_eq!(
        call(app.clone(), "POST", "/api/preflights", json!(req), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let mut body = json!(req);
    body["authorization_token"] = json!("invented");
    assert_eq!(
        call(app, "POST", "/api/preflights", body, None).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(store.preflights().await.unwrap().is_empty());
}
