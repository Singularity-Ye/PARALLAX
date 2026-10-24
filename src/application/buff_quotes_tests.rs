use super::*;
use crate::{
    adapters::{market::Provider, sqlite::Store},
    application::{
        Application,
        account_tests::{TestVault, TestVerifier},
    },
    domain::{
        accounts::*,
        analysis::evaluate,
        money::Currency,
        orderbook::{OrderBook, OrderBookRequest, OrderBookSnapshot, PriceLevel},
        planning::PlanRequest,
        selection::{MarketItem, ScanReport, ScanRequest},
    },
};
use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Source {
    calls: Arc<AtomicUsize>,
    error: Option<BuffReadError>,
    empty: bool,
}
impl BuffQuoteProvider for Source {
    async fn sell_page(
        &self,
        goods_id: u64,
        secret: Credentials,
    ) -> Result<BuffSellPage, BuffReadError> {
        assert_eq!(secret.cookies, "session=synthetic-secret");
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(BuffSellPage {
            goods_id,
            market_hash_name: "fixture (Field-Tested)".into(),
            levels: if self.empty {
                vec![]
            } else {
                vec![
                    PriceLevel {
                        price_minor: 500,
                        quantity: 1,
                    },
                    PriceLevel {
                        price_minor: 600,
                        quantity: 5,
                    },
                ]
            },
            reported_total: Some(100),
        })
    }
}
type Auth = Accounts<Store, TestVault, TestVerifier>;
async fn fixture() -> (Store, Arc<Auth>, String, ScanReport) {
    let repo = Store::open(Path::new(":memory:")).unwrap();
    let auth = Accounts::new(
        repo.clone(),
        TestVault::default(),
        TestVerifier {
            status: SessionStatus::Valid,
            calls: Arc::new(AtomicUsize::new(0)),
        },
    );
    let account = auth
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    auth.import(
        &account.id,
        Platform::Buff,
        Credentials {
            cookies: "session=synthetic-secret".into(),
            user_agent: "synthetic-UA".into(),
        },
    )
    .await
    .unwrap();
    let mut session = repo
        .session(account.id.clone(), Platform::Buff)
        .await
        .unwrap()
        .unwrap();
    session.view.status = SessionStatus::Valid;
    session.view.subject = Some("synthetic-buff-user".into());
    session.view.checked_at = Some(now());
    repo.save_session(account.id.clone(), session)
        .await
        .unwrap();
    let mut scan = ScanReport::new(ScanRequest {
        source: SourceKind::Steamdt,
        ..Default::default()
    });
    scan.status = "completed".into();
    scan.finished_at = Some(now());
    scan.candidates = vec![evaluate(
        MarketItem {
            key: "fixture (Field-Tested)".into(),
            name: "fixture".into(),
            goods_id: Some(123),
            cash_cents: 800,
            steam_gross_cents: 1150,
            volume: 300,
            observed_at: now(),
        },
        &scan.request,
    )];
    repo.save(scan.clone()).await.unwrap();
    (repo, auth, account.id, scan)
}
fn request(scan: &ScanReport) -> BuffQuoteRequest {
    BuffQuoteRequest {
        scan_id: scan.id.clone(),
        item_key: scan.candidates[0].item.key.clone(),
    }
}
fn plan_request(scan: &ScanReport, account: &str, quote: &BuffQuoteSnapshot) -> PlanRequest {
    serde_json::from_value(serde_json::json!({"scan_id":scan.id,"budget_cents":5000,"target_wallet_cents":null,"max_units_per_item":10,"max_positions":5,"max_quote_age_secs":300,
        "buff_account_id":account,"buff_snapshot_ids":[quote.id]})).unwrap()
}
#[tokio::test]
async fn bound_quote_persists_is_redacted_and_caps_only_the_visible_lowest_price() {
    let (repo, auth, id, scan) = fixture().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let app = BuffResearch::new(
        repo.clone(),
        auth.clone(),
        Source {
            calls: calls.clone(),
            error: None,
            empty: false,
        },
    );
    let quote = app.fetch(&id, request(&scan)).await.unwrap();
    assert_eq!(quote.lowest_cash_cents, Some(500));
    assert_eq!(quote.lowest_tier_visible_units, 1);
    assert!(repo.insert_buff_quote(quote.clone()).await.is_err());
    assert!(app.fetch(&id, request(&scan)).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let raw = serde_json::to_string(&app.get(&id, quote.id.clone()).await.unwrap()).unwrap();
    assert!(
        !raw.contains("synthetic-secret")
            && !raw.contains("synthetic-UA")
            && !raw.contains("credential_key")
    );
    let application = Application::new(Provider::new().unwrap(), repo.clone());
    let saved = application
        .create_plan(plan_request(&scan, &id, &quote))
        .await
        .unwrap();
    assert_eq!(saved.lines[0].candidate.item.cash_cents, 500);
    assert_eq!(saved.lines[0].quantity, 1);
    assert_eq!(
        application
            .plan(saved.id)
            .await
            .unwrap()
            .buff_valuations
            .len(),
        1
    );
    assert_eq!(
        repo.get(scan.id.clone()).await.unwrap().unwrap().candidates[0]
            .item
            .cash_cents,
        800
    );
    let other = auth
        .create("other".into(), "76561198000000002".into())
        .await
        .unwrap();
    assert!(matches!(
        app.get(&other.id, quote.id.clone()).await,
        Err(AppError::Missing(_))
    ));
    auth.revoke(&id, Platform::Buff).await.unwrap();
    assert!(
        application
            .create_plan(plan_request(&scan, &id, &quote))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn empty_page_never_falls_back_and_both_legs_need_explicit_refresh() {
    let (repo, auth, id, mut scan) = fixture().await;
    let app = BuffResearch::new(
        repo.clone(),
        auth,
        Source {
            calls: Arc::new(AtomicUsize::new(0)),
            error: None,
            empty: true,
        },
    );
    let quote = app.fetch(&id, request(&scan)).await.unwrap();
    let application = Application::new(Provider::new().unwrap(), repo.clone());
    let empty = application
        .create_plan(plan_request(&scan, &id, &quote))
        .await
        .unwrap();
    assert!(empty.lines.is_empty());
    assert!(empty.skipped[0].reason.contains("未退回榜单"));
    let mut fresh = quote;
    fresh.page.levels = vec![PriceLevel {
        price_minor: 500,
        quantity: 1,
    }];
    // Pure planning derives prices from evidence, not cached display summaries.
    scan.candidates[0].item.observed_at = now() - 1000;
    let request = plan_request(&scan, &id, &fresh);
    let stale = crate::domain::planning::preview_with_quotes(
        &scan,
        request.clone(),
        now(),
        &[],
        std::slice::from_ref(&fresh),
    )
    .unwrap();
    assert!(
        stale.lines.is_empty(),
        "BUFF refresh cannot freshen the old Steam leg"
    );
    let steam = OrderBookSnapshot::new(
        OrderBookRequest {
            currency: crate::domain::money::Currency::cny(),
            app_id: 730,
            market_hash_name: scan.candidates[0].item.key.clone(),
        },
        OrderBook {
            currency: Currency::cny(),
            asks: vec![PriceLevel {
                price_minor: 2300,
                quantity: 1,
            }],
            bids: vec![],
            reported_buy_orders: None,
            reported_sell_orders: None,
        },
        now(),
    )
    .unwrap();
    let mut request = request;
    request.steam_snapshot_ids = vec![steam.id.clone()];
    let plan =
        crate::domain::planning::preview_with_quotes(&scan, request, now(), &[steam], &[fresh])
            .unwrap();
    assert_eq!(plan.lines[0].quantity, 1);
    assert_eq!(plan.lines[0].candidate.estimated_wallet_cents, 2000);
}
#[tokio::test]
async fn challenge_cooldown_expiry_and_rotation_are_persisted_without_fake_quotes() {
    for error in [
        BuffReadError::Challenge,
        BuffReadError::RateLimited(1),
        BuffReadError::Expired,
        BuffReadError::CredentialsChanged,
        BuffReadError::Network,
    ] {
        let (repo, auth, id, scan) = fixture().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let old = repo
            .session(id.clone(), Platform::Buff)
            .await
            .unwrap()
            .unwrap();
        let app = BuffResearch::new(
            repo.clone(),
            auth.clone(),
            Source {
                calls: calls.clone(),
                error: Some(error),
                empty: false,
            },
        );
        assert!(app.fetch(&id, request(&scan)).await.is_err());
        assert!(app.list(&id).await.unwrap().is_empty());
        let policy = repo.buff_policy().await.unwrap();
        let session = repo
            .session(id.clone(), Platform::Buff)
            .await
            .unwrap()
            .unwrap();
        match error {
            BuffReadError::Challenge => {
                assert!(policy.1);
                assert_eq!(session.view.status, SessionStatus::VerificationRequired);
            }
            BuffReadError::RateLimited(_) => assert!(policy.0 >= now() + 299),
            BuffReadError::Expired => assert_eq!(session.view.status, SessionStatus::Invalid),
            BuffReadError::CredentialsChanged => {
                assert_eq!(session.view.status, SessionStatus::Unverified);
                assert_ne!(old.view.generation, session.view.generation);
            }
            BuffReadError::Network => assert_eq!(session.view.status, SessionStatus::Valid),
            _ => unreachable!(),
        }
        assert!(app.fetch(&id, request(&scan)).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
#[tokio::test]
async fn invalid_context_stale_session_and_cross_site_never_call_the_platform() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (repo, auth, id, mut scan) = fixture().await;
    let calls = Arc::new(AtomicUsize::new(0));
    let app = BuffResearch::new(
        repo.clone(),
        auth,
        Source {
            calls: calls.clone(),
            error: None,
            empty: false,
        },
    );
    assert!(app.fetch("missing", request(&scan)).await.is_err());
    scan.request.source = SourceKind::Demo;
    repo.save(scan.clone()).await.unwrap();
    assert!(app.fetch(&id, request(&scan)).await.is_err());
    scan.request.source = SourceKind::Steamdt;
    repo.save(scan.clone()).await.unwrap();
    let mut session = repo
        .session(id.clone(), Platform::Buff)
        .await
        .unwrap()
        .unwrap();
    session.view.checked_at = Some(now() - 301);
    repo.save_session(id.clone(), session).await.unwrap();
    assert!(app.fetch(&id, request(&scan)).await.is_err());
    let router = crate::transport::buff_quotes::router(app);
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/accounts/{id}/buff-quotes"))
                .header("content-type", "application/json")
                .header("host", "127.0.0.1:38480")
                .header("origin", "https://untrusted.test")
                .body(Body::from(serde_json::to_string(&request(&scan)).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

struct BlockingSource {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl BuffQuoteProvider for BlockingSource {
    async fn sell_page(
        &self,
        goods_id: u64,
        _: Credentials,
    ) -> Result<BuffSellPage, BuffReadError> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(BuffSellPage {
            goods_id,
            market_hash_name: "fixture (Field-Tested)".into(),
            levels: vec![],
            reported_total: Some(0),
        })
    }
}
#[tokio::test]
async fn reading_and_revocation_share_the_same_gate_and_stale_evidence_is_rejected() {
    let (repo, auth, id, scan) = fixture().await;
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let app = BuffResearch::new(
        repo.clone(),
        auth.clone(),
        BlockingSource {
            entered: entered.clone(),
            release: release.clone(),
        },
    );
    let id_read = id.clone();
    let input = request(&scan);
    let fetch = tokio::spawn(async move { app.fetch(&id_read, input).await });
    entered.notified().await;
    let id_revoke = id.clone();
    let mut revoke = tokio::spawn(async move { auth.revoke(&id_revoke, Platform::Buff).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(30), &mut revoke)
            .await
            .is_err(),
        "revocation must wait for the logical read operation"
    );
    release.notify_one();
    let quote = fetch.await.unwrap().unwrap();
    revoke.await.unwrap().unwrap();
    let application = Application::new(Provider::new().unwrap(), repo);
    assert!(
        application
            .create_plan(plan_request(&scan, &id, &quote))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn plan_rejects_wrong_account_generation_identity_and_stale_snapshots() {
    let (repo, auth, id, scan) = fixture().await;
    let app = BuffResearch::new(
        repo.clone(),
        auth,
        Source {
            calls: Arc::new(AtomicUsize::new(0)),
            error: None,
            empty: false,
        },
    );
    let quote = app.fetch(&id, request(&scan)).await.unwrap();
    let application = Application::new(Provider::new().unwrap(), repo.clone());
    for variant in 0..5 {
        let mut q = quote.clone();
        q.id = uuid::Uuid::new_v4().to_string();
        match variant {
            0 => q.account_id = "wrong-account".into(),
            1 => q.credential_generation = "old-generation".into(),
            2 => q.page.goods_id = 321,
            3 => q.observed_at = now() - 301,
            _ => q.observed_at = now() + 100,
        }
        let request = plan_request(&scan, &id, &q);
        repo.insert_buff_quote(q).await.unwrap();
        assert!(
            application.create_plan(request).await.is_err(),
            "variant {variant}"
        );
    }
    assert!(application.plans().await.unwrap().is_empty());
}
