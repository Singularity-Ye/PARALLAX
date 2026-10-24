use crate::{
    application::ports::FxProvider,
    domain::fx::{FxRate, FxSnapshot, SCALE, SUPPORTED},
};
use serde_json::Value;
pub struct Frankfurter {
    client: reqwest::Client,
}
impl Frankfurter {
    pub fn new() -> anyhow::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("Parallax/0.1")
                .build()?,
        })
    }
}
fn scaled(value: &Value) -> Result<u64, String> {
    let raw = value
        .as_number()
        .ok_or("汇率须为JSON十进制数字")?
        .to_string();
    let mut split = raw.split('.');
    let whole = split.next().unwrap_or("");
    let fraction = split.next().unwrap_or("");
    if split.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 8
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("汇率格式或精度不支持".into());
    }
    let major: u64 = whole.parse().map_err(|_| "汇率整数无效")?;
    let minor: u64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().map_err(|_| "汇率小数无效")?
    };
    major
        .checked_mul(SCALE)
        .and_then(|v| v.checked_add(minor * 10u64.pow(8 - fraction.len() as u32)))
        .filter(|v| *v > 0 && *v <= 1_000_000_000_000)
        .ok_or("汇率范围无效".into())
}
pub(crate) fn parse(v: &Value, at: u64) -> Result<FxSnapshot, String> {
    let rows = v
        .as_array()
        .filter(|r| r.len() == 5)
        .ok_or("汇率响应币种不完整")?;
    let today = chrono::DateTime::from_timestamp(at as i64, 0)
        .ok_or("时间无效")?
        .format("%Y-%m-%d")
        .to_string();
    let mut rates = std::collections::BTreeMap::from([(
        "CNY".into(),
        FxRate {
            date: today,
            units_per_cny_1e8: SCALE,
        },
    )]);
    for row in rows {
        let code = row["quote"].as_str().ok_or("缺少汇率币种")?;
        if row["base"] != "CNY"
            || code == "CNY"
            || !SUPPORTED.contains(&code)
            || rates.contains_key(code)
        {
            return Err("汇率基准/币种重复或不符".into());
        }
        rates.insert(
            code.into(),
            FxRate {
                date: row["date"].as_str().ok_or("缺少汇率日期")?.into(),
                units_per_cny_1e8: scaled(&row["rate"])?,
            },
        );
    }
    let result = FxSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        source: "frankfurter_v2_blended".into(),
        fetched_at: at,
        rates,
    };
    result.validate(at)?;
    Ok(result)
}
impl FxProvider for Frankfurter {
    async fn fetch_rates(&self, at: u64) -> Result<FxSnapshot, String> {
        let day = chrono::DateTime::from_timestamp(at as i64, 0)
            .ok_or("时间无效")?
            .format("%Y-%m-%d")
            .to_string();
        let mut response = self
            .client
            .get("https://api.frankfurter.dev/v2/rates")
            .query(&[
                ("base", "CNY"),
                ("quotes", "USD,EUR,GBP,HKD,TWD"),
                ("date", &day),
            ])
            .send()
            .await
            .map_err(|_| "汇率网络请求失败；未采用固定或过期汇率")?;
        if !response.status().is_success() {
            return Err(format!(
                "汇率服务返回 HTTP {}，请稍后重试",
                response.status().as_u16()
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "汇率响应读取失败")? {
            if bytes.len() + chunk.len() > 65536 {
                return Err("汇率响应超限".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(
            &serde_json::from_slice(&bytes).map_err(|_| "汇率响应不是有效JSON")?,
            at,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimals_are_exact_and_malformed_pairs_dates_rates_fail() {
        assert_eq!(scaled(&serde_json::json!(4.8799)).unwrap(), 487990000);
        for v in [
            serde_json::json!(0),
            serde_json::json!(-1),
            serde_json::json!("1.2"),
            serde_json::from_str("0.123456789").unwrap(),
        ] {
            assert!(scaled(&v).is_err());
        }
        let at = 1792800000;
        let rows: Vec<_> = SUPPORTED
            .iter()
            .filter(|c| **c != "CNY")
            .map(|c| serde_json::json!({"base":"CNY","quote":c,"date":"2026-10-23","rate":1}))
            .collect();
        let mut v = serde_json::json!(rows);
        parse(&v, at).unwrap();
        v[0]["date"] = serde_json::json!("2099-01-01");
        assert!(parse(&v, at).is_err());
        v[0]["date"] = serde_json::json!("2026-10-23");
        v[0]["quote"] = v[1]["quote"].clone();
        assert!(parse(&v, at).is_err());
    }
}
