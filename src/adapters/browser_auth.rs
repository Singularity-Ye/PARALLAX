//! Local interactive browser adapter. No attachment to existing profiles, password
//! automation, raw CDP errors, credential logging or automatic browser downloads.
use crate::{application::ports::BrowserAuth, domain::accounts::*};
use headless_chrome::{Browser, LaunchOptions, Tab};
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf, sync::Arc, time::Duration};

pub struct LocalBrowser;
pub struct LoginWindow {
    browser: Browser,
    tab: Arc<Tab>,
}

fn browser_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PARALLAX_BROWSER") {
        return Some(path.into());
    }
    // Edge is present on the primary Windows host. Explicit discovery avoids
    // installing another browser; all launches still use a fresh temp profile.
    ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"]
        .into_iter()
        .filter_map(std::env::var_os)
        .flat_map(|base| {
            [
                "Google/Chrome/Application/chrome.exe",
                "Microsoft/Edge/Application/msedge.exe",
            ]
            .map(|suffix| PathBuf::from(&base).join(suffix))
        })
        .find(|p| p.is_file())
}
fn origin(platform: Platform) -> &'static str {
    match platform {
        Platform::Steam => "https://steamcommunity.com/",
        Platform::Buff => "https://buff.163.com/",
    }
}
fn safe_page(url: &str, platform: Platform) -> bool {
    let Ok(url) = reqwest::Url::parse(url) else {
        return false;
    };
    let expected = reqwest::Url::parse(origin(platform)).unwrap();
    url.scheme() == "https"
        && url.host_str() == expected.host_str()
        && url.port_or_known_default() == Some(443)
        && url.username().is_empty()
        && url.password().is_none()
}
// Request only the cookies required by this application's platform adapters.
// Reject duplicate paths instead of silently choosing an unrelated session.
fn cookie_header(
    platform: Platform,
    cookies: impl IntoIterator<Item = (String, String)>,
) -> Result<String, &'static str> {
    let allowed: &[&str] = match platform {
        Platform::Steam => &["steamLoginSecure", "sessionid"],
        Platform::Buff => &["session", "csrf_token"],
    };
    let mut selected = BTreeMap::new();
    for (name, value) in cookies {
        if allowed.contains(&name.as_str()) && selected.insert(name, value).is_some() {
            return Err("登录 Cookie 存在多个同名值，请在独立窗口重新登录");
        }
    }
    let required = if platform == Platform::Steam {
        "steamLoginSecure"
    } else {
        "session"
    };
    if selected.get(required).is_none_or(String::is_empty) {
        return Err("尚未发现登录 Cookie，请在独立窗口完成登录后重试");
    }
    Ok(selected
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; "))
}
fn launch_at(
    path: Option<PathBuf>,
    headless: bool,
    url: &str,
) -> Result<LoginWindow, &'static str> {
    let options = LaunchOptions::default_builder()
        .path(path)
        .headless(headless)
        .sandbox(true)
        .enable_logging(false)
        .ignore_certificate_errors(false)
        .idle_browser_timeout(Duration::from_secs(660))
        .args(vec![OsStr::new("--remote-debugging-address=127.0.0.1")])
        .build()
        .map_err(|_| "浏览器启动配置无效")?;
    let browser = Browser::new(options).map_err(
        |_| "无法启动本地 Chrome/Edge；请安装浏览器或设置 PARALLAX_BROWSER 可执行文件路径",
    )?;
    browser.set_default_timeout(Duration::from_secs(20));
    let tab = browser.new_tab().map_err(|_| "无法创建独立登录页")?;
    tab.navigate_to(url)
        .map_err(|_| "登录页打开失败，请检查网络后重新发起")?;
    Ok(LoginWindow { browser, tab })
}
impl BrowserAuth for LocalBrowser {
    type Window = LoginWindow;
    async fn launch(&self, platform: Platform) -> Result<LoginWindow, &'static str> {
        tokio::task::spawn_blocking(move || {
            let url = match platform {
                Platform::Steam => "https://steamcommunity.com/login/home/",
                Platform::Buff => origin(platform),
            };
            launch_at(browser_path(), false, url)
        })
        .await
        .map_err(|_| "浏览器启动任务中断")?
    }
    async fn capture(
        &self,
        window: &LoginWindow,
        platform: Platform,
    ) -> Result<Credentials, &'static str> {
        let tab = window.tab.clone();
        let browser = window.browser.clone();
        tokio::task::spawn_blocking(move || {
            let ua = browser
                .get_version()
                .map_err(|_| "登录窗口已关闭，请取消后重试")?
                .user_agent;
            if !safe_page(&tab.get_url(), platform) {
                return Err("请在原登录页完成登录，并返回对应 Steam 社区或 BUFF 网站后重试");
            }
            let cookies = tab
                .get_cookies()
                .map_err(|_| "无法读取独立登录窗口，请确认窗口仍打开")?;
            // Recheck URL after capture to avoid forwarding a different site's
            // same-named cookies if the user navigated during the CDP request.
            if !safe_page(&tab.get_url(), platform) {
                return Err("登录页面已变化，请返回对应平台后重试");
            }
            let host = reqwest::Url::parse(origin(platform))
                .unwrap()
                .host_str()
                .unwrap()
                .to_string();
            let cookies = cookies
                .into_iter()
                .filter(|c| {
                    let domain = c.domain.trim_start_matches('.');
                    host == domain || host.ends_with(&format!(".{domain}"))
                })
                .map(|c| (c.name, c.value));
            Ok(Credentials {
                cookies: cookie_header(platform, cookies)?,
                user_agent: ua,
            })
        })
        .await
        .map_err(|_| "浏览器读取任务中断")?
    }
    async fn close(&self, window: LoginWindow) -> Result<(), &'static str> {
        tokio::task::spawn_blocking(move || drop(window))
            .await
            .map_err(|_| "浏览器关闭任务中断")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_scope_and_cookie_selection() {
        assert!(safe_page(
            "https://steamcommunity.com/market/",
            Platform::Steam
        ));
        for bad in [
            "http://steamcommunity.com/",
            "https://steamcommunity.com.evil.test/",
            "https://steamcommunity.com:8443/",
            "https://user@steamcommunity.com/",
            "https://store.steampowered.com/",
        ] {
            assert!(!safe_page(bad, Platform::Steam));
        }
        assert!(!safe_page("https://steamcommunity.com/", Platform::Buff));
        let values = vec![
            ("session".into(), "synthetic".into()),
            ("unrelated".into(), "excluded".into()),
        ];
        assert_eq!(
            cookie_header(Platform::Buff, values).unwrap(),
            "session=synthetic"
        );
        assert!(
            cookie_header(
                Platform::Buff,
                vec![
                    ("session".into(), "a".into()),
                    ("session".into(), "b".into())
                ]
            )
            .is_err()
        );
        assert!(cookie_header(Platform::Steam, Vec::new()).is_err());
    }
    #[tokio::test]
    #[ignore = "launches only an isolated headless browser on about:blank; no platform or user-profile access"]
    async fn isolated_browser_smoke() {
        let window = tokio::task::spawn_blocking(|| launch_at(browser_path(), true, "about:blank"))
            .await
            .unwrap()
            .unwrap();
        assert!(window.tab.get_cookies().unwrap().is_empty());
        assert!(
            LocalBrowser
                .capture(&window, Platform::Steam)
                .await
                .is_err()
        );
        LocalBrowser.close(window).await.unwrap();
    }
}
