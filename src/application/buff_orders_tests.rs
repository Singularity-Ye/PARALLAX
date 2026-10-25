use super::*;
use crate::{
    adapters::sqlite::Store,
    application::account_tests::{TestVault, TestVerifier},
    domain::accounts::*,
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

#[tokio::test]
async fn buff_orders_do_not_clear_another_connections_challenge_between_pages() {
    let (store, auth, id) = fixture().await;
    struct ChallengeDuringRead {
        store: Store,
        calls: Arc<AtomicUsize>,
    }
    impl BuffOrderProvider for ChallengeDuringRead {
        async fn history_page(
            &self,
            page: u32,
            size: u32,
            _: &str,
            _: &Credentials,
        ) -> Result<OrderHistoryPage, BuffReadError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.store.set_buff_policy(now() + 120, true).await.unwrap();
            Ok(OrderHistoryPage {
                page_num: page,
                page_size: size,
                reported_total: Some(2),
                orders: vec![ObservedOrder {
                    order_id: "a".into(),
                    state_code: None,
                    pay_expire_timeout_secs: None,
                }],
            })
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let app = BuffOrderResearch::new(
        store.clone(),
        auth,
        ChallengeDuringRead {
            store: store.clone(),
            calls: calls.clone(),
        },
    );
    assert!(matches!(
        app.fetch(&id, request()).await,
        Err(AppError::Buff(BuffReadError::Challenge))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(store.buff_policy().await.unwrap().1);
    assert!(app.list(&id).await.unwrap().is_empty());
}

#[tokio::test]
async fn buff_orders_recheck_database_generation_before_commit() {
    let (store, auth, id) = fixture().await;
    struct ChangedDuringRead {
        store: Store,
        id: String,
    }
    impl BuffOrderProvider for ChangedDuringRead {
        async fn history_page(
            &self,
            page: u32,
            size: u32,
            _: &str,
            _: &Credentials,
        ) -> Result<OrderHistoryPage, BuffReadError> {
            let mut session = self
                .store
                .session(self.id.clone(), Platform::Buff)
                .await
                .unwrap()
                .unwrap();
            session.view.generation = "replacement-from-another-process".into();
            self.store
                .save_session(self.id.clone(), session)
                .await
                .unwrap();
            Ok(OrderHistoryPage {
                page_num: page,
                page_size: size,
                reported_total: Some(0),
                orders: vec![],
            })
        }
    }
    let app = BuffOrderResearch::new(
        store.clone(),
        auth,
        ChangedDuringRead {
            store,
            id: id.clone(),
        },
    );
    assert!(matches!(
        app.fetch(&id, request()).await,
        Err(AppError::Conflict(_))
    ));
    assert!(app.list(&id).await.unwrap().is_empty());
}

struct Source {
    calls: Arc<AtomicUsize>,
    fail_page: Option<(u32, BuffReadError)>,
    duplicate: bool,
}
impl BuffOrderProvider for Source {
    async fn history_page(
        &self,
        page: u32,
        size: u32,
        principal: &str,
        secret: &Credentials,
    ) -> Result<OrderHistoryPage, BuffReadError> {
        assert_eq!(principal, "synthetic-buff-user");
        assert_eq!(secret.cookies, "session=synthetic-order-secret");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some((number, e)) = self.fail_page
            && number == page
        {
            return Err(e);
        }
        Ok(OrderHistoryPage {
            page_num: page,
            page_size: size,
            reported_total: Some(2),
            orders: vec![ObservedOrder {
                order_id: if self.duplicate {
                    "order-1".into()
                } else {
                    format!("order-{page}")
                },
                state_code: Some(if page == 1 {
                    OrderStateCode::Text("PAYING".into())
                } else {
                    OrderStateCode::Number(3)
                }),
                pay_expire_timeout_secs: Some(60),
            }],
        })
    }
}
type Auth = Accounts<Store, TestVault, TestVerifier>;
async fn fixture() -> (Store, Arc<Auth>, String) {
    let store = Store::open(Path::new(":memory:")).unwrap();
    let auth = Accounts::new(
        store.clone(),
        TestVault::default(),
        TestVerifier {
            status: SessionStatus::Valid,
            calls: Arc::new(AtomicUsize::new(0)),
        },
    );
    let a = auth
        .create("synthetic".into(), "76561198000000001".into())
        .await
        .unwrap();
    auth.import(
        &a.id,
        Platform::Buff,
        Credentials {
            cookies: "session=synthetic-order-secret".into(),
            user_agent: "private-agent".into(),
        },
    )
    .await
    .unwrap();
    let mut s = store
        .session(a.id.clone(), Platform::Buff)
        .await
        .unwrap()
        .unwrap();
    s.view.status = SessionStatus::Valid;
    s.view.checked_at = Some(now());
    s.view.subject = Some("synthetic-buff-user".into());
    store.save_session(a.id.clone(), s).await.unwrap();
    (store, auth, a.id)
}
fn source(error: Option<(u32, BuffReadError)>, duplicate: bool) -> Source {
    Source {
        calls: Arc::new(AtomicUsize::new(0)),
        fail_page: error,
        duplicate,
    }
}
fn request() -> OrderHistoryRequest {
    OrderHistoryRequest {
        max_pages: 2,
        page_size: 1,
    }
}

#[tokio::test]
async fn buff_orders_pages_persist_as_immutable_bound_observation_and_exact_lookup() {
    let (store, auth, id) = fixture().await;
    let app = BuffOrderResearch::new(store.clone(), auth.clone(), source(None, false));
    let s = app.fetch(&id, request()).await.unwrap();
    assert_eq!(s.pages.len(), 2);
    assert_eq!(s.stop_reason, HistoryStop::ReportedEnd);
    assert!(!s.complete && !s.execution_allowed && !s.protocol_live_verified);
    assert!(store.insert_buff_orders(s.clone()).await.is_err());
    assert_eq!(
        app.lookup(
            &id,
            s.id.clone(),
            OrderLookupRequest {
                order_id: "order-2".into()
            }
        )
        .await
        .unwrap()
        .payment_evidence,
        PaymentEvidence::Unknown
    );
    assert!(
        !app.lookup(
            &id,
            s.id.clone(),
            OrderLookupRequest {
                order_id: "absent".into()
            }
        )
        .await
        .unwrap()
        .found
    );
    let raw = serde_json::to_string(&s).unwrap();
    assert!(
        !raw.contains("synthetic-order-secret")
            && !raw.contains("private-agent")
            && !raw.contains("credential_key")
    );
    let other = auth
        .create("other".into(), "76561198000000002".into())
        .await
        .unwrap();
    assert!(matches!(
        app.get(&other.id, s.id.clone()).await,
        Err(AppError::Missing(_))
    ));
    auth.revoke(&id, Platform::Buff).await.unwrap();
    assert_eq!(
        serde_json::to_string(&app.get(&id, s.id).await.unwrap()).unwrap(),
        raw
    );
}
#[tokio::test]
async fn buff_orders_later_page_failure_does_not_save_partial_result_and_challenge_blocks_retries()
{
    let (store, auth, id) = fixture().await;
    let app = BuffOrderResearch::new(
        store.clone(),
        auth.clone(),
        source(Some((2, BuffReadError::Challenge)), false),
    );
    assert!(matches!(
        app.fetch(&id, request()).await,
        Err(AppError::Buff(BuffReadError::Challenge))
    ));
    assert!(app.list(&id).await.unwrap().is_empty());
    assert!(store.buff_policy().await.unwrap().1);
    let attempts = app.provider.calls.load(Ordering::SeqCst);
    assert!(app.fetch(&id, request()).await.is_err());
    assert_eq!(app.provider.calls.load(Ordering::SeqCst), attempts);
    assert_eq!(
        store
            .session(id, Platform::Buff)
            .await
            .unwrap()
            .unwrap()
            .view
            .status,
        SessionStatus::VerificationRequired
    );
}
#[tokio::test]
async fn buff_orders_invalid_identity_stale_session_and_cooldown_do_not_call_provider() {
    let (store, auth, id) = fixture().await;
    let app = BuffOrderResearch::new(store.clone(), auth, source(None, false));
    let mut s = store
        .session(id.clone(), Platform::Buff)
        .await
        .unwrap()
        .unwrap();
    s.view.checked_at = Some(now() - 301);
    store.save_session(id.clone(), s.clone()).await.unwrap();
    assert!(app.fetch(&id, request()).await.is_err());
    s.view.checked_at = Some(now());
    store.save_session(id.clone(), s).await.unwrap();
    store.set_buff_policy(now() + 100, false).await.unwrap();
    assert!(app.fetch(&id, request()).await.is_err());
    assert_eq!(app.provider.calls.load(Ordering::SeqCst), 0);
    assert!(app.list(&id).await.unwrap().is_empty());
}
#[tokio::test]
async fn buff_orders_rotated_credentials_invalidate_generation_and_do_not_save() {
    let (store, auth, id) = fixture().await;
    let generation = store
        .session(id.clone(), Platform::Buff)
        .await
        .unwrap()
        .unwrap()
        .view
        .generation;
    let app = BuffOrderResearch::new(
        store.clone(),
        auth,
        source(Some((1, BuffReadError::CredentialsChanged)), false),
    );
    assert!(matches!(
        app.fetch(&id, request()).await,
        Err(AppError::Buff(BuffReadError::CredentialsChanged))
    ));
    let s = store
        .session(id.clone(), Platform::Buff)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(s.view.generation, generation);
    assert_eq!(s.view.status, SessionStatus::Unverified);
    assert!(app.list(&id).await.unwrap().is_empty());
}
#[tokio::test]
async fn buff_orders_page_drift_does_not_replace_previous_observation() {
    let (store, auth, id) = fixture().await;
    let first = BuffOrderResearch::new(store.clone(), auth.clone(), source(None, false))
        .fetch(&id, request())
        .await
        .unwrap();
    store.set_buff_policy(0, false).await.unwrap();
    let app = BuffOrderResearch::new(store, auth, source(None, true));
    assert!(app.fetch(&id, request()).await.is_err());
    let list = app.list(&id).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, first.id);
}
#[tokio::test]
async fn buff_orders_http_origin_shape_scope_and_missing_are_distinct() {
    let (store, auth, id) = fixture().await;
    let app = BuffOrderResearch::new(store, auth, source(None, false));
    let router = crate::transport::buff_orders::router(app.clone());
    async fn call(
        router: axum::Router,
        path: &str,
        v: Value,
        origin: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut r = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json");
        if let Some(o) = origin {
            r = r.header("host", "127.0.0.1:38480").header("origin", o);
        }
        let response = router
            .oneshot(r.body(Body::from(v.to_string())).unwrap())
            .await
            .unwrap();
        assert_eq!(response.headers()["cache-control"], "no-store");
        let status = response.status();
        (
            status,
            serde_json::from_slice(&to_bytes(response.into_body(), 100_000).await.unwrap())
                .unwrap(),
        )
    }
    let path = format!("/api/accounts/{id}/buff-order-observations");
    assert_eq!(
        call(
            router.clone(),
            &path,
            json!({"max_pages":1,"page_size":20}),
            Some("https://outside.example")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            router.clone(),
            &path,
            json!({"max_pages":1,"page_size":20,"paid":true}),
            None
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        call(
            router.clone(),
            &path,
            json!({"max_pages":0,"page_size":20}),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (status, result) = call(
        router.clone(),
        &path,
        json!({"max_pages":1,"page_size":1}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(result["stop_reason"], "page_budget");
    let lookup = format!("{path}/{}/lookup", result["id"].as_str().unwrap());
    let (status, result) = call(router.clone(), &lookup, json!({"order_id":"absent"}), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["found"], false);
    assert_eq!(result["payment_evidence"], "unknown");
    let other = lookup.replace(&id, "other-account");
    assert_eq!(
        call(router, &other, json!({"order_id":"absent"}), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}
