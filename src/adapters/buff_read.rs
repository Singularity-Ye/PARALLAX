//! Shared BUFF read transport: GET only, no redirects, bounded JSON and redacted failures.
use crate::{application::ports::BuffReadError as Error, domain::accounts::Credentials};
use serde_json::Value;
use std::time::Duration;
pub(super) struct BuffHttp {
    client: reqwest::Client,
}
impl BuffHttp {
    pub fn new() -> anyhow::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
    pub async fn get(
        &self,
        path: &str,
        query: &[(&str, String)],
        referer: &str,
        secret: &Credentials,
    ) -> Result<Value, Error> {
        let mut response = self
            .client
            .get(format!("https://buff.163.com{path}"))
            .query(query)
            .header("Cookie", &secret.cookies)
            .header(
                "User-Agent",
                if secret.user_agent.is_empty() {
                    "Mozilla/5.0 Parallax/0.1"
                } else {
                    &secret.user_agent
                },
            )
            .header("Referer", referer)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|_| Error::Network)?;
        if let Some(error) = status_error(
            response.status().as_u16(),
            response
                .headers()
                .get("location")
                .and_then(|h| h.to_str().ok()),
            response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok()),
        ) {
            return Err(error);
        }
        let changed = rotation(response.headers(), secret);
        let mut raw = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Network)? {
            if raw.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(Error::InvalidPayload);
            }
            raw.extend_from_slice(&chunk);
        }
        let body: Value = serde_json::from_slice(&raw).map_err(|_| {
            let text = String::from_utf8_lossy(&raw).to_lowercase();
            if ["captcha", "geetest", "challenge", "安全验证"]
                .iter()
                .any(|s| text.contains(s))
            {
                Error::Challenge
            } else {
                Error::InvalidPayload
            }
        })?;
        if let Some(error) = payload_error(&body) {
            return Err(error);
        }
        if let Some(error) = changed {
            return Err(error);
        }
        Ok(body)
    }
}
pub(super) fn payload_error(body: &Value) -> Option<Error> {
    let code = body["code"].as_str().unwrap_or("").to_ascii_lowercase();
    if matches!(
        code.as_str(),
        "captcha required"
            | "riskcontrol"
            | "verificationrequired"
            | "risk_control"
            | "verification required"
    ) {
        return Some(Error::Challenge);
    }
    if matches!(
        code.as_str(),
        "login required" | "loginrequired" | "login_required" | "unauthorized" | "not logged in"
    ) {
        return Some(Error::Expired);
    }
    if matches!(
        code.as_str(),
        "rate limit" | "rate_limit" | "too many requests" | "ratelimited"
    ) {
        return Some(Error::RateLimited(300));
    }
    if code != "ok" {
        let message = body["error"]
            .as_str()
            .or_else(|| body["message"].as_str())
            .or_else(|| body["msg"].as_str())
            .unwrap_or("")
            .to_lowercase();
        if ["captcha", "challenge", "安全验证", "人机验证"]
            .iter()
            .any(|s| message.contains(s))
        {
            return Some(Error::Challenge);
        }
        if ["请求过于频繁", "rate limit", "too many requests"]
            .iter()
            .any(|s| message.contains(s))
        {
            return Some(Error::RateLimited(300));
        }
        return Some(Error::Rejected);
    }
    None
}
pub(super) fn status_error(
    status: u16,
    location: Option<&str>,
    retry: Option<&str>,
) -> Option<Error> {
    match status {
        403 | 412 => Some(Error::Challenge),
        401 => Some(Error::Expired),
        429 => Some(Error::RateLimited(
            retry
                .and_then(|r| r.parse::<u64>().ok())
                .unwrap_or(300)
                .clamp(300, 604800),
        )),
        300..=399 => Some(
            if location.is_some_and(|s| s.contains("/login") || s.contains("passport")) {
                Error::Expired
            } else {
                Error::Challenge
            },
        ),
        200..=299 => None,
        _ => Some(Error::Rejected),
    }
}
pub(super) fn rotation(
    headers: &reqwest::header::HeaderMap,
    secret: &Credentials,
) -> Option<Error> {
    for header in headers.get_all("set-cookie") {
        let Ok(value) = header.to_str() else {
            return Some(Error::CredentialsChanged);
        };
        if let Some((name, value)) = value.split(';').next().unwrap_or("").split_once('=')
            && matches!(name.trim(), "session" | "csrf_token")
            && secret.cookie(name.trim()) != Some(value.trim())
        {
            return Some(if name.trim() == "session" && value.trim().is_empty() {
                Error::Expired
            } else {
                Error::CredentialsChanged
            });
        }
    }
    None
}
