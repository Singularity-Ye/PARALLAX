use crate::{
    application::ports::{HistoryProvider, SteamReadError as E},
    domain::{accounts::Credentials, history::*},
};
use chrono::{FixedOffset, NaiveDateTime, TimeZone};
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};

pub struct SteamHistory {
    client: reqwest::Client,
}
impl SteamHistory {
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
fn timestamp(raw: &str) -> Result<u64, E> {
    if raw.len() > 60 {
        return Err(E::InvalidPayload);
    }
    let (stamp, offset) = raw.trim().split_once(':').ok_or(E::InvalidPayload)?;
    let offset = offset.trim();
    let (sign, digits) = match offset.as_bytes().first() {
        Some(b'+') => (1, &offset[1..]),
        Some(b'-') => (-1, &offset[1..]),
        _ => return Err(E::InvalidPayload),
    };
    if digits.is_empty() || digits.len() > 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(E::InvalidPayload);
    }
    let n: i32 = digits.parse().map_err(|_| E::InvalidPayload)?;
    let (h, m) = if digits.len() <= 2 {
        (n, 0)
    } else {
        (n / 100, n % 100)
    };
    if h > 14 || m > 59 || (h == 14 && m != 0) {
        return Err(E::InvalidPayload);
    }
    let date =
        NaiveDateTime::parse_from_str(&format!("{}:00:00", stamp.trim()), "%b %d %Y %H:%M:%S")
            .map_err(|_| E::InvalidPayload)?;
    let offset = FixedOffset::east_opt(sign * (h * 3600 + m * 60)).ok_or(E::InvalidPayload)?;
    u64::try_from(
        offset
            .from_local_datetime(&date)
            .single()
            .ok_or(E::InvalidPayload)?
            .timestamp(),
    )
    .map_err(|_| E::InvalidPayload)
}
fn price(v: &Value) -> Result<u64, E> {
    let text = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err(E::InvalidPayload),
    };
    if text.is_empty() || text.len() > 24 {
        return Err(E::InvalidPayload);
    }
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text.as_str(), None), |(a, b)| (a, Some(b)));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction
            .is_some_and(|s| s.is_empty() || s.len() > 4 || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(E::InvalidPayload);
    }
    let whole: u64 = whole.parse().map_err(|_| E::InvalidPayload)?;
    let frac = if let Some(f) = fraction {
        f.parse::<u64>().map_err(|_| E::InvalidPayload)? * 10u64.pow(4 - f.len() as u32)
    } else {
        0
    };
    whole
        .checked_mul(10000)
        .and_then(|n| n.checked_add(frac))
        .filter(|n| *n > 0 && *n <= 10_000_000_000)
        .ok_or(E::InvalidPayload)
}
fn volume(v: Option<&Value>) -> Result<Option<u64>, E> {
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let n = v
        .as_u64()
        .or_else(|| {
            v.as_str()
                .filter(|s| {
                    !s.is_empty() && s.len() <= 10 && s.bytes().all(|b| b.is_ascii_digit())
                })?
                .parse()
                .ok()
        })
        .filter(|n| *n <= 1_000_000_000)
        .ok_or(E::InvalidPayload)?;
    Ok(Some(n))
}
fn currency(data: &Value) -> Result<Option<crate::domain::money::Currency>, E> {
    let prefix = data
        .get("price_prefix")
        .map(|v| {
            v.as_str()
                .filter(|s| s.len() <= 32 && !s.chars().any(char::is_control))
                .ok_or(E::InvalidPayload)
        })
        .transpose()?
        .unwrap_or("")
        .trim();
    let suffix = data
        .get("price_suffix")
        .map(|v| {
            v.as_str()
                .filter(|s| s.len() <= 32 && !s.chars().any(char::is_control))
                .ok_or(E::InvalidPayload)
        })
        .transpose()?
        .unwrap_or("")
        .trim();
    let code = match (prefix, suffix) {
        ("CN¥", "") | ("CNY", "") | ("" | "¥" | "CN¥", "CNY") => Some("CNY"),
        ("US$" | "USD", "") | ("" | "$" | "US$", "USD") => Some("USD"),
        ("€" | "EUR", "") | ("" | "€", "EUR") => Some("EUR"),
        ("£" | "GBP", "") | ("" | "£", "GBP") => Some("GBP"),
        ("HK$" | "HKD", "") | ("" | "HK$", "HKD") => Some("HKD"),
        ("NT$" | "TWD", "") | ("" | "NT$", "TWD") => Some("TWD"),
        ("UAH" | "₴" | "грн" | "грн.", "")
        | ("", "UAH" | "₴" | "грн" | "грн.")
        | ("₴" | "грн" | "грн.", "UAH") => Some("UAH"),
        _ => None,
    };
    code.map(|s| s.to_string().try_into().map_err(|_| E::InvalidPayload))
        .transpose()
}
fn parse(body: &[u8], request: &HistoryRequest, at: u64) -> Result<HistorySeries, E> {
    let data: Value = serde_json::from_slice(body).map_err(|_| E::InvalidPayload)?;
    if data["success"] != true && data["success"] != 1 {
        return Err(E::Rejected);
    }
    let raw = data["prices"]
        .as_array()
        .filter(|p| p.len() <= 100000)
        .ok_or(E::InvalidPayload)?;
    let currency = currency(&data)?;
    let today = at / 86400;
    let from = today.saturating_sub(u64::from(request.window_days));
    let mut points = BTreeMap::new();
    let mut before = 0;
    let mut open = 0;
    let mut seen = std::collections::HashSet::new();
    for row in raw {
        let row = row
            .as_array()
            .filter(|r| (2..=3).contains(&r.len()))
            .ok_or(E::InvalidPayload)?;
        let time = timestamp(row[0].as_str().ok_or(E::InvalidPayload)?)?;
        let price = price(&row[1])?;
        let vol = volume(row.get(2))?;
        if time > at || !seen.insert(time) {
            return Err(E::InvalidPayload);
        }
        if time / 86400 < from {
            before += 1;
            continue;
        }
        if time / 86400 >= today {
            open += 1;
            continue;
        }
        points.insert(
            time,
            HistorySample {
                at: time,
                price_major_1e4: price,
                volume: vol,
            },
        );
    }
    let series = HistorySeries {
        currency_basis: if currency.is_some() {
            "explicit_code_or_unique_marker"
        } else {
            "ambiguous_or_unsupported"
        }
        .into(),
        currency,
        points: points.into_values().collect(),
        response_rows: raw.len() as u32,
        excluded_before_window: before,
        excluded_open_day: open,
        from_utc_day: from,
        until_utc_day_exclusive: today,
    };
    series
        .validate(request, at)
        .map_err(|_| E::InvalidPayload)?;
    Ok(series)
}
impl HistoryProvider for SteamHistory {
    async fn history(
        &self,
        request: &HistoryRequest,
        secret: &Credentials,
        at: u64,
    ) -> Result<HistorySeries, E> {
        request.validate().map_err(|_| E::InvalidPayload)?;
        let mut response = self
            .client
            .get("https://steamcommunity.com/market/pricehistory/")
            .query(&[
                ("appid", "730"),
                ("market_hash_name", request.item_key.as_str()),
                ("l", "english"),
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
            .send()
            .await
            .map_err(|_| E::Network)?;
        super::steam_read::classify(response.status(), response.headers(), secret)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| E::Network)? {
            if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
                return Err(E::InvalidPayload);
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(&bytes, request, at)
    }
}

#[cfg(test)]
#[path = "steam_history_tests.rs"]
mod tests;
