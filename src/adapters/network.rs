//! Process-stable outbound routes. No automatic proxy rotation on authenticated requests.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};
#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub nodes: Vec<ProxyNode>,
    pub routes: BTreeMap<String, String>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProxyNode {
    pub id: String,
    pub url: String,
}
static CONFIG: OnceLock<ProxyConfig> = OnceLock::new();
fn scope(host: &str) -> Option<&'static str> {
    match host {
        "steamcommunity.com"
        | "www.steamcommunity.com"
        | "login.steampowered.com"
        | "store.steampowered.com"
        | "api.steampowered.com" => Some("steam"),
        "buff.163.com" => Some("buff"),
        "www.steamdt.com" | "steamdt.com" => Some("steamdt"),
        "api.frankfurter.app" | "api.frankfurter.dev" => Some("fx"),
        _ => None,
    }
}
impl ProxyConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.nodes.len() <= 20, "代理池至多20个节点");
        let mut ids = std::collections::HashSet::new();
        for n in &self.nodes {
            anyhow::ensure!(
                !n.id.is_empty()
                    && n.id.len() <= 64
                    && n.id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                    && ids.insert(n.id.as_str()),
                "代理节点ID无效或重复"
            );
            let u = reqwest::Url::parse(&n.url).map_err(|_| anyhow::anyhow!("代理URL格式无效"))?;
            anyhow::ensure!(
                matches!(u.scheme(), "http" | "https")
                    && u.host_str().is_some()
                    && u.username().is_empty()
                    && u.password().is_none()
                    && u.query().is_none()
                    && u.fragment().is_none()
                    && u.path() == "/",
                "当前支持无账号密码的HTTP/HTTPS代理URL，不接受路径/查询/片段"
            );
        }
        for (k, v) in &self.routes {
            anyhow::ensure!(
                ["steam", "buff", "steamdt", "fx"].contains(&k.as_str())
                    && (v == "direct" || ids.contains(v.as_str())),
                "代理路由类型或节点引用无效"
            );
        }
        Ok(())
    }
    fn selected(&self, url: &reqwest::Url) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let id = self.routes.get(scope(url.host_str()?)?)?;
        self.nodes
            .iter()
            .find(|n| &n.id == id)
            .map(|n| n.url.clone())
    }
}
static CONFIG_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

fn read_config(path: &std::path::Path) -> anyhow::Result<ProxyConfig> {
    let config = if path.exists() {
        let bytes = std::fs::read(path)
            .map_err(|_| anyhow::anyhow!("无法读取PARALLAX_PROXY_CONFIG文件"))?;
        anyhow::ensure!(bytes.len() <= 65536, "代理配置至多64KiB");
        serde_json::from_slice::<ProxyConfig>(&bytes)
            .map_err(|_| anyhow::anyhow!("代理配置JSON格式无效"))?
    } else {
        ProxyConfig::default()
    };
    config.validate()?;
    Ok(config)
}

