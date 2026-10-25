//! Scope and provenance of a rule match, never a fair-value or executable-price claim.
use super::{
    planning::{PlanLine, PlanPreview},
    selection::SourceKind,
    strategy::{ResearchObjective, StrategyDraft},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionReview {
    pub objective: ResearchObjective,
    pub conclusion_scope: String,
    pub cash_price_basis: String,
    pub price_evidence_at: u64,
    pub cash_snapshot_id: Option<String>,
    pub unchecked_rules: Vec<String>,
    pub unassessed: Vec<String>,
    pub manual_checks: Vec<String>,
    pub holding_value_established: bool,
}

pub fn review(draft: &StrategyDraft, plan: &PlanPreview, line: &PlanLine) -> DecisionReview {
    let item = &line.candidate.item;
    let buff = plan.buff_valuations.iter().find(|v| v.item_key == item.key);
    let (basis, at, id) = match (plan.source, buff) {
        (SourceKind::Demo, _) => ("demo", item.observed_at, None),
        (_, Some(v)) => (
            "buff_visible_sell_snapshot",
            v.observed_at,
            Some(v.snapshot_id.clone()),
        ),
        _ => ("steamdt_reference", item.observed_at, None),
    };
    let keys: &[&str] = if draft.objective == ResearchObjective::Holding {
        &[
            "position_capacity",
            "history_coverage",
            "sample_cv",
            "trend7",
            "calendar_p10",
        ]
    } else {
        &[
            "strict_ratio",
            "position_capacity",
            "history_coverage",
            "sample_cv",
            "trend7",
            "calendar_p10",
            "stress_exit_ratio",
        ]
    };
    DecisionReview {
        objective: draft.objective,
        conclusion_scope: "configured_rules_on_saved_plan_only".into(),
        cash_price_basis: basis.into(),
        price_evidence_at: at,
        cash_snapshot_id: id,
        unchecked_rules: keys.iter().filter(|key| !draft.rules.iter().any(|r| r.key() == **key)).map(|key| (*key).into()).collect(),
        unassessed: vec![
            "当前可成交现金价格与数量未确认".into(),
            "现金公允价值与持有期收益尚未建立".into(),
            "组合相关性、类别集中度和供应事件未完整评估".into(),
            "规则未经过样本外收益验证；重叠七日配对不是独立试验".into(),
            "仅研究已分配预算的条目；上游仍受SteamDT榜单与折算率预筛/分配影响，不代表全市场独立选品".into(),
        ],
        manual_checks: vec![
            "核对商品版本、磨损、模板与特殊溢价是否匹配".into(),
            "确认实际平台当前价格、数量、全部费用与交易限制".into(),
            "核对已有持仓、未决采购和可承受的资金占用".into(),
        ],
        holding_value_established: false,
    }
}
