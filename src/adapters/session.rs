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
fn steam_payload(body: &str, expected: &str) -> Verification {
    let id = body
        .split_once("<steamID64>")
        .and_then(|(_, s)| s.split_once("</steamID64>"))
        .map(|(s, _)| s.trim());
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
            Platform::Steam => "https://steamcommunity.com/my/?xml=1",
            Platform::Buff => {
                "https://buff.163.com/account/api/user/info/v2?meta_list=buy_order_state"
            }
        }
        .to_string();
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
                if next.path().contains("/login") {
                    return result(SessionStatus::Invalid, "平台跳转登录页，登录态已失效", None);
                }
                if platform == Platform::Buff {
                    return result(
                        SessionStatus::VerificationRequired,
                        "BUFF 返回验证跳转，请在浏览器验证",
                        None,
                    );
                }
                next.set_query(Some("xml=1"));
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
                Platform::Steam => steam_payload(body, &account.steam_id),
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
    fn identity_unknown_and_challenge_are_distinct() {
        assert_eq!(
            steam_payload(
                "<steamID64>76561198000000001</steamID64>",
                "76561198000000001"
            )
            .status,
            SessionStatus::Valid
        );
        assert_eq!(
            steam_payload(
                "<steamID64>76561198000000002</steamID64>",
                "76561198000000001"
            )
            .status,
            SessionStatus::Invalid
        );
        assert_eq!(
            steam_payload("<html>maintenance</html>", "76561198000000001").status,
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
