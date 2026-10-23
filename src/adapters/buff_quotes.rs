//! Authenticated GET only. Never forwards cookies across redirects or returns raw bodies.
use crate::{
    application::ports::{BuffQuoteProvider, BuffReadError as Error},
    domain::{accounts::Credentials, buff_quotes::*, orderbook::PriceLevel},
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

pub struct BuffSellOrders {
    client: reqwest::Client,
}
impl BuffSellOrders {
    pub fn new() -> anyhow::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
}
fn cents(value: &Value) -> Result<u64, Error> {
    let owned;
    let s = if let Some(s) = value.as_str() {
        s
    } else if let Some(n) = value.as_u64() {
        owned = n.to_string();
        &owned
    } else {
        return Err(Error::InvalidPayload);
    };
    let mut parts = s.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or("");
    if whole.is_empty()
        || s.ends_with('.')
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || parts.next().is_some()
    {
        return Err(Error::InvalidPayload);
    }
    let whole: u64 = whole.parse().map_err(|_| Error::InvalidPayload)?;
    let fraction = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<u64>().map_err(|_| Error::InvalidPayload)? * 10,
        _ => fraction.parse().map_err(|_| Error::InvalidPayload)?,
    };
    whole
        .checked_mul(100)
        .and_then(|v| v.checked_add(fraction))
        .filter(|v| *v > 0 && *v <= 100_000_000)
        .ok_or(Error::InvalidPayload)
}

#[cfg(test)]
#[path = "buff_quotes_tests.rs"]
mod tests;
fn payload_error(body: &Value) -> Option<Error> {
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
fn parse(body: &Value, goods_id: u64) -> Result<BuffSellPage, Error> {
    if let Some(error) = payload_error(body) {
        return Err(error);
    }
    let data = &body["data"];
    if data
        .get("currency")
        .is_some_and(|c| c.as_str() != Some("CNY"))
    {
        return Err(Error::InvalidPayload);
    }
    if data.get("page_num").is_some_and(|p| p.as_u64() != Some(1)) {
        return Err(Error::InvalidPayload);
    }
    let goods = &data["goods_infos"][goods_id.to_string()];
    let name = goods["market_hash_name"]
        .as_str()
        .filter(|n| !n.is_empty() && n.len() <= 500)
        .ok_or(Error::IdentityMismatch)?;
    let items = data["items"].as_array().ok_or(Error::InvalidPayload)?;
    if items.len() > 80 {
        return Err(Error::InvalidPayload);
    }
    let mut seen = HashMap::new();
    let mut levels = BTreeMap::<u64, u64>::new();
    for item in items {
        if item
            .get("goods_id")
            .is_some_and(|id| id.as_u64() != Some(goods_id))
        {
            return Err(Error::IdentityMismatch);
        }
        if item.get("amount").is_some_and(|q| q.as_u64() != Some(1)) {
            return Err(Error::InvalidPayload);
        }
        let id = item["id"]
            .as_str()
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 96
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            })
            .ok_or(Error::InvalidPayload)?;
        let price = cents(&item["price"])?;
        if let Some(previous) = seen.insert(id, price) {
            if previous != price {
                return Err(Error::InvalidPayload);
            }
            continue;
        }
        *levels.entry(price).or_default() += 1;
    }
    let reported_total = data
        .get("total_count")
        .map(|v| v.as_u64().ok_or(Error::InvalidPayload))
        .transpose()?;
    let page = BuffSellPage {
        goods_id,
        market_hash_name: name.into(),
        levels: levels
            .into_iter()
            .map(|(price_minor, quantity)| PriceLevel {
                price_minor,
                quantity,
            })
            .collect(),
        reported_total,
    };
    page.validate(goods_id, name)
        .map_err(|_| Error::InvalidPayload)?;
    Ok(page)
}
fn status_error(status: u16, location: Option<&str>, retry: Option<&str>) -> Option<Error> {
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
fn rotation(headers: &reqwest::header::HeaderMap, secret: &Credentials) -> Option<Error> {
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
impl BuffQuoteProvider for BuffSellOrders {
    async fn sell_page(&self, goods_id: u64, secret: Credentials) -> Result<BuffSellPage, Error> {
        let mut response = self
            .client
            .get("https://buff.163.com/api/market/goods/sell_order")
            .query(&[
                ("game", "csgo"),
                ("goods_id", &goods_id.to_string()),
                ("page_num", "1"),
                ("page_size", "80"),
                ("sort_by", "default"),
                ("mode", ""),
                ("allow_tradable_cooldown", "0"),
            ])
            .header("Cookie", &secret.cookies)
            .header(
                "User-Agent",
                if secret.user_agent.is_empty() {
                    "Mozilla/5.0 Parallax/0.1"
                } else {
                    &secret.user_agent
                },
            )
            .header("Referer", format!("https://buff.163.com/goods/{goods_id}"))
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
        let changed = rotation(response.headers(), &secret);
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
        parse(&body, goods_id)
    }
}
