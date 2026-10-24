//! Indicative FX evidence; never changes an account's actual settlement currency.
use super::money::{Currency, Money};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const SCALE: u64 = 100_000_000;
pub const SUPPORTED: [&str; 6] = ["CNY", "USD", "EUR", "GBP", "HKD", "TWD"];
pub fn supported(c: &Currency) -> bool {
    SUPPORTED.contains(&String::from(c.clone()).as_str())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxRate {
    pub date: String,
    pub units_per_cny_1e8: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FxSnapshot {
    pub id: String,
    pub source: String,
    pub fetched_at: u64,
    pub rates: BTreeMap<String, FxRate>,
}
impl FxSnapshot {
    pub fn validate(&self, at: u64) -> Result<(), String> {
        if self.source != "frankfurter_v2_blended"
            || self.fetched_at > at
            || at - self.fetched_at > 6 * 3600
        {
            return Err("汇率快照过期、来源或时间异常，需要刷新".into());
        }
        let today = chrono::DateTime::from_timestamp(at as i64, 0)
            .ok_or("时间无效")?
            .date_naive();
        if self.rates.len() != 6 {
            return Err("汇率币种不完整".into());
        }
        for code in SUPPORTED {
            let r = self.rates.get(code).ok_or("汇率缺失")?;
            let date = chrono::NaiveDate::parse_from_str(&r.date, "%Y-%m-%d")
                .map_err(|_| "汇率日期无效")?;
            let age = (today - date).num_days();
            if !(0..=7).contains(&age)
                || r.units_per_cny_1e8 == 0
                || r.units_per_cny_1e8 > 1_000_000_000_000
                || (code == "CNY" && r.units_per_cny_1e8 != SCALE)
            {
                return Err("汇率日期过旧/在未来或比率无效".into());
            }
        }
        Ok(())
    }
    pub fn factor(&self, from: &Currency, to: &Currency) -> Result<(u64, u64), String> {
        if !supported(from) || !supported(to) {
            return Err("币种最小单位尚未适配，不能猜测小数位".into());
        }
        let read = |c: &Currency| {
            self.rates
                .get(&String::from(c.clone()))
                .map(|r| r.units_per_cny_1e8)
                .filter(|n| *n > 0 && *n <= 1_000_000_000_000)
                .ok_or("汇率缺失或无效".to_string())
        };
        Ok((read(to)?, read(from)?))
    }
    /// All currently adapted currencies use hundredths. Truncate income/display estimates.
    pub fn convert(&self, money: &Money, to: &Currency) -> Result<Money, String> {
        let (n, d) = self.factor(&money.currency, to)?;
        let value = (money.minor as u128) * (n as u128) / (d as u128);
        if value > 1_000_000_000_000 {
            return Err("换算金额超出支持范围".into());
        }
        Ok(Money {
            minor: value as u64,
            currency: to.clone(),
        })
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn fixture(at: u64) -> FxSnapshot {
        let date = chrono::DateTime::from_timestamp(at as i64, 0)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        FxSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            source: "frankfurter_v2_blended".into(),
            fetched_at: at,
            rates: SUPPORTED
                .into_iter()
                .map(|code| {
                    (
                        code.into(),
                        FxRate {
                            date: date.clone(),
                            units_per_cny_1e8: if code == "TWD" { 5 * SCALE } else { SCALE },
                        },
                    )
                })
                .collect(),
        }
    }
    #[test]
    fn cross_conversion_is_directional_exact_and_never_relabels_native_money() {
        let fx = fixture(1_792_800_000);
        fx.validate(fx.fetched_at).unwrap();
        let native = Money {
            minor: 4600,
            currency: "TWD".to_string().try_into().unwrap(),
        };
        assert_eq!(fx.convert(&native, &Currency::cny()).unwrap().minor, 920);
        assert_eq!(
            fx.convert(&Money::cny(921), &native.currency)
                .unwrap()
                .minor,
            4605
        );
        assert_eq!(native.minor, 4600);
        assert_eq!(String::from(native.currency), "TWD");
        assert!(
            fx.convert(&Money::cny(1), &"JPY".to_string().try_into().unwrap())
                .is_err()
        );
    }
    #[test]
    fn future_stale_missing_rates_and_fraction_rounding_are_explicit() {
        let mut fx = fixture(1_792_800_000);
        assert!(fx.validate(fx.fetched_at - 1).is_err());
        assert!(fx.validate(fx.fetched_at + 21601).is_err());
        fx.rates.get_mut("TWD").unwrap().units_per_cny_1e8 = 3 * SCALE;
        let m = Money {
            minor: 1,
            currency: "TWD".to_string().try_into().unwrap(),
        };
        assert_eq!(fx.convert(&m, &Currency::cny()).unwrap().minor, 0);
        fx.rates.remove("USD");
        assert!(fx.validate(fx.fetched_at).is_err());
    }
}
