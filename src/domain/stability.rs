//! Calendar-based historical descriptors; never a strategy pass/fail or live-price substitute.
use super::{
    money::{Currency, Money},
    risk::DailyBar,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmaWindow {
    pub calendar_days: u32,
    pub usable_days: usize,
    pub ema: Option<Money>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrendWindow {
    pub calendar_days: u32,
    pub usable_days: usize,
    pub relative_slope_bps_per_day: Option<i64>,
    pub r_squared_bps: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IqrEvidence {
    pub method: String,
    pub applied: bool,
    pub retained_days: usize,
    pub excluded_utc_days: Vec<u64>,
    /// Display bounds only; filtering uses exact rational fences before rounding.
    pub lower_minor_floor: Option<i64>,
    pub upper_minor_ceil: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StabilityAnalysis {
    pub algorithm: String,
    pub basis: String,
    pub usable_days: usize,
    pub missing_or_unusable_days: u32,
    pub raw_sample_cv_bps: Option<u64>,
    pub clean_sample_cv_bps: Option<u64>,
    pub clean_mean: Option<Money>,
    pub clean_median: Option<Money>,
    pub iqr: IqrEvidence,
    pub ema7: EmaWindow,
    pub ema30: EmaWindow,
    pub trend7: TrendWindow,
    pub reference_utc_day: Option<u64>,
    pub reference_range_position_bps: Option<i64>,
    pub warnings: Vec<String>,
}
fn mean(values: &[u64]) -> Option<u64> {
    (!values.is_empty())
        .then(|| (values.iter().map(|&v| v as u128).sum::<u128>() / values.len() as u128) as u64)
}
fn cv(values: &[u64]) -> Option<u64> {
    if values.len() < 2 {
        return None;
    }
    let n = values.len() as u128;
    let sum = values.iter().map(|&v| v as u128).sum::<u128>();
    let squares = values
        .iter()
        .map(|&v| (v as u128) * (v as u128))
        .sum::<u128>();
    let variance_numerator = n * squares - sum * sum;
    // Only the dimensionless square root uses f64; sums and all money stay integer.
    Some(
        (((variance_numerator * n) as f64 / (n - 1) as f64).sqrt() * 10_000.0 / sum as f64).ceil()
            as u64,
    )
}
fn money(minor: u64, currency: &Currency) -> Money {
    Money {
        minor,
        currency: currency.clone(),
    }
}
fn ema(prices: &BTreeMap<u64, u64>, until: u64, span: u32, currency: &Currency) -> EmaWindow {
    let start = until.saturating_sub(span as u64);
    let window: Vec<_> = prices.range(start..until).map(|(_, v)| *v).collect();
    let amount = if until >= span as u64 && window.len() == span as usize {
        // Retain 1e6 sub-minor precision across the recurrence, floor only at output.
        let mut state = window[0] as u128 * 1_000_000;
        for &v in &window[1..] {
            state = (2 * v as u128 * 1_000_000 + (span as u128 - 1) * state) / (span as u128 + 1);
        }
        Some(money((state / 1_000_000) as u64, currency))
    } else {
        None
    };
    EmaWindow {
        calendar_days: span,
        usable_days: window.len(),
        ema: amount,
    }
}
fn trend(prices: &BTreeMap<u64, u64>, until: u64) -> TrendWindow {
    let start = until.saturating_sub(7);
    let window: Vec<_> = prices
        .range(start..until)
        .map(|(&d, &v)| ((d - start) as i128, v as i128))
        .collect();
    let n = window.len() as i128;
    let mut result = TrendWindow {
        calendar_days: 7,
        usable_days: window.len(),
        relative_slope_bps_per_day: None,
        r_squared_bps: None,
    };
    if n < 2 {
        return result;
    }
    let sx = window.iter().map(|p| p.0).sum::<i128>();
    let sy = window.iter().map(|p| p.1).sum::<i128>();
    let sxx = window.iter().map(|p| p.0 * p.0).sum::<i128>();
    let syy = window.iter().map(|p| p.1 * p.1).sum::<i128>();
    let sxy = window.iter().map(|p| p.0 * p.1).sum::<i128>();
    let covariance = n * sxy - sx * sy;
    let vx = n * sxx - sx * sx;
    let vy = n * syy - sy * sy;
    result.relative_slope_bps_per_day = Some((covariance * n * 10_000).div_euclid(vx * sy) as i64);
    // A constant series has known zero slope, but R² is undefined, not confidence=0/1.
    if vy > 0 {
        result.r_squared_bps = Some((covariance * covariance * 10_000 / (vx * vy)) as u32);
    }
    result
}
fn percentile_x4(sorted: &[u64], quarter: usize) -> i128 {
    let index = (sorted.len() - 1) * quarter;
    let lo = index / 4;
    let rem = index % 4;
    let hi = (lo + 1).min(sorted.len() - 1);
    sorted[lo] as i128 * 4 + (sorted[hi] - sorted[lo]) as i128 * rem as i128
}
pub fn analyze(
    bars: &[DailyBar],
    currency: &Currency,
    from: u64,
    until: u64,
) -> Result<StabilityAnalysis, String> {
    if until <= from || until - from > 365 || bars.len() > 365 {
        return Err("稳定性研究需要1–365个完整自然日窗口".into());
    }
    let mut previous = None;
    let mut prices = BTreeMap::new();
    for bar in bars {
        if bar.utc_day < from || bar.utc_day >= until || previous.is_some_and(|d| bar.utc_day <= d)
        {
            return Err("日曲线范围、顺序或重复日期无效".into());
        }
        previous = Some(bar.utc_day);
        if let Some(p) = &bar.vwap {
            // Matches HistorySample's maximum 1e10 source units /100 per native minor.
            if p.currency != *currency || p.minor == 0 || p.minor > 100_000_000 || bar.volume == 0 {
                return Err("稳定性日价币种、单位、范围或成交量无效".into());
            }
            prices.insert(bar.utc_day, p.minor);
        }
    }
    let raw: Vec<_> = prices.values().copied().collect();
    let mut sorted = raw.clone();
    sorted.sort_unstable();
    let mut iqr = IqrEvidence {
        method: "linear_quartiles_tukey_1_5_relative_mean_buffer_5pct".into(),
        applied: false,
        retained_days: raw.len(),
        excluded_utc_days: vec![],
        lower_minor_floor: None,
        upper_minor_ceil: None,
    };
    let mut clean = raw.clone();
    if raw.len() >= 3 {
        let n = raw.len() as i128;
        let sum = raw.iter().map(|&p| p as i128).sum::<i128>();
        let q1 = percentile_x4(&sorted, 1);
        let q3 = percentile_x4(&sorted, 3);
        // Common denominator 80*n: 1.5*max(IQR, 5% arithmetic mean).
        let buffer = ((q3 - q1) * 10 * n).max(2 * sum);
        let (lower, upper) = (q1 * 20 * n - 3 * buffer, q3 * 20 * n + 3 * buffer);
        let denominator = 80 * n;
        clean.clear();
        for (&day, &price) in &prices {
            if (lower..=upper).contains(&(price as i128 * denominator)) {
                clean.push(price);
            } else {
                iqr.excluded_utc_days.push(day);
            }
        }
        iqr.applied = true;
        iqr.retained_days = clean.len();
        iqr.lower_minor_floor = Some(lower.div_euclid(denominator) as i64);
        iqr.upper_minor_ceil = Some(((upper + denominator - 1) / denominator) as u64);
    }
    let mut clean_sorted = clean.clone();
    clean_sorted.sort_unstable();
    let median = (!clean_sorted.is_empty()).then(|| (percentile_x4(&clean_sorted, 2) / 4) as u64);
    let reference = prices.last_key_value();
    let position = reference.and_then(|(_, v)| {
        clean_sorted
            .first()
            .zip(clean_sorted.last())
            .and_then(|(&min, &max)| {
                (max > min).then(|| {
                    ((*v as i128 - min as i128) * 10_000).div_euclid((max - min) as i128) as i64
                })
            })
    });
    let ema7 = ema(&prices, until, 7, currency);
    let ema30 = ema(&prices, until, 30, currency);
    let trend7 = trend(&prices, until);
    let mut warnings = vec![
        "描述历史日样本，不是策略通过/失败、盈利预测或实时风险许可".into(),
        "IQR只用于独立的清洗后统计；原日线、EMA、趋势和七自然日P10保留全部有效日价".into(),
        "区间位置为最后历史价在清洗后最低/最高价间的位置，不是经验CDF或当前市场价分位".into(),
    ];
    if ema7.ema.is_none() || ema30.ema.is_none() {
        warnings.push("EMA要求对应自然日完整且价格可用；不以稀疏记录冒充连续7/30天".into());
    }
    if trend7.usable_days < 5 {
        warnings.push("近七自然日有效点少于5，趋势描述样本偏少；两点R²也不代表趋势可靠".into());
    }
    Ok(StabilityAnalysis {
        algorithm: "calendar_stability_v1".into(),
        basis: "positive_volume_daily_sample_prices_native_minor".into(),
        usable_days: raw.len(),
        missing_or_unusable_days: (until - from) as u32 - raw.len() as u32,
        raw_sample_cv_bps: cv(&raw),
        clean_sample_cv_bps: cv(&clean),
        clean_mean: mean(&clean).map(|v| money(v, currency)),
        clean_median: median.map(|v| money(v, currency)),
        iqr,
        ema7,
        ema30,
        trend7,
        reference_utc_day: reference.map(|(&day, _)| day),
        reference_range_position_bps: position,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bars(points: &[(u64, u64)]) -> Vec<DailyBar> {
        points
            .iter()
            .map(|&(utc_day, minor)| DailyBar {
                utc_day,
                vwap: Some(Money::cny(minor)),
                volume: 1,
                zero_volume_samples: 0,
                missing_volume_samples: 0,
            })
            .collect()
    }
    #[test]
    fn flat_series_is_zero_cv_but_not_defined_r_squared_or_range_position() {
        let b = bars(&(0..30).map(|d| (d, 100)).collect::<Vec<_>>());
        let a = analyze(&b, &Currency::cny(), 0, 30).unwrap();
        assert_eq!(a.raw_sample_cv_bps, Some(0));
        assert_eq!(a.clean_sample_cv_bps, Some(0));
        assert_eq!(a.ema7.ema.unwrap().minor, 100);
        assert_eq!(a.ema30.ema.unwrap().minor, 100);
        assert_eq!(a.trend7.relative_slope_bps_per_day, Some(0));
        assert_eq!(a.trend7.r_squared_bps, None);
        assert_eq!(a.reference_range_position_bps, None);
    }
    #[test]
    fn calendar_regression_uses_real_days_and_missing_days_block_ema() {
        let a = analyze(
            &bars(&[(0, 100), (3, 130), (6, 160)]),
            &Currency::cny(),
            0,
            7,
        )
        .unwrap();
        assert_eq!(a.trend7.relative_slope_bps_per_day, Some(769));
        assert_eq!(a.trend7.r_squared_bps, Some(10000));
        assert!(a.ema7.ema.is_none());
        assert!(a.ema30.ema.is_none());
        assert_eq!(a.missing_or_unusable_days, 4);
    }
    #[test]
    fn iqr_outliers_do_not_disappear_from_raw_risk_or_ema_inputs() {
        let mut input: Vec<(u64, u64)> = (0..30).map(|d| (d, 100)).collect();
        input[29].1 = 131;
        let b = bars(&input);
        let a = analyze(&b, &Currency::cny(), 0, 30).unwrap();
        assert_eq!(a.iqr.excluded_utc_days, vec![29]);
        assert_eq!(a.clean_sample_cv_bps, Some(0));
        assert!(a.raw_sample_cv_bps.unwrap() > 0);
        assert_eq!(a.ema7.ema.unwrap().minor, 107);
        assert_eq!(a.ema30.ema.unwrap().minor, 102);
        assert_eq!(b[29].vwap.as_ref().unwrap().minor, 131);
        let kept = analyze(
            &bars(&[(0, 100), (1, 100), (2, 100), (3, 100), (4, 107)]),
            &Currency::cny(),
            0,
            5,
        )
        .unwrap();
        assert!(kept.iqr.excluded_utc_days.is_empty());
        let excluded = analyze(
            &bars(&[(0, 100), (1, 100), (2, 100), (3, 100), (4, 108)]),
            &Currency::cny(),
            0,
            5,
        )
        .unwrap();
        assert_eq!(excluded.iqr.excluded_utc_days, vec![4]);
    }
    #[test]
    fn empty_single_and_unusable_days_remain_unknown() {
        let a = analyze(&[], &Currency::cny(), 0, 30).unwrap();
        assert!(a.raw_sample_cv_bps.is_none());
        assert!(a.clean_mean.is_none());
        let mut b = bars(&[(29, 100)]);
        let a = analyze(&b, &Currency::cny(), 0, 30).unwrap();
        assert!(a.raw_sample_cv_bps.is_none());
        assert!(a.trend7.relative_slope_bps_per_day.is_none());
        assert_eq!(a.clean_mean.unwrap().minor, 100);
        b[0].vwap = None;
        b[0].volume = 0;
        let a = analyze(&b, &Currency::cny(), 0, 30).unwrap();
        assert_eq!(a.usable_days, 0);
        assert_eq!(a.missing_or_unusable_days, 30);
    }
    #[test]
    fn decreasing_trend_is_signed_and_ratio_metrics_are_currency_scale_invariant() {
        let points: Vec<_> = (0..7).map(|d| (d, 160 - 10 * d)).collect();
        let a = analyze(&bars(&points), &Currency::cny(), 0, 7).unwrap();
        assert_eq!(a.trend7.relative_slope_bps_per_day, Some(-770));
        let mut other = bars(&points);
        for b in &mut other {
            let p = b.vwap.as_mut().unwrap();
            p.minor *= 10;
            p.currency = "TWD".to_string().try_into().unwrap();
        }
        let b = analyze(&other, &"TWD".to_string().try_into().unwrap(), 0, 7).unwrap();
        assert_eq!(a.raw_sample_cv_bps, b.raw_sample_cv_bps);
        assert_eq!(
            a.trend7.relative_slope_bps_per_day,
            b.trend7.relative_slope_bps_per_day
        );
        assert_eq!(a.iqr.excluded_utc_days, b.iqr.excluded_utc_days);
    }
    #[test]
    fn malformed_curves_cannot_become_metrics() {
        for points in [
            vec![(1, 100), (0, 100)],
            vec![(0, 100), (0, 100)],
            vec![(7, 100)],
            vec![(0, 0)],
            vec![(0, 100000001)],
        ] {
            assert!(analyze(&bars(&points), &Currency::cny(), 0, 7).is_err());
        }
        let mut b = bars(&[(0, 100)]);
        b[0].volume = 0;
        assert!(analyze(&b, &Currency::cny(), 0, 7).is_err());
        b[0].volume = 1;
        b[0].vwap.as_mut().unwrap().currency = "USD".to_string().try_into().unwrap();
        assert!(analyze(&b, &Currency::cny(), 0, 7).is_err());
        assert!(analyze(&[], &Currency::cny(), 0, 366).is_err());
    }
}
