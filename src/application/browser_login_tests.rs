use crate::adapters::sqlite::Store;
use crate::application::{account_tests::TestVault, accounts::Accounts, browser_login::*};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Mutex;

#[derive(Clone)]
struct Verifier {
    status: Arc<Mutex<SessionStatus>>,
    calls: Arc<AtomicUsize>,
}
impl SessionVerifier for Verifier {
    async fn verify(&self, account: &Account, _: Platform, _: Credentials) -> Verification {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let status = *self.status.lock().await;
        Verification {
            status,
            message: "synthetic verification",
            subject: (status == SessionStatus::Valid).then(|| account.steam_id.clone()),
            retry_secs: 30,
        }
    }
}
#[derive(Clone, Default)]
struct Browser {
    launched: Arc<AtomicUsize>,
    closed: Arc<AtomicUsize>,
}
impl BrowserAuth for Browser {
    type Window = ();
    async fn launch(&self, _: Platform) -> Result<(), &'static str> {
        self.launched.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn capture(&self, _: &(), platform: Platform) -> Result<Credentials, &'static str> {
        Ok(secret(platform))
    }
    async fn close(&self, _: ()) -> Result<(), &'static str> {
        self.closed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
fn secret(platform: Platform) -> Credentials {
    Credentials {
        cookies: match platform {
            Platform::Steam => "steamLoginSecure=76561198000000001%7C%7Csynthetic-new",
            Platform::Buff => "session=synthetic-new",
        }
        .into(),
        user_agent: "synthetic-UA".into(),
    }
}
fn fixture() -> (Store, TestVault, Verifier, Browser) {
    (
        Store::open(Path::new(":memory:")).unwrap(),
        TestVault::default(),
        Verifier {
            status: Arc::new(Mutex::new(SessionStatus::Valid)),
            calls: Arc::new(AtomicUsize::new(0)),
        },
        Browser::default(),
    )
}

