use super::*;
use crate::domain::funding;
use crate::{
    adapters::sqlite::Store,
    application::ports::*,
    domain::{
        accounts::*,
        preflight::{self, tests::inspection_fixture},
    },
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
fn saved<T>(r: CheckoutOutcome<T>) -> T {
    match r {
        CheckoutOutcome::Saved(v) => v.item,
        CheckoutOutcome::Conflict(e) => panic!("conflict: {e}"),
        CheckoutOutcome::Invalid(e) => panic!("invalid: {e}"),
        CheckoutOutcome::Missing(e) => panic!("missing: {e}"),
    }
}
async fn seed(store: &Store, at: u64, id: &str) -> (preflight::Preflight, FundingSnapshot) {
    let (mut r, mut e, mut c) = inspection_fixture(at);
    c.account.id = id.into();
    r.account_id = Some(id.into());
    e.request.account_id = Some(id.into());
    e.plan.request.buff_account_id = Some(id.into());
    for b in &mut e.plan.buff_valuations {
        b.account_id = id.into();
    }
    for h in &mut e.histories {
        h.account_id = id.into();
    }
    store.add_account(c.account.clone()).await.unwrap();
    for view in &c.sessions {
        store
            .save_session(
                id.into(),
                StoredSession {
                    view: view.clone(),
                    credential_key: "private-locator-never-read".into(),
                },
            )
            .await
            .unwrap();
    }
    let p = preflight::inspect(r, e, Some(c), at).unwrap();
    store.insert_preflight(p.clone()).await.unwrap();
    let funding = if id == "a" {
        saved(
            store
                .observe_funding(
                    id.into(),
                    FundingRequest {
                        client_key: "fund".into(),
                        available_cash_cents: 1000,
                        observed_at: at - 1,
                        evidence_ref: "fund-evidence".into(),
                    },
                    at,
                )
                .await
                .unwrap(),
        )
    } else {
        saved(store.funding_view(id.into(), at).await.unwrap())
            .snapshot
            .unwrap()
    };
    (p, funding)
}
fn prepare_request(p: &preflight::Preflight, f: &FundingSnapshot) -> PrepareRequest {
    PrepareRequest {
        client_key: format!("intent-{}", p.id),
        preflight_id: p.id.clone(),
        funding_snapshot_id: f.id.clone(),
        max_funding_age_secs: 90,
    }
}
async fn call(
    app: Router,
    method: &str,
    path: &str,
    value: Value,
    origin: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(origin) = origin {
        req = req
            .header("host", "127.0.0.1:38480")
            .header("origin", origin);
    }
    let r = app
        .oneshot(req.body(Body::from(value.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(r.headers()["cache-control"], "no-store");
    let status = r.status();
    (
        status,
        serde_json::from_slice(&to_bytes(r.into_body(), 6_000_000).await.unwrap()).unwrap(),
    )
}
#[tokio::test]
async fn funding_zero_idempotency_and_observation_order_preserve_unknown_and_known_zero() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let at = 1_792_800_000;
    let (_, f) = seed(&store, at, "a").await;
    assert!(matches!(
        store
            .observe_funding("a".into(), f.request.clone(), at)
            .await
            .unwrap(),
        CheckoutOutcome::Saved(CheckoutWrite { replayed: true, .. })
    ));
    let mut r = f.request;
    r.available_cash_cents = 0;
    assert!(matches!(
        store
            .observe_funding("a".into(), r.clone(), at)
            .await
            .unwrap(),
        CheckoutOutcome::Conflict(_)
    ));
    r.client_key = "zero".into();
    r.evidence_ref = "zero-proof".into();
    r.observed_at = at;
    saved(store.observe_funding("a".into(), r, at).await.unwrap());
    let v = saved(store.funding_view("a".into(), at).await.unwrap());
    assert_eq!(v.available_after_reservations_cents, Some(0));
    assert!(v.fresh);
    assert!(!v.execution_allowed);
    let v = funding::view("unknown".into(), None, 0, at);
    assert_eq!(v.available_after_reservations_cents, None);
    assert!(!v.fresh);
}
#[tokio::test]
async fn shared_buff_subject_is_single_flight_across_accounts_and_connections() {
    let path = std::env::temp_dir().join(format!(
        "parallax-checkout-race-{}.db",
        uuid::Uuid::new_v4()
    ));
    let a = Store::open(&path).unwrap();
    let b = Store::open(&path).unwrap();
    let at = 1_792_800_000;
    let (p, f) = seed(&a, at, "a").await;
    let (other, _) = seed(&a, at, "b").await;
    let (left, right) = tokio::join!(
        a.prepare_intent("a".into(), prepare_request(&p, &f), at),
        b.prepare_intent("b".into(), prepare_request(&other, &f), at)
    );
    assert_ne!(
        matches!(left.unwrap(), CheckoutOutcome::Saved(_)),
        matches!(right.unwrap(), CheckoutOutcome::Saved(_))
    );
    let fa = saved(a.funding_view("a".into(), at).await.unwrap());
    let fb = saved(b.funding_view("b".into(), at).await.unwrap());
    assert_eq!(fa.reserved_cash_cents, 600);
    assert_eq!(fb.reserved_cash_cents, 600);
    assert_eq!(fa.available_after_reservations_cents, Some(400));
    drop(a);
    drop(b);
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn lower_balance_keeps_reservation_and_cancellation_releases_exactly_once() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let at = 1_792_800_000;
    let (p, f) = seed(&store, at, "a").await;
    let v = saved(
        store
            .prepare_intent("a".into(), prepare_request(&p, &f), at)
            .await
            .unwrap(),
    );
    saved(
        store
            .observe_funding(
                "a".into(),
                FundingRequest {
                    client_key: "lower".into(),
                    available_cash_cents: 100,
                    observed_at: at + 1,
                    evidence_ref: "lower".into(),
                },
                at + 1,
            )
            .await
            .unwrap(),
    );
    let view = saved(store.funding_view("a".into(), at + 1).await.unwrap());
    assert_eq!(view.reserved_cash_cents, 600);
    assert_eq!(view.deficit_cents, Some(500));
    assert_eq!(view.available_after_reservations_cents, Some(0));
    let cancel = CancelRequest {
        client_key: "cancel".into(),
        expected_revision: 1,
        reason: "只取消本地准备".into(),
    };
    let next = saved(
        store
            .cancel_intent("a".into(), v.id.clone(), cancel.clone(), at + 2)
            .await
            .unwrap(),
    );
    assert_eq!(next.state, IntentState::Cancelled);
    assert!(matches!(
        store
            .cancel_intent("a".into(), v.id.clone(), cancel, at + 3)
            .await
            .unwrap(),
        CheckoutOutcome::Saved(CheckoutWrite { replayed: true, .. })
    ));
    assert_eq!(
        saved(store.funding_view("a".into(), at + 3).await.unwrap()).reserved_cash_cents,
        0
    );
    assert_eq!(
        store.intent_events("a".into(), v.id).await.unwrap().len(),
        1
    );
}
#[tokio::test]
async fn restart_expiry_and_credential_revocation_never_silently_release_or_repeat_intent() {
    let path = std::env::temp_dir().join(format!(
        "parallax-checkout-reopen-{}.db",
        uuid::Uuid::new_v4()
    ));
    let store = Store::open(&path).unwrap();
    let at = 1_792_800_000;
    let (p, f) = seed(&store, at, "a").await;
    let req = prepare_request(&p, &f);
    let old = saved(
        store
            .prepare_intent("a".into(), req.clone(), at)
            .await
            .unwrap(),
    );
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        json!(
            store
                .purchase_intent("a".into(), old.id.clone())
                .await
                .unwrap()
                .unwrap()
        ),
        json!(old)
    );
    let mut session = store
        .session("a".into(), Platform::Buff)
        .await
        .unwrap()
        .unwrap();
    session.view.status = SessionStatus::Missing;
    session.view.subject = None;
    store.save_session("a".into(), session).await.unwrap();
    let replay = saved(
        store
            .prepare_intent("a".into(), req.clone(), at + 1000)
            .await
            .unwrap(),
    );
    assert_eq!(replay.id, old.id);
    assert_eq!(replay.state, IntentState::Reserved);
    assert!(matches!(
        store
            .observe_funding("a".into(), f.request, at + 1000)
            .await
            .unwrap(),
        CheckoutOutcome::Saved(CheckoutWrite { replayed: true, .. })
    ));
    let c = CancelRequest {
        client_key: "after-revocation".into(),
        expected_revision: 1,
        reason: "取消尚未提交的本地意图".into(),
    };
    saved(
        store
            .cancel_intent("a".into(), old.id, c, at + 1000)
            .await
            .unwrap(),
    );
    let mut different = req;
    different.client_key = "new-key".into();
    assert!(!matches!(
        store
            .prepare_intent("a".into(), different, at + 1000)
            .await
            .unwrap(),
        CheckoutOutcome::Saved(_)
    ));
    drop(store);
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn obsolete_funding_stale_quotes_and_rotation_never_leave_partial_reservations() {
    let at = 1_792_800_000;
    for case in 0..4 {
        let store = Store::open(std::path::Path::new(":memory:")).unwrap();
        let (p, f) = seed(&store, at, "a").await;
        let mut req = prepare_request(&p, &f);
        let mut check_at = at;
        match case {
            0 => {
                saved(
                    store
                        .observe_funding(
                            "a".into(),
                            FundingRequest {
                                client_key: "new".into(),
                                available_cash_cents: 100,
                                observed_at: at,
                                evidence_ref: "new".into(),
                            },
                            at,
                        )
                        .await
                        .unwrap(),
                );
            }
            1 => check_at = at + 91,
            2 => {
                let mut s = store
                    .session("a".into(), Platform::Buff)
                    .await
                    .unwrap()
                    .unwrap();
                s.view.generation = "rotated".into();
                store.save_session("a".into(), s).await.unwrap();
            }
            _ => {
                req.max_funding_age_secs = 1;
                check_at = at + 2;
            }
        }
        assert!(!matches!(
            store
                .prepare_intent("a".into(), req, check_at)
                .await
                .unwrap(),
            CheckoutOutcome::Saved(_)
        ));
        assert!(store.purchase_intents("a".into()).await.unwrap().is_empty());
        assert_eq!(
            saved(store.funding_view("a".into(), check_at).await.unwrap()).reserved_cash_cents,
            0
        );
    }
}
#[tokio::test]
async fn cancelled_preflight_is_not_reused_and_wrong_cancellation_version_has_no_effect() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let at = 1_792_800_000;
    let (p, f) = seed(&store, at, "a").await;
    let req = prepare_request(&p, &f);
    let v = saved(
        store
            .prepare_intent("a".into(), req.clone(), at)
            .await
            .unwrap(),
    );
    let mut c = CancelRequest {
        client_key: "c".into(),
        expected_revision: 2,
        reason: "cancel".into(),
    };
    assert!(matches!(
        store
            .cancel_intent("a".into(), v.id.clone(), c.clone(), at)
            .await
            .unwrap(),
        CheckoutOutcome::Conflict(_)
    ));
    assert!(
        store
            .intent_events("a".into(), v.id.clone())
            .await
            .unwrap()
            .is_empty()
    );
    c.expected_revision = 1;
    saved(
        store
            .cancel_intent("a".into(), v.id.clone(), c, at)
            .await
            .unwrap(),
    );
    let mut second = req.clone();
    second.client_key = "other-key".into();
    assert!(matches!(
        store.prepare_intent("a".into(), second, at).await.unwrap(),
        CheckoutOutcome::Conflict(_)
    ));
    let replay = saved(store.prepare_intent("a".into(), req, at).await.unwrap());
    assert_eq!(replay.state, IntentState::Cancelled);
    assert_eq!(replay.revision, 2);
}
#[tokio::test]
async fn http_is_account_bound_and_does_not_accept_execution_flags_or_cross_site_writes() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let at = crate::domain::selection::now();
    let (p, f) = seed(&store, at, "a").await;
    let app = router(CheckoutPreparation::new(store.clone()));
    let body = json!(prepare_request(&p, &f));
    assert_eq!(
        call(
            app.clone(),
            "POST",
            "/api/accounts/a/purchase-intents",
            body.clone(),
            Some("https://external.example")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut invalid = body.clone();
    invalid["execution_allowed"] = json!(true);
    assert_eq!(
        call(
            app.clone(),
            "POST",
            "/api/accounts/a/purchase-intents",
            invalid,
            None
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (status, out) = call(
        app.clone(),
        "POST",
        "/api/accounts/a/purchase-intents",
        body,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(out["item"]["execution_allowed"], false);
    assert!(!out.to_string().contains("private-locator-never-read"));
    let id = out["item"]["id"].as_str().unwrap();
    assert_eq!(
        call(
            app.clone(),
            "GET",
            &format!("/api/accounts/other/purchase-intents/{id}"),
            Value::Null,
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            app,
            "POST",
            &format!("/api/accounts/other/purchase-intents/{id}/cancel"),
            json!({"client_key":"x","expected_revision":1,"reason":"other"}),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}
