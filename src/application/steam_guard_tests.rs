use super::*;
use crate::{adapters::sqlite::Store, domain::accounts::Account};
use std::collections::HashMap;
#[derive(Clone, Default)]
struct Memory(Arc<tokio::sync::Mutex<HashMap<String, String>>>);
impl GuardSecretStore for Memory {
    async fn guard_secret(&self, key: String) -> anyhow::Result<Option<GuardSecret>> {
        self.0
            .lock()
            .await
            .get(&key)
            .map(|s| GuardSecret::parse(s).map_err(anyhow::Error::msg))
            .transpose()
    }
    async fn put_guard_secret(&self, key: String, secret: GuardSecret) -> anyhow::Result<()> {
        self.0.lock().await.insert(key, secret.encoded());
        Ok(())
    }
    async fn remove_guard_secret(&self, key: String) -> anyhow::Result<()> {
        self.0.lock().await.remove(&key);
        Ok(())
    }
}
#[tokio::test]
async fn steam_guard_http_binds_accounts_redacts_secrets_and_only_generates_on_request() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let db = Store::open(std::path::Path::new(":memory:")).unwrap();
    for id in ["a", "b"] {
        db.add_account(Account {
            id: id.into(),
            steam_id: if id == "a" {
                "76561198000000001"
            } else {
                "76561198000000002"
            }
            .into(),
            label: "fixture".into(),
            created_at: 1,
        })
        .await
        .unwrap();
    }
    let vault = Memory::default();
    let service = SteamGuard::new(db, vault.clone());
    assert!(!service.status("a").await.unwrap().configured);
    let router = crate::transport::steam_guard::router(service.clone());
    let body = r#"{"shared_secret":"MTIzNDU2Nzg5MDEyMzQ1Njc4OTA="}"#;
    let cross = router
        .clone()
        .oneshot(
            Request::post("/api/accounts/a/steam-guard")
                .header("origin", "https://other.invalid")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cross.status(), StatusCode::FORBIDDEN);
    let response = router
        .clone()
        .oneshot(
            Request::post("/api/accounts/a/steam-guard")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 2048).await.unwrap();
    let raw = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!raw.contains("MTIz"));
    assert!(
        serde_json::from_str::<serde_json::Value>(&raw)
            .unwrap()
            .get("code")
            .is_none()
    );
    assert!(service.status("a").await.unwrap().configured);
    assert!(!service.status("b").await.unwrap().configured);
    assert!(service.code("b").await.is_err());
    assert!(service.status("missing").await.is_err());
    let response = router
        .oneshot(
            Request::post("/api/accounts/a/steam-guard/code")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 2048).await.unwrap()).unwrap();
    assert_eq!(value["code"].as_str().unwrap().len(), 5);
    assert_eq!(value["automatic_login_available"], false);
    assert_eq!(vault.0.lock().await.len(), 1);
    service.remove("a").await.unwrap();
    assert!(service.code("a").await.is_err());
    assert!(vault.0.lock().await.is_empty());
}
