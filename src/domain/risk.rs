//! Offline, calendar-day research primitives. No fallback that turns missing history into safety.
use super::money::{Currency, Money};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PricePoint {
    pub at: u64,
    pub price: Money,
    pub volume: Option<u64>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DailyBar {
    pub utc_day: u64,
    pub vwap: Option<Money>,
    pub volume: u64,
    pub zero_volume_samples: u32,
    pub missing_volume_samples: u32,
}
pub fn daily_bars(points: &[PricePoint], currency: &Currency) -> Result<Vec<DailyBar>, String> {
    if points.len() > 20_000 {
        return Err("历史样本最多 20000 条".into());
    }
    // weighted sum, positive volume, zero-count, missing-count
    let mut days: BTreeMap<u64, (u128, u64, u32, u32)> = BTreeMap::new();
    for point in points {
        if point.price.currency != *currency {
            return Err("历史价格币种不一致，需要先显式转换".into());
        }
        if point.price.minor == 0 {
            return Err("历史价格必须大于零".into());
        }
        let row = days.entry(point.at / 86_400).or_default();
        match point.volume {
            Some(0) => row.2 += 1,
            None => row.3 += 1,
            Some(v) => {
                row.0 = row
                    .0
                    .checked_add((point.price.minor as u128) * (v as u128))
                    .ok_or("历史加权金额溢出")?;
                row.1 = row.1.checked_add(v).ok_or("历史成交量溢出")?;
            }
        }
    }
    days.into_iter()
        .map(|(day, (pv, v, zero, missing))| {
            Ok(DailyBar {
                utc_day: day,
                vwap: if v == 0 {
                    None
                } else {
                    Some(Money {
                        minor: u64::try_from(pv / (v as u128)).map_err(|_| "日均价溢出")?,
                        currency: currency.clone(),
                    })
                },
                volume: v,
                zero_volume_samples: zero,
                missing_volume_samples: missing,
            })
        })
        .collect()
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SevenDayRisk {
    pub algorithm: String,
    pub sample_count: usize,
    pub return_p10_bps: Option<i64>,
    pub conservative_gross: Option<Money>,
    pub slippage_bps: u32,
}
pub fn seven_day_risk(
    bars: &[DailyBar],
    reference: &Money,
    slippage_bps: u32,
) -> Result<SevenDayRisk, String> {
    if bars.len() > 20_000 || slippage_bps > 10_000 || reference.minor == 0 {
        return Err("历史窗口或滑点参数无效".into());
    }
    let mut prices = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    for bar in bars {
        if !seen.insert(bar.utc_day) {
            return Err("日曲线含重复日期，必须先聚合".into());
        }
        if let Some(price) = &bar.vwap {
            if price.currency != reference.currency || price.minor == 0 || bar.volume == 0 {
                return Err("日曲线币种、价格或成交量无效".into());
            }
            prices.insert(bar.utc_day, price.minor);
        }
    }
    let mut returns = Vec::new();
    for (&day, &after) in &prices {
        if let Some(before) = day.checked_sub(7).and_then(|d| prices.get(&d)) {
            let bps = ((i128::from(after) - i128::from(*before)) * 10_000)
                .div_euclid(i128::from(*before));
            returns.push(i64::try_from(bps).map_err(|_| "历史收益比例溢出")?);
        }
    }
    returns.sort_unstable();
    let p10 = (!returns.is_empty()).then(|| returns[(returns.len() - 1) / 10]);
    let conservative = p10.map(|r| {
        let factor = (10_000 + r.min(0)) as u128;
        Money {
            minor: ((reference.minor as u128) * factor * u128::from(10_000 - slippage_bps)
                / 100_000_000) as u64,
            currency: reference.currency.clone(),
        }
    });
    Ok(SevenDayRisk {
        algorithm: "calendar_p10_v1".into(),
        sample_count: returns.len(),
        return_p10_bps: p10,
        conservative_gross: conservative,
        slippage_bps,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_and_missing_volume_never_create_vwap_outside_traded_prices() {
        let points = [
            PricePoint {
                at: 0,
                price: Money::cny(100),
                volume: Some(0),
            },
            PricePoint {
                at: 1,
                price: Money::cny(200),
                volume: Some(10),
            },
            PricePoint {
                at: 2,
                price: Money::cny(999),
                volume: None,
            },
        ];
        let bars = daily_bars(&points, &Currency::cny()).unwrap();
        assert_eq!(bars[0].vwap.as_ref().unwrap().minor, 200);
        assert_eq!(
            (
                bars[0].volume,
                bars[0].zero_volume_samples,
                bars[0].missing_volume_samples
            ),
            (10, 1, 1)
        );
        assert!(
            daily_bars(&points[..1], &Currency::cny()).unwrap()[0]
                .vwap
                .is_none()
        );
    }
    #[test]
    fn calendar_pairs_zero_return_and_missing_samples_are_distinct() {
        let point = |day: u64| PricePoint {
            at: day * 86400,
            price: Money::cny(10_000),
            volume: Some(1),
        };
        let sparse = daily_bars(&[point(0), point(14)], &Currency::cny()).unwrap();
        assert_eq!(
            seven_day_risk(&sparse, &Money::cny(10000), 300)
                .unwrap()
                .return_p10_bps,
            None
        );
        let exact = daily_bars(&[point(0), point(7)], &Currency::cny()).unwrap();
        let risk = seven_day_risk(&exact, &Money::cny(10000), 300).unwrap();
        assert_eq!(risk.return_p10_bps, Some(0));
        assert_eq!(risk.conservative_gross.unwrap().minor, 9700);
        let mut duplicated = exact.clone();
        duplicated.push(exact[0].clone());
        assert!(seven_day_risk(&duplicated, &Money::cny(10000), 300).is_err());
    }
}
