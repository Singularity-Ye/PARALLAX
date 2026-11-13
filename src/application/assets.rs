use super::AppError;
use crate::{
    adapters::sqlite::{
        Store,
        maintenance::{CleanupReport, DEFAULT_KEEP_SCANS, MaintenanceStats},
    },
    domain::selection::now,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

pub const DEFAULT_MAX_ICON_BYTES: u64 = 100 * 1024 * 1024;
const MAX_ICON_DOWNLOAD_BYTES: usize = 2 * 1024 * 1024;
const TOUCH_INTERVAL_SECS: u64 = 60;
const LOCK_SHARDS: usize = 64;

#[derive(Clone, Debug)]
pub struct CachedIcon {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupRequest {
    #[serde(default)]
    pub clear_icons: bool,
    #[serde(default)]
    pub clear_logs: bool,
    #[serde(default = "default_keep_scans")]
    pub keep_scans: usize,
}

fn default_keep_scans() -> usize {
    DEFAULT_KEEP_SCANS
}

#[derive(Debug)]
pub enum IconError {
    Invalid(&'static str),
    NotFound,
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for IconError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

pub struct ItemIconCache {
    store: Store,
    icons_dir: PathBuf,
    db_path: PathBuf,
    max_bytes: u64,
    client: reqwest::Client,
    shards: [Mutex<()>; LOCK_SHARDS],
    #[cfg(test)]
    allow_loopback_http: bool,
}

impl ItemIconCache {
    pub fn new(store: Store, data_dir: &Path, db_path: PathBuf) -> anyhow::Result<Arc<Self>> {
        let icons_dir = data_dir.join("cache").join("icons");
        std::fs::create_dir_all(&icons_dir)?;
        let client = build_cdn_client(false)?;
        Ok(Arc::new(Self {
            store,
            icons_dir,
            db_path,
            max_bytes: DEFAULT_MAX_ICON_BYTES,
            client,
            shards: std::array::from_fn(|_| Mutex::new(())),
            #[cfg(test)]
            allow_loopback_http: false,
        }))
    }

    pub async fn stats(&self) -> Result<MaintenanceStats, AppError> {
        Ok(self
            .store
            .maintenance_stats(
                self.icons_dir.clone(),
                self.db_path.clone(),
                self.max_bytes,
                DEFAULT_KEEP_SCANS,
            )
            .await?)
    }

    pub async fn cleanup(&self, req: CleanupRequest) -> Result<CleanupReport, AppError> {
        if !req.clear_icons && !req.clear_logs {
            return Err(AppError::Invalid("请至少选择一项清理内容".into()));
        }
        let keep_scans = req.keep_scans.min(50);
        let mut combined = CleanupReport {
            cleared_icon_files: 0,
            freed_icon_bytes: 0,
            deleted_scans: 0,
            deleted_snapshots: 0,
            reclaimed_db_bytes: 0,
            stats: self.stats().await?,
        };
        if req.clear_icons {
            let r = self
                .store
                .clear_icon_cache(
                    self.icons_dir.clone(),
                    self.db_path.clone(),
                    self.max_bytes,
                    keep_scans,
                )
                .await?;
            combined.cleared_icon_files = r.cleared_icon_files;
            combined.freed_icon_bytes = r.freed_icon_bytes;
            combined.stats = r.stats;
        }
        if req.clear_logs {
            let r = self
                .store
                .cleanup_runtime_logs(
                    self.icons_dir.clone(),
                    self.db_path.clone(),
                    self.max_bytes,
                    keep_scans,
                )
                .await?;
            combined.deleted_scans = r.deleted_scans;
            combined.deleted_snapshots = r.deleted_snapshots;
            combined.reclaimed_db_bytes = r.reclaimed_db_bytes;
            combined.stats = r.stats;
        }
        Ok(combined)
    }

    /// Resolves a CS2 item thumbnail: serves cached bytes offline first, otherwise fetches
    /// from explicit/indexed/SteamDT-CDN sources and persists cleanly under the disk budget.
    pub async fn resolve_icon(
        &self,
        raw_key: &str,
        hint_url: Option<&str>,
    ) -> Result<CachedIcon, IconError> {
        let key = raw_key.trim();
        if key.is_empty() || key.len() > 500 || key.chars().any(char::is_control) {
            return Err(IconError::Invalid("物品名称无效"));
        }
        if is_demo_or_fixture_key(key) {
            return Err(IconError::NotFound);
        }

        if let Some(hit) = self.try_read_cached(key).await? {
            return Ok(hit);
        }

        let shard = shard_index(key);
        let _guard = self.shards[shard].lock().await;
        if let Some(hit) = self.try_read_cached(key).await? {
            return Ok(hit);
        }

        let candidates = self.build_candidate_urls(key, hint_url).await?;
        for url in candidates {
            if let Some((bytes, content_type, ext)) = self.download_valid_image(&url).await {
                let hash = sha256_hex(key);
                let file_name = format!("{hash}.{ext}");
                if let Err(err) = self.write_cached_file(&file_name, &bytes).await {
                    eprintln!("Warning: failed writing icon cache {file_name}: {err}");
                    return Ok(CachedIcon {
                        bytes,
                        content_type: content_type.into(),
                    });
                }
                let _ = self
                    .store
                    .mark_item_icon_cached(
                        key.into(),
                        url,
                        file_name,
                        content_type.into(),
                        bytes.len() as u64,
                        now(),
                    )
                    .await;
                let _ = self
                    .store
                    .evict_icons_over_budget(self.icons_dir.clone(), self.max_bytes)
                    .await;
                return Ok(CachedIcon {
                    bytes,
                    content_type: content_type.into(),
                });
            }
        }
        Err(IconError::NotFound)
    }

    async fn try_read_cached(&self, key: &str) -> Result<Option<CachedIcon>, IconError> {
        let Some(rec) = self.store.lookup_item_icon(key.into()).await? else {
            return Ok(None);
        };
        let Some(fname) = rec.file_name.as_deref().and_then(safe_single_filename) else {
            return Ok(None);
        };
        let path = self.icons_dir.join(fname);
        let Ok(bytes) = tokio::task::spawn_blocking(move || std::fs::read(&path))
            .await
            .map_err(anyhow::Error::from)?
        else {
            return Ok(None);
        };
        if bytes.is_empty() {
            return Ok(None);
        }
        let ct = rec
            .content_type
            .as_deref()
            .filter(|s| s.starts_with("image/"))
            .map(ToString::to_string)
            .or_else(|| sniff_image(&bytes).map(|(c, _)| c.to_string()))
            .unwrap_or_else(|| "image/png".into());
        let ts = now();
        if ts.saturating_sub(rec.last_used_at) >= TOUCH_INTERVAL_SECS {
            let _ = self.store.touch_item_icon(key.into(), ts).await;
        }
        Ok(Some(CachedIcon {
            bytes,
            content_type: ct,
        }))
    }

    async fn build_candidate_urls(
        &self,
        key: &str,
        hint_url: Option<&str>,
    ) -> Result<Vec<String>, IconError> {
        let mut urls = Vec::new();
        let mut seen = HashSet::new();
        let allow_http = self.allow_local_http();

        if let Some(raw_hint) = hint_url
            && let Some(normalized) = normalize_icon_url(raw_hint, allow_http)
            && seen.insert(normalized.clone())
        {
            urls.push(normalized);
        }
        if let Some(rec) = self.store.lookup_item_icon(key.into()).await?
            && let Some(normalized) = normalize_icon_url(&rec.source_url, allow_http)
            && seen.insert(normalized.clone())
        {
            urls.push(normalized);
        }
        if !allow_http {
            let zbt = zbt_fallback_url(key);
            if seen.insert(zbt.clone()) {
                urls.push(zbt);
            }
        }
        Ok(urls)
    }

    async fn download_valid_image(&self, url: &str) -> Option<(Vec<u8>, &'static str, &'static str)> {
        let mut resp = self
            .client
            .get(url)
            .header("accept", "image/png,image/webp,image/jpeg,image/gif,*/*")
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        if !is_allowed_cdn_url(resp.url(), self.allow_local_http()) {
            return None;
        }
        if let Some(len) = resp.content_length()
            && len > MAX_ICON_DOWNLOAD_BYTES as u64
        {
            return None;
        }
        let mut bytes = Vec::with_capacity(32 * 1024);
        while let Ok(Some(chunk)) = resp.chunk().await {
            if bytes.len().saturating_add(chunk.len()) > MAX_ICON_DOWNLOAD_BYTES {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }
        let (ct, ext) = sniff_image(&bytes)?;
        Some((bytes, ct, ext))
    }

    async fn write_cached_file(&self, file_name: &str, bytes: &[u8]) -> std::io::Result<()> {
        let icons_dir = self.icons_dir.clone();
        let file_name = file_name.to_string();
        let bytes = bytes.to_vec();
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&icons_dir)?;
            let final_path = icons_dir.join(&file_name);
            let tmp_name = format!("{file_name}.{}.tmp", uuid::Uuid::new_v4());
            let tmp_path = icons_dir.join(tmp_name);
            std::fs::write(&tmp_path, &bytes)?;
            std::fs::rename(&tmp_path, &final_path)
        })
        .await
        .map_err(std::io::Error::other)?
    }

    fn allow_local_http(&self) -> bool {
        #[cfg(test)]
        {
            self.allow_loopback_http
        }
        #[cfg(not(test))]
        {
            false
        }
    }
}

fn build_cdn_client(allow_loopback_http: bool) -> anyhow::Result<reqwest::Client> {
    let policy = reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= 3 {
            attempt.stop()
        } else if is_allowed_cdn_url(attempt.url(), allow_loopback_http) {
            attempt.follow()
        } else {
            attempt.stop()
        }
    });
    Ok(crate::adapters::network::client_builder()
        .timeout(Duration::from_secs(8))
        .redirect(policy)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Parallax/0.1")
        .build()?)
}

