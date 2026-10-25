//! Frozen evidence comparison, reusing holding rules; no current-price or return claims.
use super::{
    history::HistorySnapshot,
    holding_research::{HoldingResearchRequest, evaluate},
    money::Money,
    strategy::{ItemDecision, StrategyVersion},
    watchlist::{WatchItem, WatchObservation},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineRequest {
    pub observation_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchBaseline {
    pub account_id: String,
    pub watch_id: String,
    pub fixed_at: u64,
    pub observation: WatchObservation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompareRequest {
    pub observation_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricChange {
    pub key: String,
    pub before: Option<i64>,
    pub after: Option<i64>,
    pub delta: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceEvidence {
    pub reference_utc_day: Option<u64>,
    pub price: Option<Money>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchComparison {
    pub item_key: String,
    pub baseline_observation_id: String,
    pub later_observation_id: String,
    pub baseline_fixed_at: u64,
    pub baseline_collected_at: u64,
    pub later_collected_at: u64,
    pub baseline_research_at: u64,
    pub later_research_at: u64,
    pub collection_after_baseline: bool,
    pub new_completed_day: bool,
    pub same_measurement_basis: bool,
    pub fixed_strategy: StrategyVersion,
    pub later_original_strategy: StrategyVersion,
    pub before: ItemDecision,
    pub after_fixed_rules: ItemDecision,
    pub before_price: PriceEvidence,
    pub after_price: PriceEvidence,
    pub historical_price_change_bps: Option<i64>,
    pub metrics: Vec<MetricChange>,
    pub limitations: Vec<String>,
    pub execution_allowed: bool,
    pub returns_validated: bool,
}
fn evidence<'a>(item: &WatchItem, o: &'a WatchObservation) -> Result<&'a HistorySnapshot, String> {
    if o.account_id != item.account_id
        || o.watch_id != item.id
        || o.research.account_id != item.account_id
    {
        return Err("观察不属于该账户关注商品".into());
    }
    let mut histories = o
        .research
        .histories
        .iter()
        .filter(|h| h.request.item_key == item.item_key && h.account_id == item.account_id);
    let h = histories.next().ok_or("观察缺少商品历史")?;
    if histories.next().is_some() {
        return Err("观察商品历史不唯一".into());
    }
    Ok(h)
}
fn price(h: &HistorySnapshot) -> PriceEvidence {
    let day = h.analysis.reference_utc_day;
    let value = day
        .and_then(|day| h.analysis.daily_bars.iter().find(|b| b.utc_day == day))
        .and_then(|b| b.vwap.clone())
        .filter(|m| Some(&m.currency) == h.series.currency.as_ref() && m.minor > 0);
    PriceEvidence {
        reference_utc_day: day,
        price: value,
    }
}
fn metrics(h: &HistorySnapshot) -> [Option<i64>; 4] {
    let s = h.analysis.stability.as_ref();
    [
        s.and_then(|s| s.raw_sample_cv_bps)
            .and_then(|v| i64::try_from(v).ok()),
        s.and_then(|s| s.clean_sample_cv_bps)
            .and_then(|v| i64::try_from(v).ok()),
        s.and_then(|s| s.trend7.relative_slope_bps_per_day),
        h.analysis.risk.as_ref().and_then(|r| r.return_p10_bps),
    ]
}
pub fn compare(
    item: &WatchItem,
    base: &WatchBaseline,
    later: &WatchObservation,
) -> Result<WatchComparison, String> {
    if base.account_id != item.account_id || base.watch_id != item.id {
        return Err("基准不属于当前关注商品".into());
    }
    let old = &base.observation;
    let b = evidence(item, old)?;
    let h = evidence(item, later)?;
    if old.id == later.id
        || b.id == h.id
        || h.observed_at <= b.observed_at
        || later.research.created_at < old.research.created_at
    {
        return Err("请选择另一条包含更新采集历史的后续观察，重复引用原历史不算新样本".into());
    }
    let before = old
        .research
        .items
        .iter()
        .find(|i| i.item_key == item.item_key)
        .ok_or("基准缺少原判断")?
        .clone();
    let fixed = old.research.strategy.clone();
    let request = HoldingResearchRequest {
        strategy_id: fixed.strategy_id.clone(),
        revision: fixed.revision,
        history_snapshot_ids: vec![h.id.clone()],
    };
    let after = evaluate(
        &item.account_id,
        fixed.clone(),
        request,
        vec![h.clone()],
        later.research.created_at,
    )?
    .items
    .remove(0);
    let before_price = price(b);
    let after_price = price(h);
    let new_completed_day = before_price
        .reference_utc_day
        .zip(after_price.reference_utc_day)
        .is_some_and(|(b, h)| h > b);
    let same_measurement_basis = b.source == h.source
        && b.series.currency.is_some()
        && b.series.currency == h.series.currency
        && b.request.window_days == h.request.window_days
        && b.request.slippage_bps == h.request.slippage_bps
        && b.analysis
            .stability
            .as_ref()
            .zip(h.analysis.stability.as_ref())
            .is_some_and(|(b, h)| b.algorithm == h.algorithm && b.basis == h.basis);
    let historical_price_change_bps = if same_measurement_basis && new_completed_day {
        before_price
            .price
            .as_ref()
            .zip(after_price.price.as_ref())
            .and_then(|(b, h)| {
                i64::try_from(
                    (i128::from(h.minor) - i128::from(b.minor)) * 10000 / i128::from(b.minor),
                )
                .ok()
            })
    } else {
        None
    };
    let bm = metrics(b);
    let hm = metrics(h);
    let metrics = [
        "raw_cv_bps",
        "clean_cv_bps",
        "trend_bps_per_day",
        "calendar_p10_bps",
    ]
    .into_iter()
    .enumerate()
    .map(|(i, key)| MetricChange {
        key: key.into(),
        before: bm[i],
        after: hm[i],
        delta: if same_measurement_basis {
            bm[i].zip(hm[i]).and_then(|(b, h)| h.checked_sub(b))
        } else {
            None
        },
    })
    .collect();
    Ok(WatchComparison {
        item_key: item.item_key.clone(),
        baseline_observation_id: old.id.clone(),
        later_observation_id: later.id.clone(),
        baseline_fixed_at: base.fixed_at,
        baseline_collected_at: b.observed_at,
        later_collected_at: h.observed_at,
        baseline_research_at: old.research.created_at,
        later_research_at: later.research.created_at,
        collection_after_baseline: h.started_at > base.fixed_at,
        new_completed_day,
        same_measurement_basis,
        fixed_strategy: fixed,
        later_original_strategy: later.research.strategy.clone(),
        before,
        after_fixed_rules: after,
        before_price,
        after_price,
        historical_price_change_bps,
        metrics,
        limitations: vec![
            "后续规则沿用冻结版本，在后续原研究时间重算；原两份研究均不修改，不按今天时间判断过期"
                .into(),
            "日价来自Steam历史钱包市场口径，变化bps向零取整；不是可成交现金价、扣费收益或投资回报"
                .into(),
            "同日重采、币种/窗口/滑点/算法不一致不输出价格变化；不猜汇率或把缺失当零".into(),
            "冻结以前已采集的数据只能作为回顾；即使后续出现新完整日，也不代表样本外收益验证完成"
                .into(),
            "这里不固定全市场样本组或统计胜率，失败/未知观察同样保留，不创建采购或资金预留".into(),
        ],
        execution_allowed: false,
        returns_validated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        history::analyze,
        strategy::{ResearchObjective, ResearchStatus, StrategyRule, tests::fixture},
    };
    fn observation(at: u64, id: &str, strict: bool) -> WatchObservation {
        let (mut s, _, mut h, _) = fixture(at);
        s.draft.schema_version = 3;
        s.draft.objective = ResearchObjective::Holding;
        s.draft.rules.remove(0);
        if strict {
            for r in &mut s.draft.rules {
                if let StrategyRule::SampleCv { max_cv_bps, .. } = r {
                    *max_cv_bps = 1;
                }
            }
        } else {
            s.revision = 2;
        }
        h.id = format!("history-{id}");
        let req = HoldingResearchRequest {
            strategy_id: s.strategy_id.clone(),
            revision: s.revision,
            history_snapshot_ids: vec![h.id.clone()],
        };
        let research = evaluate(&h.account_id.clone(), s, req, vec![h], at).unwrap();
        WatchObservation {
            id: id.into(),
            watch_id: "watch".into(),
            account_id: research.account_id.clone(),
            recorded_at: at,
            research,
        }
    }
    pub(super) fn setup() -> (WatchItem, WatchBaseline, WatchObservation) {
        let at = 10000 * 86400 + 120;
        let old = observation(at, "old", true);
        let mut later = observation(at + 86400, "later", false);
        let h = &mut later.research.histories[0];
        for p in &mut h.series.points {
            p.price_major_1e4 *= 2;
        }
        h.analysis = analyze(&h.series, &h.request, h.observed_at).unwrap();
        let item = WatchItem {
            id: "watch".into(),
            account_id: old.account_id.clone(),
            app_id: 730,
            item_key: old.research.items[0].item_key.clone(),
            reason: "fixture only".into(),
            created_at: at,
        };
        let base = WatchBaseline {
            account_id: old.account_id.clone(),
            watch_id: "watch".into(),
            fixed_at: at + 60,
            observation: old,
        };
        (item, base, later)
    }
    #[test]
    fn comparison_uses_frozen_rules_and_later_as_of_time_without_rewriting_reports() {
        let (item, base, later) = setup();
        let prior = serde_json::to_value(&later).unwrap();
        let c = compare(&item, &base, &later).unwrap();
        assert_eq!(c.historical_price_change_bps, Some(10000));
        assert!(c.new_completed_day);
        assert!(c.collection_after_baseline);
        assert!(c.same_measurement_basis);
        assert_eq!(c.fixed_strategy.revision, 1);
        assert_eq!(c.later_original_strategy.revision, 2);
        assert_eq!(later.research.items[0].status, ResearchStatus::Pass);
        assert_eq!(c.before.status, ResearchStatus::Fail);
        assert_eq!(c.after_fixed_rules.status, ResearchStatus::Fail);
        assert_eq!(serde_json::to_value(&later).unwrap(), prior);
        assert!(!c.execution_allowed);
        assert!(!c.returns_validated);
    }
    #[test]
    fn comparison_rejects_reused_or_reversed_history_and_foreign_observations() {
        let (item, base, later) = setup();
        assert!(compare(&item, &base, &base.observation).is_err());
        for mode in 0..4 {
            let mut o = later.clone();
            match mode {
                0 => o.account_id = "other".into(),
                1 => o.watch_id = "other".into(),
                2 => o.research.histories[0].id = base.observation.research.histories[0].id.clone(),
                _ => {
                    o.research.histories[0].observed_at =
                        base.observation.research.histories[0].observed_at - 1
                }
            };
            assert!(compare(&item, &base, &o).is_err());
        }
    }
    #[test]
    fn comparison_never_invents_zero_returns_for_missing_or_incompatible_evidence() {
        let (item, base, later) = setup();
        for mode in 0..5 {
            let mut o = later.clone();
            let h = &mut o.research.histories[0];
            match mode {
                0 => h.series.currency = Some("UAH".to_string().try_into().unwrap()),
                1 => {
                    h.analysis.reference_utc_day = base.observation.research.histories[0]
                        .analysis
                        .reference_utc_day
                }
                2 => h.request.window_days = 90,
                3 => h.analysis.daily_bars.clear(),
                _ => h.series.currency = None,
            };
            let c = compare(&item, &base, &o).unwrap();
            assert_eq!(c.historical_price_change_bps, None);
            if matches!(mode, 0 | 2 | 4) {
                assert!(!c.same_measurement_basis);
                assert!(c.metrics.iter().all(|m| m.delta.is_none()));
            }
        }
    }
    #[test]
    fn comparison_distinguishes_retrospective_data_and_stale_unknown_rules() {
        let (item, mut base, mut later) = setup();
        base.fixed_at = later.research.histories[0].observed_at + 1;
        later.research.created_at += 7200;
        let c = compare(&item, &base, &later).unwrap();
        assert!(!c.collection_after_baseline);
        assert_eq!(c.after_fixed_rules.status, ResearchStatus::Unknown);
    }
}
