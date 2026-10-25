//! Read-only probes. Platform response text and request errors never leave this adapter.
use crate::{application::ports::SessionVerifier, domain::accounts::*};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::time::Duration;

pub struct OnlineVerifier {
    client: Client,
}
impl OnlineVerifier {
    pub fn new() -> anyhow::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
}
fn result(status: SessionStatus, message: &'static str, subject: Option<String>) -> Verification {
    Verification {
        status,
        message,
        subject,
        retry_secs: 30,
    }
}
fn unknown() -> Verification {
    result(
        SessionStatus::Unknown,
        "网络或平台响应异常，无法确认登录态",
        None,
    )
}
fn classify(status: StatusCode, retry_after: Option<&str>) -> Option<Verification> {
    match status {
        StatusCode::UNAUTHORIZED => Some(result(
            SessionStatus::Invalid,
            "平台返回未登录或会话失效",
            None,
        )),
        StatusCode::FORBIDDEN => Some(result(
            SessionStatus::VerificationRequired,
            "平台拒绝访问，需要浏览器验证",
            None,
        )),
        StatusCode::TOO_MANY_REQUESTS => {
            let mut out = result(
                SessionStatus::RateLimited,
                "平台限流，请等待冷却后再验证",
                None,
            );
            out.retry_secs = retry_after
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(300)
                .max(300);
            // Bound an untrusted Retry-After before timestamp arithmetic.
            out.retry_secs = out.retry_secs.min(7 * 24 * 3600);
            Some(out)
        }
        s if !s.is_success() && !s.is_redirection() => Some(unknown()),
        _ => None,
    }
}
// /my/ is the authenticated route. A redirect from it to a numeric profile
// identifies the signed-in account; fetching someone's public XML does not.
fn steam_my_redirect(
    from: &reqwest::Url,
    next: &reqwest::Url,
    expected: &str,
) -> Option<Verification> {
    if from.scheme() != "https"
        || from.host_str() != Some("steamcommunity.com")
        || next.scheme() != "https"
        || next.host_str() != Some("steamcommunity.com")
        || next.port_or_known_default() != Some(443)
        || !next.username().is_empty()
        || next.password().is_some()
        || from.path().trim_end_matches('/') != "/my"
    {
        return None;
    }
    let parts: Vec<_> = next.path().trim_matches('/').split('/').collect();
    match parts.as_slice() {
        ["profiles", id] if id.len() == 17 && id.bytes().all(|b| b.is_ascii_digit()) => {
            Some(if *id == expected {
                result(
                    SessionStatus::Valid,
                    "Steam 在线身份验证通过",
                    Some(expected.into()),
                )
            } else {
                result(SessionStatus::Invalid, "在线 Steam 身份与账户不匹配", None)
            })
        }
        _ => None,
    }
}
fn steam_profile_path(url: &reqwest::Url) -> bool {
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    matches!(parts.as_slice(), ["profiles" | "id", id] if !id.is_empty())
}
// Public XML only identifies an account after the authenticated /my/ route
// has resolved that account's vanity profile. It is never login proof alone.
fn steam_profile_payload(body: &str, expected: &str, authenticated_profile: bool) -> Verification {
    if !authenticated_profile {
        return unknown();
    }
    let id = body
        .split_once("<steamID64>")
        .and_then(|(_, s)| s.split_once("</steamID64>"))
        .map(|(id, _)| id.trim());
    match id {
        Some(id) if id == expected => result(
            SessionStatus::Valid,
            "Steam 在线身份验证通过",
            Some(id.into()),
        ),
        Some(_) => result(SessionStatus::Invalid, "在线 Steam 身份与账户不匹配", None),
        None => unknown(),
    }
}
fn buff_payload(body: &str, expected: &str) -> Verification {
    let Ok(data) = serde_json::from_str::<Value>(body) else {
        return unknown();
    };
    let code = data["code"].as_str().unwrap_or("");
    if code == "OK" {
        let user = &data["data"]["user_info"];
        let linked_steam = user["steamid"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| user["steamid"].as_u64().map(|id| id.to_string()));
        match linked_steam.as_deref() {
            Some(id) if id != expected => {
                return result(
                    SessionStatus::Invalid,
                    "BUFF 绑定的 Steam 身份与账户不匹配",
                    None,
                );
            }
            None => return unknown(),
            _ => {}
        }
        let id = user["user_id"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| user["id"].as_str().map(str::to_owned))
            .or_else(|| {
                user["user_id"]
                    .as_u64()
                    .or_else(|| user["id"].as_u64())
                    .map(|v| v.to_string())
            });
        if let Some(id) = id.filter(|s| {
            !s.is_empty()
                && s.len() <= 80
                && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }) {
            return result(SessionStatus::Valid, "BUFF 在线身份验证通过", Some(id));
        }
    }
    match code {
        "Login Required" | "Unauthorized" | "LoginRequired" => {
            result(SessionStatus::Invalid, "BUFF 登录态已失效", None)
        }
        "Captcha Required" | "RiskControl" | "VerificationRequired" => result(
            SessionStatus::VerificationRequired,
            "BUFF 要求浏览器验证",
            None,
        ),
        _ => unknown(),
    }
}
impl SessionVerifier for OnlineVerifier {
    async fn verify(
        &self,
        account: &Account,
        platform: Platform,
        secret: Credentials,
    ) -> Verification {
        let mut url = match platform {
            Platform::Steam => "https://steamcommunity.com/my/",
            Platform::Buff => {
                "https://buff.163.com/account/api/user/info/v2?meta_list=buy_order_state"
            }
        }
        .to_string();
        let mut steam_profile_resolved = false;
        // Explicit same-host redirects; never forward Cookie to an arbitrary Location.
        for _ in 0..4 {
            let ua = if secret.user_agent.is_empty() {
                "Mozilla/5.0 Parallax/0.1"
            } else {
                &secret.user_agent
            };
            let response = self
                .client
                .get(&url)
                .header("Cookie", &secret.cookies)
                .header("User-Agent", ua)
                .send()
                .await;
            let Ok(mut response) = response else {
                return unknown();
            };
            if let Some(out) = classify(
                response.status(),
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok()),
            ) {
                return out;
            }
            if response.status().is_redirection() {
                let next = response
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| response.url().join(s).ok());
                let Some(mut next) = next else {
                    return unknown();
                };
                let host = if platform == Platform::Steam {
                    "steamcommunity.com"
                } else {
                    "buff.163.com"
                };
                if next.scheme() != "https"
                    || next.host_str() != Some(host)
                    || next.port_or_known_default() != Some(443)
                    || !next.username().is_empty()
                    || next.password().is_some()
                {
                    return unknown();
                }
                if next.path().trim_end_matches('/') == "/login"
                    || next.path().starts_with("/login/")
                {
                    return if platform == Platform::Steam && steam_profile_resolved {
                        result(
                            SessionStatus::Unknown,
                            "Steam 身份路由发生登录跳转，无法确认会话；请在独立窗口检查 /my/",
                            None,
                        )
                    } else {
                        result(
                            SessionStatus::Invalid,
                            "平台返回登录页，程序尚未确认有效登录态",
                            None,
                        )
                    };
                }
                if platform == Platform::Buff {
                    return result(
                        SessionStatus::VerificationRequired,
                        "BUFF 返回验证跳转，请在浏览器验证",
                        None,
                    );
                }
                if let Some(identity) = steam_my_redirect(response.url(), &next, &account.steam_id)
                {
                    return identity;
                }
                if response.url().path().trim_end_matches('/') == "/my" && steam_profile_path(&next)
                {
                    steam_profile_resolved = true;
                    next.set_query(Some("xml=1"));
                }
                url = next.to_string();
                continue;
            }
            // Bound the untrusted response rather than buffering an arbitrary page.
            let mut bytes = Vec::new();
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) if bytes.len() + chunk.len() <= 512 * 1024 => {
                        bytes.extend_from_slice(&chunk)
                    }
                    Ok(Some(_)) | Err(_) => return unknown(),
                    Ok(None) => break,
                }
            }
            let Ok(body) = std::str::from_utf8(&bytes) else {
                return unknown();
            };
            return match platform {
                Platform::Steam => {
                    steam_profile_payload(body, &account.steam_id, steam_profile_resolved)
                }
                Platform::Buff => buff_payload(body, &account.steam_id),
            };
        }
        unknown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steam_my_redirect_verifies_exact_numeric_identity_only_on_the_authenticated_route() {
        let from = reqwest::Url::parse("https://steamcommunity.com/my/").unwrap();
        let own =
            reqwest::Url::parse("https://steamcommunity.com/profiles/76561198000000001/").unwrap();
        assert_eq!(
            steam_my_redirect(&from, &own, "76561198000000001")
                .unwrap()
                .status,
            SessionStatus::Valid
        );
        assert_eq!(
            steam_my_redirect(&from, &own, "76561198000000002")
                .unwrap()
                .status,
            SessionStatus::Invalid
        );
        assert!(
            steam_my_redirect(&own, &own, "76561198000000001").is_none(),
            "a public profile visit is not authentication"
        );
        for bad in [
            "https://evil.test/profiles/76561198000000001/",
            "http://steamcommunity.com/profiles/76561198000000001/",
            "https://user@steamcommunity.com/profiles/76561198000000001/",
            "https://steamcommunity.com:8443/profiles/76561198000000001/",
            "https://steamcommunity.com/profiles/76561198000000001/inventory/",
        ] {
            assert!(
                steam_my_redirect(
                    &from,
                    &reqwest::Url::parse(bad).unwrap(),
                    "76561198000000001"
                )
                .is_none()
            );
        }
    }
    #[test]
    fn steam_vanity_identity_requires_a_prior_authenticated_my_redirect() {
        let from = reqwest::Url::parse("https://steamcommunity.com/my/").unwrap();
        let vanity =
            reqwest::Url::parse("https://steamcommunity.com/id/login-is-a-valid-vanity/").unwrap();
        assert!(steam_profile_path(&vanity));
        assert!(steam_my_redirect(&from, &vanity, "76561198000000001").is_none());
        let public_xml = "<profile><steamID64>76561198000000001</steamID64></profile>";
        assert_eq!(
            steam_profile_payload(public_xml, "76561198000000001", false).status,
            SessionStatus::Unknown
        );
        assert_eq!(
            steam_profile_payload(public_xml, "76561198000000001", true).status,
            SessionStatus::Valid
        );
        assert_eq!(
            steam_profile_payload(public_xml, "76561198000000002", true).status,
            SessionStatus::Invalid
        );
        assert_eq!(
            steam_profile_payload(
                "<html>login or redirect loop</html>",
                "76561198000000001",
                true
            )
            .status,
            SessionStatus::Unknown
        );
    }
    #[test]
    fn identity_unknown_and_challenge_are_distinct() {
        assert_eq!(
            steam_profile_payload(
                "<steamID64>76561198000000001</steamID64>",
                "76561198000000001",
                true
            )
            .status,
            SessionStatus::Valid
        );
        assert_eq!(
            steam_profile_payload(
                "<steamID64>76561198000000002</steamID64>",
                "76561198000000001",
                true
            )
            .status,
            SessionStatus::Invalid
        );
        assert_eq!(
            steam_profile_payload("<html>maintenance</html>", "76561198000000001", true).status,
            SessionStatus::Unknown
        );
        assert_eq!(
            buff_payload(r#"{"code":"OK","data":{"user_info":{"user_id":"U123","steamid":"76561198000000001"}}}"#, "76561198000000001").status,
            SessionStatus::Valid
        );
        assert_eq!(
            buff_payload(r#"{"code":"OK","data":{}}"#, "76561198000000001").status,
            SessionStatus::Unknown
        );
        assert_eq!(
            classify(StatusCode::FORBIDDEN, None).unwrap().status,
            SessionStatus::VerificationRequired
        );
        assert!(
            classify(StatusCode::TOO_MANY_REQUESTS, Some("999999999999999"))
                .unwrap()
                .retry_secs
                <= 604800
        );
    }
}
