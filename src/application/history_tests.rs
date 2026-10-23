use super::*;
use crate::{
    adapters::sqlite::Store,
    application::account_tests::{TestVault, TestVerifier},
    domain::{
        accounts::*,
        analysis::evaluate,
        money::Currency,
        selection::{MarketItem, ScanReport, ScanRequest},
    },
};
use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Notify;
type Auth = Accounts<Store, TestVault, TestVerifier>;
struct Source {
    calls: Arc<AtomicUsize>,
    error: Option<SteamReadError>,
    entered: Option<Arc<Notify>>,
    release: Option<Arc<Notify>>,
}
fn source(error: Option<SteamReadError>) -> Source {
    Source {
        calls: Arc::new(AtomicUsize::new(0)),
        error,
        entered: None,
        release: None,
    }
}
impl HistoryProvider for Source {
    async fn history(
        &self,
        request: &HistoryRequest,
        secret: &Credentials,
        at: u64,
    ) -> Result<HistorySeries, SteamReadError> {
        assert!(secret.cookies.contains("synthetic-secret"));
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(n) = &self.entered {
            n.notify_one();
        }
        if let Some(n) = &self.release {
            n.notified().await;
        }
        if let Some(error) = self.error {
            return Err(error);
        }
        let today = at / 86400;
        Ok(HistorySeries {
            currency: Some(Currency::cny()),
            currency_basis: "explicit_code_or_unique_marker".into(),
            points: (0..8)
                .map(|i| HistorySample {
                    at: (today - 8 + i) * 86400,
                    price_major_1e4: 100000 + i * 1000,
                    volume: Some(10),
                })
                .collect(),
            response_rows: 8,
            excluded_before_window: 0,
            excluded_open_day: 0,
            from_utc_day: today - u64::from(request.window_days),
            until_utc_day_exclusive: today,
        })
    }
}
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
    let a = auth
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    auth.import(
        &a.id,
        Platform::Steam,
        Credentials {
            cookies: "steamLoginSecure=76561198000000001%7C%7Csynthetic-secret".into(),
            user_agent: "synthetic-UA".into(),
        },
    )
    .await
    .unwrap();
    let mut session = repo
        .session(a.id.clone(), Platform::Steam)
        .await
        .unwrap()
        .unwrap();
    session.view.status = SessionStatus::Valid;
    session.view.subject = Some(a.steam_id);
    session.view.checked_at = Some(now());
    repo.save_session(a.id.clone(), session).await.unwrap();
    let mut scan = ScanReport::new(ScanRequest {
        source: SourceKind::Steamdt,
        ..Default::default()
    });
    scan.status = "completed".into();
    scan.finished_at = Some(now());
    scan.candidates.push(evaluate(
        MarketItem {
            key: "fixture".into(),
            name: "fixture".into(),
            goods_id: Some(1),
            cash_cents: 500,
            steam_gross_cents: 1000,
            volume: 500,
            observed_at: now(),
        },
        &scan.request,
    ));
    repo.save(scan.clone()).await.unwrap();
    (repo, auth, a.id, scan)
}
fn request(scan: &ScanReport) -> HistoryRequest {
    HistoryRequest {
        scan_id: scan.id.clone(),
        item_key: "fixture".into(),
        window_days: 30,
        slippage_bps: 300,
    }
}
#[tokio::test]
async fn history_http_binds_candidate_account_and_saves_immutable_native_research() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (repo, auth, id, scan) = fixture().await;
    let src = source(None);
    let calls = src.calls.clone();
    let app = HistoryResearch::new(repo.clone(), auth.clone(), src);
    let router = crate::transport::history::router(app.clone());
    let response = router
        .oneshot(
            Request::post(format!("/api/accounts/{id}/price-histories"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request(&scan)).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let raw = std::str::from_utf8(&bytes).unwrap();
    assert!(
        !raw.contains("synthetic-secret")
            && !raw.contains("synthetic-UA")
            && !raw.contains("credential_key")
    );
    let saved: HistorySnapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(saved.analysis.risk.as_ref().unwrap().sample_count, 1);
    assert_eq!(
        saved.analysis.reference_utc_day,
        Some(saved.series.until_utc_day_exclusive - 1)
    );
    assert!(!saved.platform_verified);
    assert!(repo.insert_history(saved.clone()).await.is_err());
    let a = auth
        .create("other".into(), "76561198000000002".into())
        .await
        .unwrap();
    assert!(matches!(
        app.get(&a.id, saved.id.clone()).await,
        Err(AppError::Missing(_))
    ));
    auth.revoke(&id, Platform::Steam).await.unwrap();
    assert_eq!(app.get(&id, saved.id).await.unwrap().series.points.len(), 8);
    assert_eq!(app.list(&id).await.unwrap().len(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        repo.get(scan.id).await.unwrap().unwrap().candidates[0]
            .item
            .steam_gross_cents,
        1000
    );
}
#[tokio::test]
async fn history_guards_demo_bad_candidate_session_age_and_cross_site_before_secrets_or_platform() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (repo, auth, id, scan) = fixture().await;
    let src = source(None);
    let calls = src.calls.clone();
    let app = HistoryResearch::new(repo.clone(), auth.clone(), src);
    let mut req = request(&scan);
    req.item_key = "not-in-scan".into();
    assert!(app.fetch(&id, req).await.is_err());
    let mut demo = scan.clone();
    demo.request.source = SourceKind::Demo;
    repo.save(demo).await.unwrap();
    assert!(app.fetch(&id, request(&scan)).await.is_err());
    repo.save(scan.clone()).await.unwrap();
    let original = repo
        .session(id.clone(), Platform::Steam)
        .await
        .unwrap()
        .unwrap();
    for variant in 0..4 {
        let mut s = original.clone();
        match variant {
            0 => s.view.status = SessionStatus::Unverified,
            1 => s.view.checked_at = Some(now() - 301),
            2 => s.view.checked_at = Some(now() + 20),
            _ => s.view.subject = Some("wrong".into()),
        }
        repo.save_session(id.clone(), s).await.unwrap();
        assert!(app.fetch(&id, request(&scan)).await.is_err());
    }
    let router = crate::transport::history::router(app);
    let response = router
        .clone()
        .oneshot(
            Request::post(format!("/api/accounts/{id}/price-histories"))
                .header("host", "127.0.0.1:38480")
                .header("origin", "https://untrusted.example")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request(&scan)).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = router
        .oneshot(
            Request::get("/api/accounts/missing/price-histories")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn history_errors_reuse_identity_updates_and_persistent_scoped_rate_limits() {
    for error in [
        SteamReadError::RateLimited(1),
        SteamReadError::Rejected,
        SteamReadError::Network,
        SteamReadError::Expired,
        SteamReadError::CredentialsChanged,
    ] {
        let (repo, auth, id, scan) = fixture().await;
        let prior = repo
            .session(id.clone(), Platform::Steam)
            .await
            .unwrap()
            .unwrap();
        repo.set_steam_read_retry(SteamReadScope::Inventory, now() + 100)
            .await
            .unwrap();
        let src = source(Some(error));
        let calls = src.calls.clone();
        let app = HistoryResearch::new(repo.clone(), auth, src);
        assert!(app.fetch(&id, request(&scan)).await.is_err());
        let after = repo
            .session(id.clone(), Platform::Steam)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.view.checked_at, prior.view.checked_at);
        match error {
            SteamReadError::RateLimited(_) => {
                assert!(
                    repo.steam_read_retry(SteamReadScope::PriceHistory)
                        .await
                        .unwrap()
                        >= now() + 299
                );
                assert!(app.fetch(&id, request(&scan)).await.is_err());
            }
            SteamReadError::Expired => assert_eq!(after.view.status, SessionStatus::Invalid),
            SteamReadError::CredentialsChanged => {
                assert_eq!(after.view.status, SessionStatus::Unverified);
                assert_ne!(after.view.generation, prior.view.generation);
            }
            _ => assert_eq!(after.view.status, SessionStatus::Valid),
        }
        assert!(repo.histories(id).await.unwrap().is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(
            repo.steam_read_retry(SteamReadScope::Inventory)
                .await
                .unwrap()
                > now()
        );
    }
}
#[tokio::test]
async fn history_read_and_credential_revocation_share_the_inventory_gate() {
    let (repo, auth, id, scan) = fixture().await;
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let mut src = source(None);
    src.entered = Some(entered.clone());
    src.release = Some(release.clone());
    let app = HistoryResearch::new(repo.clone(), auth.clone(), src);
    let read_id = id.clone();
    let reader = tokio::spawn(async move { app.fetch(&read_id, request(&scan)).await });
    entered.notified().await;
    let revoke_id = id.clone();
    let revoker = tokio::spawn(async move { auth.revoke(&revoke_id, Platform::Steam).await });
    tokio::task::yield_now().await;
    assert!(!revoker.is_finished());
    release.notify_one();
    let snapshot = reader.await.unwrap().unwrap();
    revoker.await.unwrap().unwrap();
    let after = repo
        .session(id.clone(), Platform::Steam)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(snapshot.credential_generation, after.view.generation);
    assert_eq!(repo.histories(id).await.unwrap().len(), 1);
}
#[tokio::test]
async fn historical_results_and_policy_survive_restart_without_recomputing_quotes() {
    let (repo, auth, id, scan) = fixture().await;
    let app = HistoryResearch::new(repo, auth, source(None));
    let snapshot = app.fetch(&id, request(&scan)).await.unwrap();
    let path = std::env::temp_dir().join(format!("parallax-history-{}.db", uuid::Uuid::new_v4()));
    let retry = now() + 300;
    {
        let file = Store::open(&path).unwrap();
        file.insert_history(snapshot.clone()).await.unwrap();
        file.set_steam_read_retry(SteamReadScope::PriceHistory, retry)
            .await
            .unwrap();
    }
    {
        let file = Store::open(&path).unwrap();
        assert_eq!(
            file.steam_read_retry(SteamReadScope::PriceHistory)
                .await
                .unwrap(),
            retry
        );
        assert_eq!(
            file.history(snapshot.id.clone())
                .await
                .unwrap()
                .unwrap()
                .analysis
                .risk
                .unwrap()
                .sample_count,
            1
        );
        assert!(file.insert_history(snapshot).await.is_err());
    }
    std::fs::remove_file(path).unwrap();
}