pub fn zbt_fallback_url(market_hash_name: &str) -> String {
    let encoded = STANDARD.encode(market_hash_name.as_bytes());
    format!("https://img.zbt.com/e/steam/item/730/{encoded}.png")
}

pub fn is_demo_or_fixture_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.starts_with("demo-") || key.starts_with("演示·") || lower.contains("fixture")
}

pub fn normalize_icon_url(raw: &str, allow_loopback_http: bool) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 1000 || trimmed.chars().any(char::is_control) {
        return None;
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let parsed = reqwest::Url::parse(trimmed).ok()?;
        return is_allowed_cdn_url(&parsed, allow_loopback_http).then(|| parsed.into());
    }
    // Steam economy API often returns a bare CDN image path (no scheme or slashes like "..").
    if trimmed.contains("..") || trimmed.contains('?') || trimmed.contains('#') || trimmed.contains(' ') {
        return None;
    }
    let clean = trimmed.trim_start_matches('/');
    if clean.is_empty() {
        return None;
    }
    let url = format!("https://community.fastly.steamstatic.com/economy/image/{clean}/96fx96f");
    let parsed = reqwest::Url::parse(&url).ok()?;
    is_allowed_cdn_url(&parsed, allow_loopback_http).then(|| parsed.into())
}

fn is_allowed_cdn_url(url: &reqwest::Url, allow_loopback_http: bool) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let h = host.to_ascii_lowercase();
    if allow_loopback_http && url.scheme() == "http" && (h == "127.0.0.1" || h == "localhost") {
        return true;
    }
    if url.scheme() != "https" {
        return false;
    }
    matches!(
        h.as_str(),
        "img.zbt.com"
            | "steamcommunity-a.akamaihd.net"
            | "buff.163.com"
            | "g.fp.ps.netease.com"
            | "market.fp.ps.netease.com"
            | "steamcommunity.com"
    ) || h.ends_with(".steamstatic.com")
        || h.ends_with(".zbt.com")
        || h.ends_with(".fp.ps.netease.com")
}