pub fn initialize(data_path: &std::path::Path) -> anyhow::Result<()> {
    let path = std::env::var_os("PARALLAX_PROXY_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| data_path.with_extension("proxy.json"));
    if std::env::var_os("PARALLAX_PROXY_CONFIG").is_some() && !path.is_file() {
        anyhow::bail!("无法读取 PARALLAX_PROXY_CONFIG 文件");
    }
    let config = read_config(&path)?;
    CONFIG_PATH
        .set(path)
        .map_err(|_| anyhow::anyhow!("网络配置已初始化"))?;
    CONFIG
        .set(config)
        .map_err(|_| anyhow::anyhow!("网络配置已初始化"))?;
    Ok(())
}
pub fn client_builder() -> reqwest::ClientBuilder {
    CONFIG.get().map_or_else(base_builder, builder_for)
}
fn base_builder() -> reqwest::ClientBuilder {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder()
}
fn builder_for(config: &ProxyConfig) -> reqwest::ClientBuilder {
    let builder = base_builder();
    match config.enabled {
        true => {
            let config = config.clone();
            builder
                .no_proxy()
                .proxy(reqwest::Proxy::custom(move |url| config.selected(url)))
        }
        false => builder,
    }
}
#[derive(Serialize)]
pub struct ProxyView {
    pub mode: &'static str,
    pub nodes: Vec<String>,
    pub routes: BTreeMap<String, String>,
    pub browser_routed: bool,
    pub automatic_rotation: bool,
    pub requires_restart: bool,
}
pub fn view() -> ProxyView {
    let c = CONFIG.get();
    ProxyView {
        mode: if c.is_some_and(|c| c.enabled) {
            "explicit_routes"
        } else {
            "system_environment"
        },
        nodes: c
            .map(|c| c.nodes.iter().map(|n| n.id.clone()).collect())
            .unwrap_or_default(),
        routes: c.map(|c| c.routes.clone()).unwrap_or_default(),
        browser_routed: false,
        automatic_rotation: false,
        requires_restart: true,
    }
}
pub async fn probe(id: &str) -> Result<serde_json::Value, String> {
    let n = CONFIG
        .get()
        .and_then(|c| c.nodes.iter().find(|n| n.id == id))
        .ok_or("代理节点不存在")?;
    let proxy = reqwest::Proxy::all(&n.url).map_err(|_| "代理配置不可用")?;
    let client = base_builder()
        .no_proxy()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "代理测试客户端构建失败")?;
    let started = std::time::Instant::now();
    match client.get("https://steamcommunity.com/").send().await {
        Ok(r) => Ok(
            serde_json::json!({"node_id":id,"outcome":"http_response","http_status":r.status().as_u16(),"latency_ms":started.elapsed().as_millis(),"scope":"unauthenticated_steam_community","identity_verified":false}),
        ),
        Err(_) => Ok(
            serde_json::json!({"node_id":id,"outcome":"connection_failed","latency_ms":started.elapsed().as_millis(),"identity_verified":false}),
        ),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn https_proxy_requests_connect_and_rejection_stays_an_error() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0; 1];
            while request.len() < 8192 && !request.ends_with(b"\r\n\r\n") {
                if socket.read(&mut byte).unwrap() == 0 {
                    break;
                }
                request.push(byte[0]);
            }
            socket
                .write_all(
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            String::from_utf8(request).unwrap()
        });
        let config = ProxyConfig {
            enabled: true,
            nodes: vec![ProxyNode {
                id: "fixture".into(),
                url: format!("http://{address}"),
            }],
            routes: BTreeMap::from([("steam".into(), "fixture".into())]),
        };
        let client = builder_for(&config)
            .timeout(std::time::Duration::from_secs(3))
            .resolve(
                "steamcommunity.com",
                std::net::SocketAddr::from(([127, 0, 0, 1], 1)),
            )
            .build()
            .unwrap();
        assert!(
            client
                .get("https://steamcommunity.com:1/my/")
                .header("cookie", "synthetic-session=not-a-real-secret")
                .send()
                .await
                .is_err()
        );
        let request = server.join().unwrap();
        assert!(request.starts_with("CONNECT steamcommunity.com:1 HTTP/1.1\r\n"));
        assert!(!request.to_ascii_lowercase().contains("cookie:"));
        assert!(!request.contains("synthetic-session"));
        assert!(!request.contains("/my/"));
    }
    #[tokio::test]
    async fn proxy_transport_uses_selected_node_and_never_falls_back_direct() {
        use std::io::{Read, Write};
        use std::net::{SocketAddr, TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 2048];
            let n = socket.read(&mut bytes).unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .unwrap();
            String::from_utf8_lossy(&bytes[..n]).into_owned()
        });
        let mut config = ProxyConfig {
            enabled: true,
            nodes: vec![ProxyNode {
                id: "fixture".into(),
                url: format!("http://{address}"),
            }],
            routes: BTreeMap::from([("steam".into(), "fixture".into())]),
        };
        let client = builder_for(&config)
            .timeout(std::time::Duration::from_secs(2))
            .resolve("steamcommunity.com", SocketAddr::from(([127, 0, 0, 1], 1)))
            .build()
            .unwrap();
        assert_eq!(
            client
                .get("http://steamcommunity.com:1/proxy-fixture")
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap(),
            "ok"
        );
        assert!(
            server
                .join()
                .unwrap()
                .starts_with("GET http://steamcommunity.com:1/proxy-fixture ")
        );
        let direct = TcpListener::bind("127.0.0.1:0").unwrap();
        direct.set_nonblocking(true).unwrap();
        let target = direct.local_addr().unwrap();
        config.nodes[0].url = "http://127.0.0.1:1".into();
        let client = builder_for(&config)
            .timeout(std::time::Duration::from_secs(1))
            .resolve("steamcommunity.com", target)
            .build()
            .unwrap();
        assert!(
            client
                .get(format!("http://steamcommunity.com:{}/", target.port()))
                .send()
                .await
                .is_err()
        );
        assert_eq!(
            direct.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
    #[test]
    fn proxy_routes_are_explicit_sticky_and_unknown_hosts_are_direct() {
        let c:ProxyConfig=serde_json::from_value(serde_json::json!({"enabled":true,"nodes":[{"id":"local","url":"http://127.0.0.1:7890"},{"id":"backup","url":"https://127.0.0.1:7891"}],"routes":{"steam":"local","buff":"direct","steamdt":"backup"}})).unwrap();
        c.validate().unwrap();
        for _ in 0..3 {
            assert_eq!(
                c.selected(&reqwest::Url::parse("https://steamcommunity.com/my/").unwrap())
                    .as_deref(),
                Some("http://127.0.0.1:7890")
            );
        }
        assert!(
            c.selected(&reqwest::Url::parse("https://buff.163.com/").unwrap())
                .is_none()
        );
        assert!(
            c.selected(&reqwest::Url::parse("http://127.0.0.1:38480/").unwrap())
                .is_none()
        );
        for url in [
            "http://user:password@localhost:80",
            "socks5://localhost:1080",
            "http://localhost/path",
            "http://localhost/?token=synthetic",
        ] {
            let invalid = ProxyConfig {
                enabled: true,
                nodes: vec![ProxyNode {
                    id: "x".into(),
                    url: url.into(),
                }],
                routes: BTreeMap::new(),
            };
            assert!(invalid.validate().is_err());
        }
    }
}

pub fn editable_config() -> anyhow::Result<serde_json::Value> {
    let path = CONFIG_PATH
        .get()
        .ok_or_else(|| anyhow::anyhow!("配置未初始化"))?;
    let saved = read_config(path)?;
    let pending_restart = CONFIG.get() != Some(&saved);
    Ok(serde_json::json!({"config": saved, "pending_restart": pending_restart}))
}

pub fn save_config(config: ProxyConfig) -> anyhow::Result<()> {
    config.validate()?;
    let path = CONFIG_PATH
        .get()
        .ok_or_else(|| anyhow::anyhow!("配置未初始化"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(file.as_file_mut(), &config)?;
    anyhow::ensure!(
        file.as_file().metadata()?.len() <= 65536,
        "代理配置至多64KiB"
    );
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
