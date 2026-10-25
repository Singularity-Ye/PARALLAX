//! Authenticated GET only. Never forwards cookies across redirects or returns raw bodies.
use super::buff_read::{BuffHttp, payload_error};
#[cfg(test)]
use super::buff_read::{rotation, status_error};
use crate::{
    application::ports::{BuffQuoteProvider, BuffReadError as Error},
    domain::{accounts::Credentials, buff_quotes::*, orderbook::PriceLevel},
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
pub struct BuffSellOrders {
    client: BuffHttp,
}
impl BuffSellOrders {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: BuffHttp::new()?,
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
impl BuffQuoteProvider for BuffSellOrders {
    async fn sell_page(&self, goods_id: u64, secret: Credentials) -> Result<BuffSellPage, Error> {
        let body = self
            .client
            .get(
                "/api/market/goods/sell_order",
                &[
                    ("game", "csgo".into()),
                    ("goods_id", goods_id.to_string()),
                    ("page_num", "1".into()),
                    ("page_size", "80".into()),
                    ("sort_by", "default".into()),
                    ("mode", String::new()),
                    ("allow_tradable_cooldown", "0".into()),
                ],
                &format!("https://buff.163.com/goods/{goods_id}"),
                &secret,
            )
            .await?;
        parse(&body, goods_id)
    }
}
