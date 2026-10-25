//! Shared historical rule evaluation for plan-bound and independent research.
use super::{
    history::HistorySnapshot,
    strategy::{CvBasis, RuleDecision, StrategyRule, decision, fresh},
};
use serde_json::{Value, json};
pub(crate) fn history_rule(
    rule: &StrategyRule,
    history: Option<&HistorySnapshot>,
    covered: &mut bool,
    at: u64,
    expected_binding: &str,
) -> RuleDecision {
    if !matches!(rule, StrategyRule::HistoryCoverage { .. }) && !*covered {
        return decision(
            rule.key(),
            None,
            "历史覆盖门未通过，不能据此放行指标规则",
            Value::Null,
        );
    }
    match rule {
        StrategyRule::HistoryCoverage {
            min_usable_days,
            max_missing_days,
            max_age_secs,
            max_reference_lag_days,
        } => {
            if let Some(h) = history.filter(|h| {
                h.source == "steam_authenticated_pricehistory"
                    && h.identity_binding == expected_binding
                    && h.series.currency.is_some()
                    && h.series.currency_basis == "explicit_code_or_unique_marker"
                    && h.started_at <= h.observed_at
                    && fresh(h.observed_at, at, *max_age_secs)
                    && h.series.until_utc_day_exclusive == at / 86400
            }) {
                if let Some(s) = h
                    .analysis
                    .stability
                    .as_ref()
                    .filter(|s| s.algorithm == "calendar_stability_v1")
                {
                    let lag = s
                        .reference_utc_day
                        .and_then(|day| (at / 86400).checked_sub(day));
                    let pass = s.usable_days >= *min_usable_days as usize
                        && s.missing_or_unusable_days <= *max_missing_days
                        && lag.is_some_and(|d| d >= 1 && d <= u64::from(*max_reference_lag_days));
                    *covered = pass;
                    decision(
                        rule.key(),
                        Some(pass),
                        "按结束的 UTC 自然日校验有效样本、缺口和最后有效日",
                        json!({"snapshot_id":h.id,"observed_at":h.observed_at,"currency":h.series.currency,"usable_days":s.usable_days,"missing_days":s.missing_or_unusable_days,"reference_lag_days":lag}),
                    )
                } else {
                    decision(
                        rule.key(),
                        None,
                        "旧快照缺少受支持的稳定性指标；回看不重算",
                        Value::Null,
                    )
                }
            } else {
                decision(
                    rule.key(),
                    None,
                    "历史缺失、币种/来源不明、过期或未覆盖当前结束日；不猜测安全",
                    Value::Null,
                )
            }
        }
        StrategyRule::SampleCv { basis, max_cv_bps } => {
            let s = history.unwrap().analysis.stability.as_ref().unwrap();
            let cv = match basis {
                CvBasis::Raw => s.raw_sample_cv_bps,
                CvBasis::IqrCleaned => s.clean_sample_cv_bps,
            };
            decision(
                rule.key(),
                cv.map(|v| v <= u64::from(*max_cv_bps)),
                "所选样本 CV 不高于上限；清洗后结果不替代原始尾部风险",
                json!({"basis":basis,"cv_bps":cv,"max_cv_bps":max_cv_bps,"raw_cv_bps":s.raw_sample_cv_bps,"excluded_days":s.iqr.excluded_utc_days}),
            )
        }
        StrategyRule::Trend7 {
            min_points,
            min_slope_bps_per_day,
            max_slope_bps_per_day,
            min_r_squared_bps,
        } => {
            let trend = &history.unwrap().analysis.stability.as_ref().unwrap().trend7;
            let outcome = if trend.usable_days < *min_points as usize {
                None
            } else {
                trend
                    .relative_slope_bps_per_day
                    .zip(trend.r_squared_bps)
                    .map(|(slope, r2)| {
                        slope >= *min_slope_bps_per_day
                            && slope <= *max_slope_bps_per_day
                            && r2 >= *min_r_squared_bps
                    })
            };
            decision(
                rule.key(),
                outcome,
                "近七个自然日的相对斜率范围与 R² 门槛；常量序列 R² 未定义",
                json!({"trend":trend,"min_points":min_points,"min_slope_bps_per_day":min_slope_bps_per_day,"max_slope_bps_per_day":max_slope_bps_per_day,"min_r_squared_bps":min_r_squared_bps}),
            )
        }
        StrategyRule::CalendarP10 {
            min_pairs,
            min_return_bps,
            slippage_bps,
        } => {
            let risk = history.unwrap().analysis.risk.as_ref();
            let result = risk
                .filter(|r| {
                    r.algorithm == "calendar_p10_v1"
                        && r.sample_count >= *min_pairs as usize
                        && r.slippage_bps == *slippage_bps
                })
                .and_then(|r| r.return_p10_bps)
                .map(|p10| p10 >= *min_return_bps);
            decision(
                rule.key(),
                result,
                "七自然日历史收益 P10 下限；需足够配对与相同压力滑点，P10 不是未来损失保证",
                json!({"risk":risk,"min_pairs":min_pairs,"min_return_bps":min_return_bps,"required_slippage_bps":slippage_bps}),
            )
        }
        _ => decision(rule.key(), None, "此规则不是历史指标规则", Value::Null),
    }
}
