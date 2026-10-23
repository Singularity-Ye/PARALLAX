use super::*;
use crate::{
    adapters::sqlite::Store,
    application::account_tests::{TestVault, TestVerifier},
    domain::accounts::*,
};
use std::{
    collections::VecDeque,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{Mutex, Notify};
type Auth = Accounts<Store, TestVault, TestVerifier>;
struct Source {
    pages: Mutex<VecDeque<Result<InventoryPage, SteamReadError>>>,
    calls: Arc<AtomicUsize>,
}
impl InventoryProvider for Source {
    async fn page(
        &self,
        steam_id: &str,
        context: &str,
        cursor: Option<&str>,
        secret: &Credentials,
    ) -> Result<InventoryPage, SteamReadError> {
        assert_eq!(steam_id, "76561198000000001");
        assert!(secret.cookies.contains("synthetic-secret"));
        assert!(["2", "16"].contains(&context));
        assert!(cursor.is_none_or(|s| numeric_id(s, false)));
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.pages
            .lock()
            .await
            .pop_front()
            .expect("unexpected platform call")
    }
}
fn source(pages: Vec<Result<InventoryPage, SteamReadError>>) -> Source {
    Source {
        pages: Mutex::new(pages.into()),
        calls: Arc::new(AtomicUsize::new(0)),
    }
}
fn asset(id: &str, context: &str) -> InventoryAsset {
    InventoryAsset {
        app_id: 730,
        context_id: context.into(),
        asset_id: id.into(),
        class_id: "10".into(),
        instance_id: "0".into(),
        amount: 1,
        market_hash_name: Some("fixture".into()),
        reported_tradable: true,
        reported_marketable: true,
        restriction_note_present: false,
    }
}
fn page(
    context: &str,
    ids: &[&str],
    total: u32,
    next: Option<&str>,
) -> Result<InventoryPage, SteamReadError> {
    Ok(InventoryPage {
        assets: ids.iter().map(|id| asset(id, context)).collect(),
        total,
        next_asset_id: next.map(str::to_owned),
    })
}
async fn fixture() -> (Store, Arc<Auth>, String) {
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
        Platform::Steam,
        Credentials {
            cookies: "steamLoginSecure=76561198000000001%7C%7Csynthetic-secret".into(),
            user_agent: "synthetic-UA".into(),
        },
    )
    .await
    .unwrap();
    let mut s = repo
        .session(account.id.clone(), Platform::Steam)
        .await
        .unwrap()
        .unwrap();
    s.view.status = SessionStatus::Valid;
    s.view.subject = Some(account.steam_id);
    s.view.checked_at = Some(now());
    repo.save_session(account.id.clone(), s).await.unwrap();
    (repo, auth, account.id)
}
#[tokio::test]
async fn complete_two_context_snapshot_is_immutable_account_bound_and_redacted() {
    let (repo, auth, id) = fixture().await;
    let app = InventoryResearch::new(
        repo.clone(),
        auth.clone(),
        source(vec![
            page("2", &["100"], 2, Some("100")),
            page("2", &["101"], 2, None),
            page("16", &["102"], 1, None),
        ]),
    );
    let snapshot = app.fetch(&id, InventoryRequest::default()).await.unwrap();
    assert!(snapshot.coverage_complete());
    assert_eq!(snapshot.assets.len(), 3);
    assert_eq!(snapshot.coverage[0].pages_read, 2);
    assert!(!snapshot.assets[2].reported_transferable());
    assert!(!snapshot.platform_verified);
    let raw = serde_json::to_string(&snapshot).unwrap();
    assert!(
        !raw.contains("synthetic-secret")
            && !raw.contains("synthetic-UA")
            && !raw.contains("credential_key")
    );
    assert!(repo.insert_inventory(snapshot.clone()).await.is_err());
    auth.revoke(&id, Platform::Steam).await.unwrap();
    assert_eq!(
        app.get(&id, snapshot.id.clone())
            .await
            .unwrap()
            .assets
            .len(),
        3
    ); // history remains history
    let other = auth
        .create("other".into(), "76561198000000002".into())
        .await
        .unwrap();
    assert!(matches!(
        app.get(&other.id, snapshot.id).await,
        Err(AppError::Missing(_))
    ));
    assert!(app.list(&other.id).await.unwrap().is_empty());
}
#[tokio::test]
async fn page_budget_is_explicit_and_second_context_failure_never_saves_partial_success() {
    let (repo, auth, id) = fixture().await;
    let app = InventoryResearch::new(
        repo.clone(),
        auth.clone(),
        source(vec![
            page("2", &["100"], 3, Some("100")),
            page("16", &[], 0, None),
        ]),
    );
    let snapshot = app
        .fetch(
            &id,
            InventoryRequest {
                max_pages_per_context: 1,
            },
        )
        .await
        .unwrap();
    assert!(!snapshot.coverage_complete());
    assert_eq!(snapshot.coverage[0].stop_reason, "page_budget");
    assert_eq!(snapshot.coverage[1].observed_assets, 0);
    repo.set_steam_read_retry(SteamReadScope::Inventory, 0)
        .await
        .unwrap();
    let failed = InventoryResearch::new(
        repo.clone(),
        auth,
        source(vec![
            page("2", &["101"], 1, None),
            Err(SteamReadError::Network),
        ]),
    );
    assert!(
        failed
            .fetch(&id, InventoryRequest::default())
            .await
            .is_err()
    );
    let history = failed.list(&id).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, snapshot.id);
}
#[tokio::test]
async fn pagination_drift_totals_duplicates_and_missing_assets_never_become_full_holdings() {
    for pages in [
        vec![
            page("2", &["100"], 2, Some("100")),
            page("2", &["101"], 3, None),
        ],
        vec![
            page("2", &["100"], 2, Some("100")),
            page("2", &["100"], 2, None),
        ],
        vec![page("2", &["100"], 1, None), page("16", &["100"], 1, None)],
        vec![page("2", &["100"], 2, None)],
    ] {
        let (repo, auth, id) = fixture().await;
        let app = InventoryResearch::new(repo.clone(), auth, source(pages));
        assert!(matches!(
            app.fetch(&id, InventoryRequest::default()).await,
            Err(AppError::SteamRead(SteamReadError::ChangedDuringRead))
        ));
        assert!(repo.inventories(id).await.unwrap().is_empty());
    }
}
#[tokio::test]
async fn request_identity_freshness_and_same_origin_guard_run_before_platform_reads() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (repo, auth, id) = fixture().await;
    let src = source(vec![]);
    let calls = src.calls.clone();
    let app = InventoryResearch::new(repo.clone(), auth.clone(), src);
    assert!(
        app.fetch(
            &id,
            InventoryRequest {
                max_pages_per_context: 11
            }
        )
        .await
        .is_err()
    );
    assert!(
        app.fetch("missing", InventoryRequest::default())
            .await
            .is_err()
    );
    let original = repo
        .session(id.clone(), Platform::Steam)
        .await
        .unwrap()
        .unwrap();
    for variant in 0..5 {
        let mut s = original.clone();
        match variant {
            0 => s.view.status = SessionStatus::Unverified,
            1 => s.view.checked_at = Some(now() - 301),
            2 => s.view.checked_at = Some(now() + 10),
            3 => s.view.subject = Some("76561198000000002".into()),
            _ => s.view.subject = None,
        }
        repo.save_session(id.clone(), s).await.unwrap();
        assert!(app.fetch(&id, InventoryRequest::default()).await.is_err());
    }
    let router = crate::transport::inventory::router(app);
    let response = router
        .clone()
        .oneshot(
            Request::post(format!("/api/accounts/{id}/inventories"))
                .header("host", "127.0.0.1:38480")
                .header("origin", "https://untrusted.example")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let response = router
        .oneshot(
            Request::get("/api/accounts/missing/inventories")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn read_failures_persist_cooldown_or_identity_changes_without_erasing_old_evidence() {
    for error in [
        SteamReadError::RateLimited(1),
        SteamReadError::Rejected,
        SteamReadError::Expired,
        SteamReadError::Network,
        SteamReadError::CredentialsChanged,
    ] {
        let (repo, auth, id) = fixture().await;
        let prior = repo
            .session(id.clone(), Platform::Steam)
            .await
            .unwrap()
            .unwrap();
        let src = source(vec![Err(error)]);
        let calls = src.calls.clone();
        let app = InventoryResearch::new(repo.clone(), auth.clone(), src);
        assert!(app.fetch(&id, InventoryRequest::default()).await.is_err());
        let after = repo
            .session(id.clone(), Platform::Steam)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.view.checked_at, prior.view.checked_at);
        match error {
            SteamReadError::RateLimited(_) => {
                assert!(
                    repo.steam_read_retry(SteamReadScope::Inventory)
                        .await
                        .unwrap()
                        >= now() + 299
                );
                assert_eq!(after.view.status, SessionStatus::Valid);
                // Import and another account cannot clear the app-wide inventory cooldown.
                auth.import(
                    &id,
                    Platform::Steam,
                    Credentials {
                        cookies: "steamLoginSecure=76561198000000001%7C%7Csynthetic-secret".into(),
                        user_agent: "".into(),
                    },
                )
                .await
                .unwrap();
                assert!(
                    repo.steam_read_retry(SteamReadScope::Inventory)
                        .await
                        .unwrap()
                        >= now() + 299
                );
                let other = auth
                    .create("other".into(), "76561198000000002".into())
                    .await
                    .unwrap();
                let mut s = prior.clone();
                s.view.subject = Some(other.steam_id);
                s.view.retry_at = 0;
                repo.save_session(other.id.clone(), s).await.unwrap();
                assert!(matches!(
                    app.fetch(&other.id, InventoryRequest::default()).await,
                    Err(AppError::SteamRead(SteamReadError::RateLimited(_)))
                ));
            }
            SteamReadError::Expired => assert_eq!(after.view.status, SessionStatus::Invalid),
            SteamReadError::CredentialsChanged => {
                assert_eq!(after.view.status, SessionStatus::Unverified);
                assert_ne!(after.view.generation, prior.view.generation);
            }
            _ => assert_eq!(after.view.status, SessionStatus::Valid),
        }
        assert!(repo.inventories(id).await.unwrap().is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn inventory_snapshot_and_global_cooldown_survive_database_reopen() {
    let (memory, auth, id) = fixture().await;
    let app = InventoryResearch::new(
        memory,
        auth,
        source(vec![page("2", &[], 0, None), page("16", &[], 0, None)]),
    );
    let snapshot = app.fetch(&id, InventoryRequest::default()).await.unwrap();
    let path = std::env::temp_dir().join(format!("parallax-inventory-{}.db", uuid::Uuid::new_v4()));
    let retry = now() + 300;
    {
        let file = Store::open(&path).unwrap();
        file.insert_inventory(snapshot.clone()).await.unwrap();
        file.set_steam_read_retry(SteamReadScope::Inventory, retry)
            .await
            .unwrap();
    }
    {
        let file = Store::open(&path).unwrap();
        assert_eq!(
            file.steam_read_retry(SteamReadScope::Inventory)
                .await
                .unwrap(),
            retry
        );
        let restored = file.inventory(snapshot.id.clone()).await.unwrap().unwrap();
        assert!(restored.coverage_complete());
        assert!(restored.assets.is_empty());
        assert!(file.insert_inventory(snapshot).await.is_err());
    }
    std::fs::remove_file(path).unwrap();
}
struct PausedSource {
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
impl InventoryProvider for PausedSource {
    async fn page(
        &self,
        _: &str,
        context: &str,
        _: Option<&str>,
        _: &Credentials,
    ) -> Result<InventoryPage, SteamReadError> {
        if context == "2" {
            self.entered.notify_one();
            self.release.notified().await;
        }
        page(context, &[], 0, None)
    }
}
#[tokio::test]
async fn inventory_read_holds_the_existing_credential_gate_until_both_contexts_finish() {
    let (repo, auth, id) = fixture().await;
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let app = InventoryResearch::new(
        repo.clone(),
        auth.clone(),
        PausedSource {
            entered: entered.clone(),
            release: release.clone(),
        },
    );
    let read_id = id.clone();
    let reader =
        tokio::spawn(async move { app.fetch(&read_id, InventoryRequest::default()).await });
    entered.notified().await;
    let revoke_id = id.clone();
    let revoker = tokio::spawn(async move { auth.revoke(&revoke_id, Platform::Steam).await });
    tokio::task::yield_now().await;
    assert!(!revoker.is_finished());
    release.notify_one();
    reader.await.unwrap().unwrap();
    revoker.await.unwrap().unwrap();
    assert_eq!(
        repo.session(id, Platform::Steam)
            .await
            .unwrap()
            .unwrap()
            .view
            .status,
        SessionStatus::Missing
    );
}