struct UnwritableVault(TestVault);
impl CredentialStore for UnwritableVault {
    async fn put(&self, _: String, _: Credentials) -> anyhow::Result<()> {
        anyhow::bail!("synthetic vault failure")
    }
    async fn get(&self, key: String) -> anyhow::Result<Option<Credentials>> {
        self.0.get(key).await
    }
    async fn remove(&self, key: String) -> anyhow::Result<()> {
        self.0.remove(key).await
    }
}
#[tokio::test]
async fn vault_failure_does_not_clear_challenge_or_discard_old_secret() {
    let (repo, vault, verifier, browser) = fixture();
    let initial = Accounts::new(repo.clone(), vault.clone(), verifier.clone());
    let account = initial
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    let old = initial
        .import(&account.id, Platform::Buff, secret(Platform::Buff))
        .await
        .unwrap();
    repo.set_buff_policy(0, true).await.unwrap();
    let accounts = Accounts::new(repo.clone(), UnwritableVault(vault.clone()), verifier);
    let app = BrowserLogins::new(accounts, browser);
    let login = app.start(&account.id, Platform::Buff).await.unwrap();
    assert!(app.finish(&login.id).await.is_err());
    assert!(repo.buff_policy().await.unwrap().1);
    assert_eq!(
        repo.session(account.id, Platform::Buff)
            .await
            .unwrap()
            .unwrap()
            .view
            .generation,
        old.generation
    );
    assert!(vault.get(old.generation).await.unwrap().is_some());
    assert!(app.current().await.unwrap().unwrap().status == LoginStatus::Pending);
}
#[tokio::test]
async fn login_binds_account_is_exclusive_and_finish_is_idempotent() {
    let (repo, vault, verifier, browser) = fixture();
    let accounts = Accounts::new(repo.clone(), vault.clone(), verifier.clone());
    let account = accounts
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    let app = BrowserLogins::new(accounts, browser.clone());
    assert!(app.start("missing", Platform::Steam).await.is_err());
    assert_eq!(browser.launched.load(Ordering::SeqCst), 0);
    let login = app.start(&account.id, Platform::Steam).await.unwrap();
    assert!(app.start(&account.id, Platform::Buff).await.is_err());
    assert!(app.finish("wrong-flow").await.is_err());
    let done = app.finish(&login.id).await.unwrap();
    assert!(done.status == LoginStatus::Completed);
    assert_eq!(done.account_id, account.id);
    assert_eq!(done.session.unwrap().status, SessionStatus::Valid);
    assert!(app.finish(&login.id).await.unwrap().status == LoginStatus::Completed);
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    assert_eq!(browser.closed.load(Ordering::SeqCst), 1);
    let stored = repo
        .session(account.id, Platform::Steam)
        .await
        .unwrap()
        .unwrap();
    assert!(vault.get(stored.credential_key).await.unwrap().is_some());
    let json = serde_json::to_string(&app.current().await.unwrap()).unwrap();
    assert!(
        !json.contains("synthetic-new")
            && !json.contains("synthetic-UA")
            && !json.contains("credential_key")
    );
}
#[tokio::test]
async fn stale_login_cannot_undo_import_or_revocation() {
    let (repo, vault, verifier, browser) = fixture();
    let accounts = Accounts::new(repo.clone(), vault, verifier.clone());
    let account = accounts
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    let app = BrowserLogins::new(accounts.clone(), browser);
    let login = app.start(&account.id, Platform::Steam).await.unwrap();
    accounts
        .import(&account.id, Platform::Steam, secret(Platform::Steam))
        .await
        .unwrap();
    assert!(app.finish(&login.id).await.is_err());
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 0);
    app.cancel(&login.id).await.unwrap();
    let login = app.start(&account.id, Platform::Steam).await.unwrap();
    let before = accounts.sessions(&account.id).await.unwrap()[0]
        .generation
        .clone();
    let revoked = accounts.revoke(&account.id, Platform::Steam).await.unwrap();
    assert_ne!(before, revoked.generation);
    assert!(app.finish(&login.id).await.is_err());
    assert_eq!(
        accounts.sessions(&account.id).await.unwrap()[0].status,
        SessionStatus::Missing
    );
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn failed_recovery_retains_secret_and_challenge_then_verified_recovery_clears_it() {
    let (repo, vault, verifier, browser) = fixture();
    let accounts = Accounts::new(repo.clone(), vault.clone(), verifier.clone());
    let account = accounts
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    let old = accounts
        .import(
            &account.id,
            Platform::Buff,
            Credentials {
                cookies: "session=synthetic-old".into(),
                user_agent: "old-UA".into(),
            },
        )
        .await
        .unwrap();
    repo.set_buff_policy(0, true).await.unwrap();
    let app = BrowserLogins::new(accounts.clone(), browser);
    let login = app.start(&account.id, Platform::Buff).await.unwrap();
    *verifier.status.lock().await = SessionStatus::Unknown;
    assert!(app.finish(&login.id).await.is_err());
    let after = repo
        .session(account.id.clone(), Platform::Buff)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old.generation, after.view.generation);
    assert_eq!(
        vault
            .get(after.credential_key.clone())
            .await
            .unwrap()
            .unwrap()
            .cookies,
        "session=synthetic-old"
    );
    assert!(repo.buff_policy().await.unwrap().1);
    assert!(app.finish(&login.id).await.is_err());
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 1);
    assert!(accounts.verify(&account.id, Platform::Buff).await.is_err());
    // Advance synthetic pacing metadata without making a real platform request.
    let mut after = after;
    after.view.retry_at = 0;
    repo.save_session(account.id.clone(), after).await.unwrap();
    repo.set_buff_policy(0, true).await.unwrap();
    app.active.lock().await.as_mut().unwrap().retry_at = 0;
    *verifier.status.lock().await = SessionStatus::Valid;
    let done = app.finish(&login.id).await.unwrap();
    assert_eq!(done.session.unwrap().status, SessionStatus::Valid);
    assert!(!repo.buff_policy().await.unwrap().1);
    assert!(vault.get(old.generation).await.unwrap().is_none());
}
#[tokio::test]
async fn cancellation_and_expiry_close_only_their_window() {
    let (repo, vault, verifier, browser) = fixture();
    let accounts = Accounts::new(repo, vault, verifier.clone());
    let account = accounts
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    let app = BrowserLogins::new(accounts, browser.clone());
    let first = app.start(&account.id, Platform::Steam).await.unwrap();
    app.cancel(&first.id).await.unwrap();
    app.cancel(&first.id).await.unwrap();
    assert_eq!(browser.closed.load(Ordering::SeqCst), 1);
    assert!(app.finish(&first.id).await.is_err());
    let second = app.start(&account.id, Platform::Buff).await.unwrap();
    assert!(app.cancel(&first.id).await.is_err());
    app.active.lock().await.as_mut().unwrap().view.expires_at = now() - 1;
    assert!(app.finish(&second.id).await.is_err());
    assert!(app.current().await.unwrap().unwrap().status == LoginStatus::Expired);
    assert_eq!(browser.closed.load(Ordering::SeqCst), 2);
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn wrong_identity_and_cooldown_do_not_replace_session() {
    let (repo, vault, verifier, browser) = fixture();
    let accounts = Accounts::new(repo.clone(), vault, verifier.clone());
    let account = accounts
        .create("other".into(), "76561198000000002".into())
        .await
        .unwrap();
    let app = BrowserLogins::new(accounts.clone(), browser);
    let login = app.start(&account.id, Platform::Steam).await.unwrap();
    assert!(app.finish(&login.id).await.is_err());
    assert!(accounts.sessions(&account.id).await.unwrap().is_empty());
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 0);
    app.cancel(&login.id).await.unwrap();
    let account = accounts
        .create("buff".into(), "76561198000000001".into())
        .await
        .unwrap();
    repo.set_buff_policy(now() + 300, true).await.unwrap();
    let login = app.start(&account.id, Platform::Buff).await.unwrap();
    assert!(app.finish(&login.id).await.is_err());
    assert_eq!(verifier.calls.load(Ordering::SeqCst), 0);
    assert!(repo.buff_policy().await.unwrap().1);
}
#[tokio::test]
async fn browser_http_blocks_cross_site_launch_and_recovers_pending_status() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let (repo, vault, verifier, browser) = fixture();
    let accounts = Accounts::new(repo, vault, verifier);
    let account = accounts
        .create("fixture".into(), "76561198000000001".into())
        .await
        .unwrap();
    let app = BrowserLogins::new(accounts, browser.clone());
    let router = crate::transport::browser_login::router(app.clone());
    let request = Request::builder()
        .method("POST")
        .uri(format!(
            "/api/accounts/{}/sessions/steam/browser",
            account.id
        ))
        .header("host", "127.0.0.1:38480")
        .header("origin", "https://untrusted.test")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        router.clone().oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(browser.launched.load(Ordering::SeqCst), 0);
    let started = app.start(&account.id, Platform::Steam).await.unwrap();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/browser-logins")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["item"]["id"], started.id);
    assert_eq!(body["item"]["status"], "pending");
}
