//! Original queryAction protocol; native currency is authoritative, query currency isn't.
use crate::{
    application::ports::{MarketReadError as Error, OrderBookProvider},
    domain::{orderbook::*, selection::now},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use tokio::sync::Mutex;

const STEAM_CURRENCIES: [(u64, &str); 7] = [
    (1, "USD"),
    (2, "GBP"),
    (3, "EUR"),
    (18, "UAH"),
    (23, "CNY"),
    (29, "HKD"),
    (30, "TWD"),
];
fn request_url(request: &OrderBookRequest) -> Result<reqwest::Url, Error> {
    request.validate().map_err(|_| Error::InvalidPayload)?;
    let code = String::from(request.currency.clone());
    let id = STEAM_CURRENCIES
        .iter()
        .find(|(_, name)| *name == code)
        .ok_or(Error::UnsupportedCurrency)?
        .0;
    let mut url = reqwest::Url::parse("https://steamcommunity.com/market/orderbook").unwrap();
    url.query_pairs_mut()
        .append_pair("q", "Load")
        .append_pair(
            "qp",
            &json!([request.app_id, request.market_hash_name]).to_string(),
        )
        .append_pair("currency", &id.to_string())
        .append_pair("l", "english");
    Ok(url)
}

pub struct SteamOrderBook {
    client: reqwest::Client,
    retry_at: Mutex<u64>,
}
impl SteamOrderBook {
    pub fn new() -> anyhow::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("Mozilla/5.0 Parallax/0.1")
                .build()?,
            retry_at: Mutex::new(0),
        })
    }
}
fn uint(value: &Value) -> Result<u64, Error> {
    value
        .as_u64()
        .or_else(|| {
            let s = value.as_str()?;
            (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                .then(|| s.parse().ok())
                .flatten()
        })
        .ok_or(Error::InvalidPayload)
}
fn levels(raw: &Value, asks: bool) -> Result<Vec<PriceLevel>, Error> {
    let raw = raw.as_array().ok_or(Error::InvalidPayload)?;
    if raw.len() % 2 != 0 || raw.len() > 8192 {
        return Err(Error::InvalidPayload);
    }
    let mut prices = BTreeMap::<u64, u64>::new();
    for pair in raw.as_chunks::<2>().0 {
        let (price, qty) = (uint(&pair[0])?, uint(&pair[1])?);
        if price == 0 || price > 1_000_000_000_000 || qty == 0 {
            return Err(Error::InvalidPayload);
        }
        let total = prices.entry(price).or_default();
        *total = total.checked_add(qty).ok_or(Error::InvalidPayload)?;
    }
    let mut levels: Vec<_> = prices
        .into_iter()
        .map(|(price_minor, quantity)| PriceLevel {
            price_minor,
            quantity,
        })
        .collect();
    if !asks {
        levels.reverse();
    }
    Ok(levels)
}
pub(crate) fn parse(payload: &Value) -> Result<OrderBook, Error> {
    let mut envelope = payload;
    if payload
        .get("data")
        .is_some_and(|d| d.get("success").is_some())
    {
        envelope = &payload["data"];
    }
    if envelope["success"] != true && envelope["success"] != 1 && envelope["success"] != "1" {
        return Err(Error::Rejected);
    }
    let data = &envelope["data"];
    let currency_id = uint(&data["eCurrency"])?;
    let currency = STEAM_CURRENCIES
        .iter()
        .find(|(id, _)| *id == currency_id)
        .ok_or(Error::UnsupportedCurrency)?
        .1
        .to_string()
        .try_into()
        .map_err(|_| Error::InvalidPayload)?;
    let book = OrderBook {
        currency,
        asks: levels(&data["rgCompactSellOrders"], true)?,
        bids: levels(&data["rgCompactBuyOrders"], false)?,
        reported_sell_orders: data.get("cSellOrders").map(uint).transpose()?,
        reported_buy_orders: data.get("cBuyOrders").map(uint).transpose()?,
    };
    book.validate().map_err(|_| Error::InvalidPayload)?;
    for (key, best) in [
        ("amtMinSellOrder", book.best_ask()),
        ("amtMaxBuyOrder", book.best_bid()),
    ] {
        if let Some(raw) = data.get(key)
            && uint(raw)? != best.unwrap_or(0)
        {
            return Err(Error::InvalidPayload);
        }
    }
    Ok(book)
}
impl OrderBookProvider for SteamOrderBook {
    async fn order_book(&self, request: &OrderBookRequest) -> Result<OrderBook, Error> {
        let url = request_url(request)?;
        // Serial pacing, not an automatic retry loop. Errors never fall back to demo data.
        let mut retry = self.retry_at.lock().await;
        if *retry > now() {
            return Err(Error::RateLimited(*retry));
        }
        *retry = now() + 2;
        let mut response = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .header("Referer", "https://steamcommunity.com/market/")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("x-valve-request-type", "queryAction")
            .send()
            .await
            .map_err(|_| Error::Network)?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let secs = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(300)
                .clamp(300, 604800);
            *retry = now() + secs;
            return Err(Error::RateLimited(*retry));
        }
        if !response.status().is_success() {
            return Err(Error::Rejected);
        }
        let mut raw = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| Error::Network)? {
            if raw.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(Error::InvalidPayload);
            }
            raw.extend_from_slice(&chunk);
        }
        parse(&serde_json::from_slice(&raw).map_err(|_| Error::InvalidPayload)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn payload() -> Value {
        json!({"success":true,"data":{"eCurrency":23,"amtMinSellOrder":100,"amtMaxBuyOrder":90,
        "rgCompactSellOrders":[120,2,100,3,100,4],"rgCompactBuyOrders":[80,4,90,2],"cSellOrders":20,"cBuyOrders":12}})
    }
    #[test]
    fn compact_depth_is_integer_sorted_aggregated_and_currency_is_authoritative() {
        let p = payload();
        let b = parse(&p).unwrap();
        assert_eq!(b.best_ask(), Some(100));
        assert_eq!(b.best_bid(), Some(90));
        assert_eq!(b.asks[0].quantity, 7);
        assert_eq!(b.bids[0].quantity, 2);
        let mut p = json!({"data":p});
        p["data"]["data"]["eCurrency"] = json!(30);
        let b = parse(&p).unwrap();
        assert_eq!(String::from(b.currency), "TWD");
        assert_eq!(b.asks[0].price_minor, 100);
    }
    #[test]
    fn explicit_empty_sides_are_not_failures_or_missing_sides() {
        let p = json!({"success":1,"data":{"eCurrency":23,"amtMinSellOrder":0,"amtMaxBuyOrder":0,
            "rgCompactSellOrders":[],"rgCompactBuyOrders":[]}});
        let b = parse(&p).unwrap();
        assert_eq!(b.best_ask(), None);
        assert_eq!(b.best_bid(), None);
        let mut missing = p.clone();
        missing["data"]
            .as_object_mut()
            .unwrap()
            .remove("rgCompactBuyOrders");
        assert!(parse(&missing).is_err());
        let mut failed = p;
        failed["success"] = json!(false);
        assert!(matches!(parse(&failed), Err(Error::Rejected)));
    }
    #[test]
    fn ukrainian_response_currency_18_keeps_native_prices_without_assuming_cny() {
        let mut p = payload();
        p["data"]["eCurrency"] = json!(18);
        let b = parse(&p).unwrap();
        assert_eq!(String::from(b.currency.clone()), "UAH");
        assert_eq!(b.best_ask(), Some(100));
        assert_eq!(b.best_bid(), Some(90));
        assert_eq!(b.asks[0].quantity, 7);
    }
    #[test]
    fn optional_request_currency_maps_to_steam_18_and_response_still_controls_the_native_unit() {
        let legacy = serde_json::json!({"app_id":730,"market_hash_name":"fixture"});
        let old: OrderBookRequest = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(serde_json::to_value(&old).unwrap(), legacy);
        assert!(
            request_url(&old)
                .unwrap()
                .query_pairs()
                .any(|(key, value)| key == "currency" && value == "23")
        );
        let requested: OrderBookRequest = serde_json::from_value(
            serde_json::json!({"app_id":730,"market_hash_name":"fixture","currency":"UAH"}),
        )
        .unwrap();
        assert!(
            request_url(&requested)
                .unwrap()
                .query_pairs()
                .any(|(key, value)| key == "currency" && value == "18")
        );
        let response = parse(&payload()).unwrap();
        let snapshot = OrderBookSnapshot::new(requested, response, now()).unwrap();
        assert_eq!(String::from(snapshot.requested_currency), "UAH");
        assert_eq!(String::from(snapshot.book.currency), "CNY");
        assert!(snapshot.warnings.iter().any(|w| w.contains("与请求不同")));
        let unsupported: OrderBookRequest = serde_json::from_value(
            serde_json::json!({"app_id":730,"market_hash_name":"fixture","currency":"JPY"}),
        )
        .unwrap();
        assert!(request_url(&unsupported).is_err());
    }
    #[test]
    fn malformed_prices_currency_depth_and_summary_do_not_produce_quotes() {
        for (field, value) in [
            ("rgCompactSellOrders", json!([100])),
            ("rgCompactSellOrders", json!([0, 1])),
            ("rgCompactSellOrders", json!(["1.25", 1])),
            ("rgCompactSellOrders", json!([100, -1])),
            ("rgCompactSellOrders", json!([100, u64::MAX, 100, 1])),
            ("amtMinSellOrder", json!(999)),
            ("eCurrency", json!(999)),
        ] {
            let mut p = payload();
            p["data"][field] = value;
            assert!(parse(&p).is_err(), "{field}");
        }
        let mut p = payload();
        p["data"]["eCurrency"] = json!(8);
        assert!(
            matches!(parse(&p), Err(Error::UnsupportedCurrency)),
            "unsupported zero-decimal currency must not assume /100"
        );
    }
    #[tokio::test]
    async fn pacing_does_not_make_a_request_during_cooldown() {
        let provider = SteamOrderBook::new().unwrap();
        *provider.retry_at.lock().await = now() + 300;
        let request = OrderBookRequest {
            currency: crate::domain::money::Currency::cny(),
            app_id: 730,
            market_hash_name: "fixture".into(),
        };
        assert!(matches!(
            provider.order_book(&request).await,
            Err(Error::RateLimited(_))
        ));
    }
}