fn safe_single_filename(name: &str) -> Option<&str> {
    let p = Path::new(name);
    let file = p.file_name()?.to_str()?;
    (file == name
        && !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(".."))
    .then_some(file)
}

fn sniff_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some(("image/jpeg", "jpg"))
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else {
        None
    }
}

fn sha256_hex(key: &str) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, key.as_bytes());
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn shard_index(key: &str) -> usize {
    let digest = ring::digest::digest(&ring::digest::SHA256, key.as_bytes());
    (digest.as_ref()[0] as usize) % LOCK_SHARDS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{AccountRepository, InventoryRepository, Repository};
    use crate::domain::{
        accounts::Account,
        inventory::{InventoryRequest, InventorySnapshot},
        selection::{ScanReport, ScanRequest},
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;

    const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89";

    fn test_cache(max_bytes: u64) -> (Arc<ItemIconCache>, Store, PathBuf) {
        let root = std::env::temp_dir().join(format!("parallax-assets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let db_path = root.join("parallax.db");
        let store = Store::open(&db_path).unwrap();
        let icons_dir = root.join("cache").join("icons");
        std::fs::create_dir_all(&icons_dir).unwrap();
        let cache = Arc::new(ItemIconCache {
            store: store.clone(),
            icons_dir,
            db_path,
            max_bytes,
            client: build_cdn_client(true).unwrap(),
            shards: std::array::from_fn(|_| Mutex::new(())),
            allow_loopback_http: true,
        });
        (cache, store, root)
    }

    #[test]
    fn url_allowlist_and_steam_path_normalization_enforced() {
        assert_eq!(
            normalize_icon_url("abc/xyz", false).as_deref(),
            Some("https://community.fastly.steamstatic.com/economy/image/abc/xyz/96fx96f")
        );
        assert!(normalize_icon_url("https://img.zbt.com/e/steam/item/730/x.png", false).is_some());
        assert!(normalize_icon_url("https://community.akamai.steamstatic.com/a.png", false).is_some());
        assert!(normalize_icon_url("http://127.0.0.1/secret.png", false).is_none());
        assert!(normalize_icon_url("https://evil.example/a.png", false).is_none());
        assert!(normalize_icon_url("../../etc/passwd", false).is_none());
        assert!(is_demo_or_fixture_key("demo-0"));
        assert!(is_demo_or_fixture_key("演示·潮汐武器箱"));
    }

    #[tokio::test]
    async fn downloads_caches_serves_offline_and_evicts_lru_over_budget() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = PNG_BYTES.to_vec();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(header.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });

        let (cache, _store, root) = test_cache(PNG_BYTES.len() as u64);
        let url1 = format!("http://127.0.0.1:{}/item1.png", addr.port());
        let first = cache
            .resolve_icon("AK-47 | Slate (Field-Tested)", Some(&url1))
            .await
            .unwrap();
        assert_eq!(first.bytes, PNG_BYTES);
        assert_eq!(first.content_type, "image/png");

        // Second item forces LRU eviction of first item because budget allows 1 icon file.
        let url2 = format!("http://127.0.0.1:{}/item2.png", addr.port());
        let second = cache
            .resolve_icon("AWP | Atheris (Minimal Wear)", Some(&url2))
            .await
            .unwrap();
        assert_eq!(second.bytes, PNG_BYTES);
        server.join().unwrap();

        // Listener is now shut down; second item MUST still be served offline from disk!
        let offline = cache
            .resolve_icon("AWP | Atheris (Minimal Wear)", None)
            .await
            .unwrap();
        assert_eq!(offline.bytes, PNG_BYTES);

        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.icons.cached_files, 1);
        assert_eq!(stats.icons.cached_bytes, PNG_BYTES.len() as u64);

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn cleanup_prunes_old_logs_and_icons_while_preserving_user_data() {
        let (cache, store, root) = test_cache(1024 * 1024);

        // Seed protected account
        store
            .add_account(Account {
                id: "acc-1".into(),
                label: "Main".into(),
                steam_id: "76561198000000000".into(),
                created_at: 100,
            })
            .await
            .unwrap();

        // Seed 3 scans (keep latest 1 completed -> 2 cleanable)
        for i in 1..=3u64 {
            let mut report = ScanReport::new(ScanRequest::default());
            report.id = format!("scan-{i}");
            report.created_at = 100 + i;
            report.status = "completed".into();
            store.save(report).await.unwrap();
        }

        // Seed 2 inventory snapshots for acc-1 (keep latest 1 -> 1 cleanable)
        for i in 1..=2u64 {
            store
                .insert_inventory(InventorySnapshot {
                    id: format!("inv-{i}"),
                    account_id: "acc-1".into(),
                    steam_id: "76561198000000000".into(),
                    credential_generation: "gen1".into(),
                    request: InventoryRequest {
                        max_pages_per_context: 1,
                    },
                    started_at: 200 + i,
                    observed_at: 200 + i,
                    source: "test".into(),
                    platform_verified: false,
                    coverage: vec![],
                    assets: vec![],
                    warnings: vec![],
                })
                .await
                .unwrap();
        }

        // Seed 1 cached icon file
        std::fs::write(cache.icons_dir.join("testicon.png"), PNG_BYTES).unwrap();
        store
            .mark_item_icon_cached(
                "Rio 2022 Dust II Souvenir Package".into(),
                "https://img.zbt.com/e/steam/item/730/x.png".into(),
                "testicon.png".into(),
                "image/png".into(),
                PNG_BYTES.len() as u64,
                300,
            )
            .await
            .unwrap();

        let before = cache.stats().await.unwrap();
        assert_eq!(before.logs.scans_total, 3);
        assert_eq!(before.logs.scans_cleanable, 2);
        assert_eq!(before.logs.snapshots_total, 2);
        assert_eq!(before.logs.snapshots_cleanable, 1);
        assert_eq!(before.icons.cached_files, 1);

        let cleared = cache
            .cleanup(CleanupRequest {
                clear_icons: true,
                clear_logs: true,
                keep_scans: 1,
            })
            .await
            .unwrap();
        assert_eq!(cleared.cleared_icon_files, 1);
        assert_eq!(cleared.deleted_scans, 2);
        assert_eq!(cleared.deleted_snapshots, 1);
        assert_eq!(cleared.stats.icons.cached_files, 0);
        assert_eq!(cleared.stats.logs.scans_total, 1);
        assert_eq!(cleared.stats.logs.snapshots_total, 1);
        assert!(store.account("acc-1".into()).await.unwrap().is_some());

        let _ = std::fs::remove_dir_all(root);
    }
}
