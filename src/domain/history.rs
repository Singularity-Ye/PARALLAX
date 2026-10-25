//! Research evidence only: no implicit FX, current quote replacement, or execution authorization.
use super::{
    money::Currency,
    risk::{DailyBar, SevenDayRisk, daily_bars_scaled, seven_day_risk},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryRequest {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scan_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub manual_item: bool,
    pub item_key: String,
    pub window_days: u32,
    pub slippage_bps: u32,
}
fn is_false(v: &bool) -> bool {
    !*v
}
impl HistoryRequest {
    pub fn validate(&self) -> Result<(), String> {
        let valid_id = |s: &str| !s.is_empty() && s.len() <= 64 && !s.chars().any(char::is_control);
        let sources = usize::from(!self.scan_id.is_empty())
            + usize::from(self.inventory_snapshot_id.is_some())
            + usize::from(self.manual_item);
        let valid_origin = sources == 1
            && (self.scan_id.is_empty() || valid_id(&self.scan_id))
            && self.inventory_snapshot_id.as_deref().is_none_or(valid_id);
        if !valid_origin
            || self.item_key.is_empty()
            || self.item_key.trim() != self.item_key
            || self.item_key.len() > 500
            || self
                .scan_id
                .chars()
                .chain(self.item_key.chars())
                .any(char::is_control)
            || !(8..=365).contains(&self.window_days)
            || self.slippage_bps > 10000
        {
            return Err("历史研究需且仅需扫描、库存快照或显式市场名称来源、商品标识、8–365 个自然日窗口与 0–10000 bps 滑点".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HistorySample {
    pub at: u64,
    /// Integer source price, 10000 units per major currency unit, not cents.
    pub price_major_1e4: u64,
    pub volume: Option<u64>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HistorySeries {
    pub currency: Option<Currency>,
    pub currency_basis: String,
    pub points: Vec<HistorySample>,
    pub response_rows: u32,
    pub excluded_before_window: u32,
    pub excluded_open_day: u32,
    pub from_utc_day: u64,
    pub until_utc_day_exclusive: u64,
}
impl HistorySeries {
    pub fn validate(&self, request: &HistoryRequest, at: u64) -> Result<(), String> {
        let today = at / 86400;
        if self.until_utc_day_exclusive != today
            || self.from_utc_day != today.saturating_sub(u64::from(request.window_days))
            || self.points.len() > 20000
            || self.response_rows > 100000
            || u64::from(self.response_rows)
                != self.points.len() as u64
                    + u64::from(self.excluded_before_window)
                    + u64::from(self.excluded_open_day)
            || !["explicit_code_or_unique_marker", "ambiguous_or_unsupported"]
                .contains(&self.currency_basis.as_str())
            || (self.currency.is_some()
                != (self.currency_basis == "explicit_code_or_unique_marker"))
        {
            return Err("历史范围或币种证据无效".into());
        }
        if let Some(currency) = &self.currency {
            let code: String = currency.clone().into();
            if !super::fx::SUPPORTED.contains(&code.as_str()) {
                return Err("历史最小币种单位尚未适配".into());
            }
        }
        let mut previous = None;
        for p in &self.points {
            if p.price_major_1e4 == 0
                || p.price_major_1e4 > 10_000_000_000
                || p.volume.is_some_and(|v| v > 1_000_000_000)
                || p.at / 86400 < self.from_utc_day
                || p.at / 86400 >= today
                || previous.is_some_and(|t| p.at <= t)
            {
                return Err("历史价格、成交量、日期或顺序无效".into());
            }
            previous = Some(p.at);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HistoryAnalysis {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stability: Option<super::stability::StabilityAnalysis>,
    pub status: String,
    pub daily_bars: Vec<DailyBar>,
    pub missing_calendar_days: u32,
    pub below_minor_unit_days: u32,
    pub reference_utc_day: Option<u64>,
    pub risk: Option<SevenDayRisk>,
    pub warnings: Vec<String>,
}
pub fn analyze(
    series: &HistorySeries,
    request: &HistoryRequest,
    at: u64,
) -> Result<HistoryAnalysis, String> {
    request.validate()?;
    series.validate(request, at)?;
    let mut result = HistoryAnalysis {
        stability: None,
        status: "unknown_currency".into(),
        daily_bars: vec![],
        missing_calendar_days: request.window_days
            - series
                .points
                .iter()
                .map(|p| p.at / 86400)
                .collect::<std::collections::HashSet<_>>()
                .len() as u32,
        below_minor_unit_days: 0,
        reference_utc_day: None,
        risk: None,
        warnings: vec![
            "价格历史来自社区接口；商品身份按已验证来源与请求参数绑定，非响应签名证明".into(),
            "自然日聚合的是平台历史价格样本，不保证为逐笔成交的真实 VWAP".into(),
            "P10 为过去七自然日收益分位，不是未来损失上限或盈利保证".into(),
        ],
    };
    let Some(currency) = &series.currency else {
        result
            .warnings
            .push("币种符号含糊或单位未适配，保留原始精度样本但不生成货币估值/P10".into());
        return Ok(result);
    };
    let points: Vec<_> = series
        .points
        .iter()
        .map(|p| (p.at, p.price_major_1e4, p.volume))
        .collect();
    result.daily_bars = daily_bars_scaled(&points, currency, 100)?;
    result.stability = Some(super::stability::analyze(
        &result.daily_bars,
        currency,
        series.from_utc_day,
        series.until_utc_day_exclusive,
    )?);
    result.missing_calendar_days = request.window_days - result.daily_bars.len() as u32;
    result.below_minor_unit_days = result
        .daily_bars
        .iter()
        .filter(|b| b.volume > 0 && b.vwap.is_none())
        .count() as u32;
    if let Some(reference) = result.daily_bars.iter().rev().find(|b| b.vwap.is_some()) {
        result.reference_utc_day = Some(reference.utc_day);
        let risk = seven_day_risk(
            &result.daily_bars,
            reference.vwap.as_ref().unwrap(),
            request.slippage_bps,
        )?;
        result.status = if risk.sample_count == 0 {
            "insufficient_pairs"
        } else {
            "historical_pairs_available"
        }
        .into();
        result.risk = Some(risk);
    } else {
        result.status = "no_usable_daily_prices".into();
    }
    result
        .warnings
        .push("参考价来自最后一个有效历史日，非实时买卖报价；未更新预算或扫描".into());
    Ok(result)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HistorySnapshot {
    pub id: String,
    pub account_id: String,
    pub credential_generation: String,
    pub request: HistoryRequest,
    pub started_at: u64,
    pub observed_at: u64,
    pub source: String,
    pub identity_binding: String,
    pub platform_verified: bool,
    pub series: HistorySeries,
    pub analysis: HistoryAnalysis,
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
