use crate::application::ports::MarketSource;
use crate::domain::selection::{MarketItem, ScanRequest, SourceKind, now};
use serde_json::{Value, json};
use std::time::Duration;

pub struct Provider {
    client: reqwest::Client,
}
impl Provider {
    pub fn new() -> Result<Self, reqwest::Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("Mozilla/5.0 ShellTrade/0.1")
                .build()?,
        })
    }
}
impl MarketSource for Provider {
    async fn page(&self, request: &ScanRequest, page: usize) -> Result<Vec<MarketItem>, String> {
        if request.source == SourceKind::Demo {
            return Ok(demo_items()
                .into_iter()
                .skip((page - 1) * request.page_size)
                .take(request.page_size)
                .collect());
        }
        // Ported protocol shape from AetherSwap. This is a read-only ranking query.
        let prefix = (now() * 1000)
            .to_string()
            .chars()
            .take(12)
            .collect::<String>();
        let checksum: u32 = prefix.chars().filter_map(|c| c.to_digit(10)).sum();
        let timestamp = format!("{prefix}{}", checksum % 10);
        let response = self.client.post("https://www.steamdt.com/api/user/ranking/v1/hanging-knife")
            .query(&[("timestamp", &timestamp)])
            .header("Origin", "https://www.steamdt.com")
            .header("Referer", "https://www.steamdt.com/hanging")
            .header("Language", "zh_CN").header("X-Currency", "CNY")
            .header("X-App-Version", "1.0.0").header("X-Device", "1")
            .header("Access-Token", "undefined")
            .json(&json!({"page":page,"pageSize":request.page_size,"type":"swap",
                "wantToGet":"STEAM_BALANCE","purchasePlan":"","salePlan":"STEAM_SELL_PRICE",
                "minSellPrice":format!("{:.2}", request.min_price_cents as f64 / 100.0),
                "maxSellPrice":request.max_price_cents as f64 / 100.0,
                "minTransactionCount":request.min_volume.to_string(),"platformList":["BUFF"],"timestamp":timestamp}))
            .send().await.map_err(|_| "SteamDT 网络请求失败或超时".to_string())?;
        if !response.status().is_success() {
            return Err(format!("SteamDT HTTP {}，未当作空结果", response.status()));
        }
        let value: Value = response
            .json()
            .await
            .map_err(|_| "SteamDT 返回非预期 JSON".to_string())?;
        parse_response(&value)
    }
}

fn numeric(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}
fn cents(value: &Value) -> u64 {
    numeric(value)
        .filter(|n| n.is_finite() && *n > 0.0 && *n <= 1_000_000.0)
        .map(|n| (n * 100.0).round() as u64)
        .unwrap_or(0)
}
fn goods_id(link: &str) -> Option<u64> {
    let url = reqwest::Url::parse(link).ok()?;
    if !matches!(url.host_str(), Some("buff.163.com" | "www.buff.163.com")) {
        return None;
    }
    let mut segments = url.path_segments()?;
    if segments.next()? != "goods" {
        return None;
    }
    segments.next()?.parse().ok().filter(|id| *id > 0)
}
pub fn parse_response(value: &Value) -> Result<Vec<MarketItem>, String> {
    if value.get("success").and_then(Value::as_bool) != Some(true) {
        return Err("SteamDT 未确认请求成功（可能限流或接口变化）".into());
    }
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or("SteamDT 数据结构变化：缺少 data 数组")?;
    data.iter()
        .map(|item| {
            let key = item
                .get("marketHashName")
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
                .ok_or("SteamDT 商品缺少市场名称")?;
            let platforms = item
                .get("platformList")
                .and_then(Value::as_array)
                .ok_or("SteamDT 商品缺少平台列表")?;
            let buff = platforms.iter().find(|p| p["platformEnum"] == "BUFF");
            let steam = platforms.iter().find(|p| p["platformEnum"] == "STEAM");
            let cash = buff.and_then(|p| p.get("price")).unwrap_or(&Value::Null);
            let steam_price = steam
                .and_then(|p| p.get("price"))
                .or_else(|| item.get("steamPrice"))
                .unwrap_or(&Value::Null);
            Ok(MarketItem {
                key: key.into(),
                name: item["name"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(key)
                    .into(),
                goods_id: buff.and_then(|p| p["linkUrl"].as_str()).and_then(goods_id),
                cash_cents: cents(cash),
                steam_gross_cents: cents(steam_price),
                volume: numeric(&item["transactionCount"])
                    .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1e12)
                    .unwrap_or(0.0) as u64,
                observed_at: now(),
            })
        })
        .collect()
}

pub fn demo_items() -> Vec<MarketItem> {
    let names = [
        "演示 · 潮汐武器箱",
        "演示 · 琥珀纪念包",
        "演示 · 海盐涂装",
        "演示 · 印花测试",
        "演示 · 低成交量",
        "演示 · 缺失行情",
        "演示 · 银沙武器箱",
        "演示 · 昂贵报价",
        "演示 · 无平台映射",
        "演示 · 星屑涂装",
        "演示 · 珊瑚纪念包",
        "演示 · 灯塔武器箱",
    ];
    let mut rows: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, name)| MarketItem {
            key: format!("demo-{i}"),
            name: name.to_string(),
            goods_id: Some(1000 + i as u64),
            cash_cents: 500 + i as u64 * 120,
            steam_gross_cents: 900 + i as u64 * 180,
            volume: 300 + i as u64 * 30,
            observed_at: now(),
        })
        .collect();
    rows[4].volume = 20;
    rows[5].steam_gross_cents = 0;
    rows[7].cash_cents = 50_000;
    rows[8].goods_id = None;
    // Duplicate deliberately verifies coverage accounting across page boundaries.
    rows.push(rows[0].clone());
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_failure_and_empty_success_are_distinct() {
        assert!(parse_response(&json!({"success":false,"data":[]})).is_err());
        assert!(parse_response(&json!({"success":true,"data":{}})).is_err());
        assert_eq!(
            parse_response(&json!({"success":true,"data":[]}))
                .unwrap()
                .len(),
            0
        );
    }
    #[test]
    fn mapping_requires_real_buff_hostname() {
        assert_eq!(
            goods_id("https://buff.163.com/goods/123?from=test"),
            Some(123)
        );
        assert_eq!(goods_id("https://evil.test/goods/123"), None);
    }
}
