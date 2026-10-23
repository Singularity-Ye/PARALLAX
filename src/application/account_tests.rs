use super::{accounts::Accounts, ports::*};
use crate::{adapters::sqlite::Store, domain::accounts::*};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub(crate) struct TestVault(Arc<Mutex<HashMap<String, (String, String)>>>);
impl CredentialStore for TestVault {
    async fn put(&self, key: String, secret: Credentials) -> anyhow::Result<()> {
        self.0
            .lock()
            .await
            .insert(key, (secret.cookies, secret.user_agent));
        Ok(())
    }
    async fn get(&self, key: String) -> anyhow::Result<Option<Credentials>> {
        Ok(self.0.lock().await.get(&key).map(|(c, u)| Credentials {
            cookies: c.clone(),
            user_agent: u.clone(),
        }))
    }
    async fn remove(&self, key: String) -> anyhow::Result<()> {
        self.0.lock().await.remove(&key);
        Ok(())
    }
}
pub(crate) struct TestVerifier {
    pub status: SessionStatus,
    pub calls: Arc<AtomicUsize>,
}
impl SessionVerifier for TestVerifier {
    async fn verify(&self, _: &Account, _: Platform, _: Credentials) -> Verification {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Verification {
            status: self.status,
            message: "fixture result",
            subject: None,
            retry_secs: 300,
        }
    }
}
fn secret(platform: Platform) -> Credentials {
    Credentials {
        cookies: match platform {
            Platform::Steam => "steamLoginSecure=76561198000000001%7C%7Ccanary-secret",
            Platform::Buff => "session=canary-secret",
        }
        .into(),
        user_agent: "fixture-UA".into(),
    }
}
#[tokio::test]
async fn isolated_replacement_revocation_and_redacted_views() {
    let repo = Store::open(Path::new(":memory:")).unwrap();
    let vault = TestVault::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let app = Accounts::new(
        repo.clone(),
        vault.clone(),
        TestVerifier {
            status: SessionStatus::Valid,
            calls: calls.clone(),
        },
    );
    let a = app
        .create("first".into(), "76561198000000001".into())
        .await
        .unwrap();
    let b = app
        .create("second".into(), "76561198000000002".into())
        .await
        .unwrap();
    assert!(
        app.import(&b.id, Platform::Steam, secret(Platform::Steam))
            .await
            .is_err()
    );
    assert!(vault.0.lock().await.is_empty());
    let first = app
        .import(&a.id, Platform::Steam, secret(Platform::Steam))
        .await
        .unwrap();
    assert_eq!(first.status, SessionStatus::Unverified);
    assert_eq!(
        app.verify(&a.id, Platform::Steam).await.unwrap().status,
        SessionStatus::Valid
    );
    assert!(app.verify(&a.id, Platform::Steam).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(app.sessions(&b.id).await.unwrap().is_empty());
    let raw = serde_json::to_string(&app.sessions(&a.id).await.unwrap()).unwrap();
    assert!(!raw.contains("canary-secret") && !raw.contains("cookies"));
    let replaced = app
        .import(&a.id, Platform::Steam, secret(Platform::Steam))
        .await
        .unwrap();
    assert_ne!(first.generation, replaced.generation);
    assert_eq!(vault.0.lock().await.len(), 1);
    assert!(
        app.verify(&a.id, Platform::Steam).await.is_err(),
        "replacement cannot bypass pacing"
    );
    app.revoke(&a.id, Platform::Steam).await.unwrap();
    assert!(vault.0.lock().await.is_empty());
    assert!(app.verify(&a.id, Platform::Steam).await.is_err());
    // Same repository, new application instance: status survives process ownership changes.
    let restarted = Accounts::new(
        repo,
        vault,
        TestVerifier {
            status: SessionStatus::Valid,
            calls,
        },
    );
    assert_eq!(
        restarted.sessions(&a.id).await.unwrap()[0].status,
        SessionStatus::Missing
    );
}

#[tokio::test]
async fn buff_challenge_survives_replacement_revocation_and_new_accounts() {
    let repo = Store::open(Path::new(":memory:")).unwrap();
    let vault = TestVault::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let app = Accounts::new(
        repo.clone(),
        vault.clone(),
        TestVerifier {
            status: SessionStatus::VerificationRequired,
            calls: calls.clone(),
        },
    );
    let a = app
        .create("first".into(), "76561198000000001".into())
        .await
        .unwrap();
    app.import(&a.id, Platform::Buff, secret(Platform::Buff))
        .await
        .unwrap();
    assert_eq!(
        app.verify(&a.id, Platform::Buff).await.unwrap().status,
        SessionStatus::VerificationRequired
    );
    app.revoke(&a.id, Platform::Buff).await.unwrap();
    app.import(&a.id, Platform::Buff, secret(Platform::Buff))
        .await
        .unwrap();
    let restarted = Accounts::new(
        repo,
        vault,
        TestVerifier {
            status: SessionStatus::Valid,
            calls: calls.clone(),
        },
    );
    let b = restarted
        .create("second".into(), "76561198000000002".into())
        .await
        .unwrap();
    restarted
        .import(&b.id, Platform::Buff, secret(Platform::Buff))
        .await
        .unwrap();
    assert!(restarted.verify(&a.id, Platform::Buff).await.is_err());
    assert!(restarted.verify(&b.id, Platform::Buff).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn network_unknown_never_becomes_valid_and_missing_vault_is_explicit() {
    let repo = Store::open(Path::new(":memory:")).unwrap();
    let vault = TestVault::default();
    let app = Accounts::new(
        repo.clone(),
        vault.clone(),
        TestVerifier {
            status: SessionStatus::Unknown,
            calls: Arc::new(AtomicUsize::new(0)),
        },
    );
    let a = app
        .create("first".into(), "76561198000000001".into())
        .await
        .unwrap();
    app.import(&a.id, Platform::Steam, secret(Platform::Steam))
        .await
        .unwrap();
    assert_eq!(
        app.verify(&a.id, Platform::Steam).await.unwrap().status,
        SessionStatus::Unknown
    );
    vault.0.lock().await.clear();
    let mut stored = repo
        .session(a.id.clone(), Platform::Steam)
        .await
        .unwrap()
        .unwrap();
    stored.view.retry_at = 0;
    repo.save_session(a.id.clone(), stored).await.unwrap();
    assert_eq!(
        app.verify(&a.id, Platform::Steam).await.unwrap().status,
        SessionStatus::Missing
    );
}

#[tokio::test]
async fn http_secret_errors_do_not_echo_input_and_cross_site_is_blocked() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let calls = Arc::new(AtomicUsize::new(0));
    let app = Accounts::new(
        Store::open(Path::new(":memory:")).unwrap(),
        TestVault::default(),
        TestVerifier {
            status: SessionStatus::Valid,
            calls: calls.clone(),
        },
    );
    let a = app
        .create("first".into(), "76561198000000001".into())
        .await
        .unwrap();
    let router = crate::transport::accounts::router(app);
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/accounts/{}/sessions/steam", a.id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"cookies":["canary-secret"]}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert!(
        !String::from_utf8(to_bytes(response.into_body(), 8192).await.unwrap().to_vec())
            .unwrap()
            .contains("canary-secret")
    );
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/accounts/{}/sessions/steam/verify", a.id))
                .header("host", "127.0.0.1:38480")
                .header("origin", "https://untrusted.test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn account_metadata_and_buff_policy_survive_database_reopen() {
    let path =
        std::env::temp_dir().join(format!("parallax-account-test-{}.db", uuid::Uuid::new_v4()));
    let id;
    {
        let repo = Store::open(&path).unwrap();
        let app = Accounts::new(
            repo.clone(),
            TestVault::default(),
            TestVerifier {
                status: SessionStatus::Unknown,
                calls: Arc::new(AtomicUsize::new(0)),
            },
        );
        id = app
            .create("persistence fixture".into(), "76561198000000001".into())
            .await
            .unwrap()
            .id;
        app.import(&id, Platform::Steam, secret(Platform::Steam))
            .await
            .unwrap();
        repo.set_buff_policy(123456789, true).await.unwrap();
    }
    let restored;
    let policy;
    let raw;
    {
        let reopened = Store::open(&path).unwrap();
        restored = reopened
            .session(id.clone(), Platform::Steam)
            .await
            .unwrap()
            .unwrap();
        policy = reopened.buff_policy().await.unwrap();
        let db = rusqlite::Connection::open(&path).unwrap();
        raw = db
            .query_row(
                "SELECT metadata FROM account_sessions WHERE account_id=?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .unwrap();
    }
    std::fs::remove_file(path).unwrap();
    assert_eq!(restored.view.status, SessionStatus::Unverified);
    assert_eq!(policy, (123456789, true));
    assert!(!raw.contains("canary-secret") && !raw.contains("fixture-UA"));
}
