//! Side-by-side saved evidence, not a new scoring or recommendation engine.
use super::{holding_research::HoldingResearchReport, money::Money, strategy::ResearchStatus};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CandidateRow {
    pub item_key: String,
    pub status: ResearchStatus,
    pub history_id: Option<String>,
    pub observed_at: Option<u64>,
    pub from_utc_day: Option<u64>,
    pub until_utc_day_exclusive: Option<u64>,
    pub reference_price: Option<Money>,
    pub reference_utc_day: Option<u64>,
    pub raw_cv_bps: Option<u64>,
    pub clean_cv_bps: Option<u64>,
    pub trend_bps_per_day: Option<i64>,
    pub calendar_p10_bps: Option<i64>,
    pub usable_days: Option<usize>,
    pub missing_days: Option<u32>,
    pub comparison_group: Option<usize>,
    pub failed_rules: Vec<String>,
    pub unknown_rules: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct CandidateComparison {
    pub report_id: String,
    pub strategy_id: String,
    pub revision: u32,
    pub research_at: u64,
    pub rows: Vec<CandidateRow>,
    pub limitations: Vec<String>,
    pub execution_allowed: bool,
}
pub fn summarize(report: &HoldingResearchReport) -> CandidateComparison {
    let mut bases = Vec::new();
    let rows = report
        .items
        .iter()
        .map(|item| {
            let mut matches = report.histories.iter().filter(|h| {
                h.request.item_key == item.item_key && h.account_id == report.account_id
            });
            let history = matches.next().filter(|_| matches.next().is_none());
            let stability = history.and_then(|h| h.analysis.stability.as_ref());
            let group = history.zip(stability).and_then(|(h, s)| {
                h.series.currency.as_ref().map(|currency| {
                    let basis = (
                        h.source.clone(),
                        currency.clone(),
                        h.series.from_utc_day,
                        h.series.until_utc_day_exclusive,
                        h.request.window_days,
                        h.request.slippage_bps,
                        s.algorithm.clone(),
                        s.basis.clone(),
                    );
                    if let Some(index) = bases.iter().position(|b| b == &basis) {
                        index + 1
                    } else {
                        bases.push(basis);
                        bases.len()
                    }
                })
            });
            CandidateRow {
                item_key: item.item_key.clone(),
                status: item.status,
                history_id: history.map(|h| h.id.clone()),
                observed_at: history.map(|h| h.observed_at),
                from_utc_day: history.map(|h| h.series.from_utc_day),
                until_utc_day_exclusive: history.map(|h| h.series.until_utc_day_exclusive),
                reference_utc_day: history.and_then(|h| h.analysis.reference_utc_day),
                reference_price: history.and_then(|h| {
                    h.analysis
                        .reference_utc_day
                        .and_then(|day| h.analysis.daily_bars.iter().find(|b| b.utc_day == day))
                        .and_then(|b| b.vwap.clone())
                        .filter(|p| Some(&p.currency) == h.series.currency.as_ref() && p.minor > 0)
                }),
                raw_cv_bps: stability.and_then(|s| s.raw_sample_cv_bps),
                clean_cv_bps: stability.and_then(|s| s.clean_sample_cv_bps),
                trend_bps_per_day: stability.and_then(|s| s.trend7.relative_slope_bps_per_day),
                calendar_p10_bps: history
                    .and_then(|h| h.analysis.risk.as_ref())
                    .and_then(|r| r.return_p10_bps),
                usable_days: stability.map(|s| s.usable_days),
                missing_days: history.map(|h| h.analysis.missing_calendar_days),
                comparison_group: group,
                failed_rules: item
                    .decisions
                    .iter()
                    .filter(|d| d.status == ResearchStatus::Fail)
                    .map(|d| d.module.clone())
                    .collect(),
                unknown_rules: item
                    .decisions
                    .iter()
                    .filter(|d| d.status == ResearchStatus::Unknown)
                    .map(|d| d.module.clone())
                    .collect(),
            }
        })
        .collect();
    CandidateComparison {
        report_id: report.id.clone(), strategy_id: report.strategy.strategy_id.clone(), revision: report.strategy.revision, research_at: report.created_at, rows,
        limitations: vec![
            "仅显式选择的商品，按原研究顺序保留满足、未满足、未知；没有全市场覆盖、综合评分或买入排名".into(),
            "同组表示来源、明确币种、日期范围、窗口、滑点、算法和统计口径相同；不保证同时采集或价格可成交，跨组不要直接比较价格或指标高低".into(),
            "沿用保存时的指标和规则结论，不按今天重算；显示原采集时间，历史日价不是当前现金买价，P10不是未来收益预测".into(),
            "未启用规则和逐项依据见原研究；未计算费用收益、现金公允价值或回测胜率".into(),
        ], execution_allowed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        holding_research::{HoldingResearchRequest, evaluate},
        strategy::{ResearchObjective, tests::fixture},
    };
    fn report() -> HoldingResearchReport {
        let (mut s, _, h, _) = fixture(10000 * 86400 + 120);
        s.draft.schema_version = 3;
        s.draft.objective = ResearchObjective::Holding;
        s.draft.rules.remove(0);
        let mut histories = vec![h.clone()];
        for (id, name) in [("second", "second item"), ("third", "third item")] {
            let mut next = h.clone();
            next.id = id.into();
            next.request.item_key = name.into();
            histories.push(next);
        }
        evaluate(
            &h.account_id,
            s.clone(),
            HoldingResearchRequest {
                strategy_id: s.strategy_id,
                revision: s.revision,
                history_snapshot_ids: histories.iter().map(|h| h.id.clone()).collect(),
            },
            histories,
            10000 * 86400 + 120,
        )
        .unwrap()
    }
    #[test]
    fn candidate_comparison_preserves_all_outcomes_order_and_saved_evidence() {
        let mut r = report();
        for (item, status) in r.items.iter_mut().zip([
            ResearchStatus::Pass,
            ResearchStatus::Fail,
            ResearchStatus::Unknown,
        ]) {
            item.status = status;
        }
        let original = serde_json::to_value(&r).unwrap();
        let c = summarize(&r);
        assert_eq!(c.rows.len(), 3);
        assert_eq!(c.rows[0].status, ResearchStatus::Pass);
        assert_eq!(c.rows[1].status, ResearchStatus::Fail);
        assert_eq!(c.rows[2].status, ResearchStatus::Unknown);
        assert!(c.rows.iter().all(|r| r.comparison_group == Some(1)));
        assert_eq!(c.rows[1].item_key, "second item");
        assert!(!c.execution_allowed);
        assert_eq!(serde_json::to_value(&r).unwrap(), original);
    }
    #[test]
    fn candidate_comparison_separates_incompatible_or_missing_evidence_without_zero_fallback() {
        for mode in 0..5 {
            let mut r = report();
            let h = &mut r.histories[1];
            match mode {
                0 => h.series.currency = Some("UAH".to_string().try_into().unwrap()),
                1 => h.series.from_utc_day -= 1,
                2 => h.request.slippage_bps += 1,
                3 => h.analysis.stability.as_mut().unwrap().algorithm = "other".into(),
                _ => h.series.currency = None,
            }
            let c = summarize(&r);
            assert_ne!(c.rows[0].comparison_group, c.rows[1].comparison_group);
            if matches!(mode, 0 | 4) {
                assert!(c.rows[1].reference_price.is_none());
            }
        }
        let mut r = report();
        r.histories.remove(1);
        let c = summarize(&r);
        assert!(c.rows[1].history_id.is_none());
        assert!(c.rows[1].raw_cv_bps.is_none());
        assert!(c.rows[1].comparison_group.is_none());
        let mut r = report();
        r.histories.push(r.histories[0].clone());
        assert!(summarize(&r).rows[0].history_id.is_none());
    }
}
