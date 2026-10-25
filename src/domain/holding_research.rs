//! Independent holding observation over explicitly selected historical evidence; no budget plan.
use super::{
    history::HistorySnapshot,
    strategy::{
        ItemDecision, ResearchObjective, ResearchStatus, StrategyRule, StrategyVersion, aggregate,
        decision,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldingResearchRequest {
    pub strategy_id: String,
    pub revision: u32,
    pub history_snapshot_ids: Vec<String>,
}
impl HoldingResearchRequest {
    pub fn validate(&self) -> Result<(), String> {
        let id = |s: &str| !s.is_empty() && s.len() <= 64 && !s.chars().any(char::is_control);
        let mut seen = HashSet::new();
        if !id(&self.strategy_id)
            || self.revision == 0
            || self.history_snapshot_ids.is_empty()
            || self.history_snapshot_ids.len() > 20
            || self
                .history_snapshot_ids
                .iter()
                .any(|s| !id(s) || !seen.insert(s))
        {
            return Err("独立持有研究需策略版本及1–20份唯一历史快照".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HoldingResearchReport {
    pub id: String,
    pub account_id: String,
    pub created_at: u64,
    pub algorithm: String,
    pub input_scope: String,
    pub status: ResearchStatus,
    pub preview_only: bool,
    pub execution_allowed: bool,
    pub holding_value_established: bool,
    pub request: HoldingResearchRequest,
    pub strategy: StrategyVersion,
    pub histories: Vec<HistorySnapshot>,
    pub items: Vec<ItemDecision>,
    pub unchecked_rules: Vec<String>,
    pub limitations: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HoldingResearchSummary {
    pub id: String,
    pub created_at: u64,
    pub strategy_name: String,
    pub status: ResearchStatus,
    pub item_count: usize,
}
impl From<HoldingResearchReport> for HoldingResearchSummary {
    fn from(r: HoldingResearchReport) -> Self {
        Self {
            id: r.id,
            created_at: r.created_at,
            strategy_name: r.strategy.draft.name,
            status: r.status,
            item_count: r.items.len(),
        }
    }
}
pub fn evaluate(
    account: &str,
    strategy: StrategyVersion,
    request: HoldingResearchRequest,
    histories: Vec<HistorySnapshot>,
    at: u64,
) -> Result<HoldingResearchReport, String> {
    request.validate()?;
    strategy.draft.validate()?;
    if account.is_empty()
        || strategy.strategy_id != request.strategy_id
        || strategy.revision != request.revision
        || strategy.draft.objective != ResearchObjective::Holding
    {
        return Err("独立持有研究仅接受准确引用的持有观察策略".into());
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    if histories.len() != request.history_snapshot_ids.len()
        || histories.iter().any(|h| {
            let binding = if h.request.manual_item {
                "manual_market_name_and_request_parameters"
            } else if h.request.inventory_snapshot_id.is_some() {
                "inventory_snapshot_and_request_parameters"
            } else {
                "scan_candidate_and_request_parameters"
            };
            h.account_id != account
                || !request.history_snapshot_ids.contains(&h.id)
                || !ids.insert(&h.id)
                || !names.insert(&h.request.item_key)
                || h.source != "steam_authenticated_pricehistory"
                || h.identity_binding != binding
                || h.request.validate().is_err()
        })
    {
        return Err(
            "历史需属于本账户、来源绑定正确，每个商品仅一份；不能引用重复或外部账户证据".into(),
        );
    }
    let mut items = Vec::new();
    for h in &histories {
        let mut covered = false;
        let mut decisions = Vec::new();
        for rule in &strategy.draft.rules {
            let result = match rule {
                StrategyRule::PositionCapacity => decision(
                    rule.key(),
                    None,
                    "独立商品研究未引用完整持仓与购入数量，不能按零持仓通过",
                    Value::Null,
                ),
                _ => super::strategy_history::history_rule(
                    rule,
                    Some(h),
                    &mut covered,
                    at,
                    &h.identity_binding,
                ),
            };
            decisions.push(result);
        }
        items.push(ItemDecision {
            item_key: h.request.item_key.clone(),
            status: aggregate(decisions.iter().map(|d| d.status)),
            decisions,
            review: None,
            exit_valuation: None,
        });
    }
    let keys = [
        "position_capacity",
        "history_coverage",
        "sample_cv",
        "trend7",
        "calendar_p10",
    ];
    let unchecked_rules = keys
        .iter()
        .filter(|k| !strategy.draft.rules.iter().any(|r| r.key() == **k))
        .map(|k| (*k).into())
        .collect();
    Ok(HoldingResearchReport {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: account.into(),
        created_at: at,
        algorithm: "independent_holding_research_v1".into(),
        input_scope: "explicit_history_snapshots".into(),
        status: aggregate(items.iter().map(|i| i.status)),
        preview_only: true,
        execution_allowed: false,
        holding_value_established: false,
        request,
        strategy,
        histories,
        items,
        unchecked_rules,
        limitations: vec![
            "本次范围仅为明确选择的历史商品；不依赖榜单/预算过滤，也不代表全市场覆盖".into(),
            "不含实时现金报价或购入成本，不能判断现金低估、折扣、可买数量或未来收益".into(),
            "指定市场名称按请求参数绑定；不证明当前持有、平台签名确认或特殊磨损/模板溢价".into(),
            "仓位与买入数量未建模；启用仓位规则时保持未知，未启用规则不属于已检查范围".into(),
            "尚未进行样本外收益验证；重叠七日配对不是独立试验，不创建订单或资金预留".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::strategy::tests::fixture;
    fn setup(at: u64) -> (StrategyVersion, HistorySnapshot, HoldingResearchRequest) {
        let (mut s, _, h, _) = fixture(at);
        s.draft.schema_version = 3;
        s.draft.objective = ResearchObjective::Holding;
        s.draft.rules.remove(0);
        let r = HoldingResearchRequest {
            strategy_id: s.strategy_id.clone(),
            revision: s.revision,
            history_snapshot_ids: vec![h.id.clone()],
        };
        (s, h, r)
    }
    #[test]
    fn independent_rules_match_existing_rules_without_a_plan_or_wallet_value() {
        let at = 10000 * 86400 + 120;
        let (mut s, p, h, r) = fixture(at);
        s.draft.schema_version = 3;
        s.draft.objective = ResearchObjective::Holding;
        s.draft.rules.remove(0);
        let planned =
            crate::domain::strategy::evaluate(s.clone(), p, vec![h.clone()], r, at).unwrap();
        let request = HoldingResearchRequest {
            strategy_id: s.strategy_id.clone(),
            revision: s.revision,
            history_snapshot_ids: vec![h.id.clone()],
        };
        let independent = evaluate(&h.account_id, s, request, vec![h.clone()], at).unwrap();
        let expected: Vec<_> = planned.items[0]
            .decisions
            .iter()
            .filter(|d| d.module != "quote_freshness")
            .collect();
        assert_eq!(
            serde_json::to_value(expected).unwrap(),
            serde_json::to_value(&independent.items[0].decisions).unwrap()
        );
        assert_eq!(independent.status, ResearchStatus::Pass);
        assert!(!independent.execution_allowed);
        assert!(!independent.holding_value_established);
        assert!(
            serde_json::to_value(independent)
                .unwrap()
                .get("plan")
                .is_none()
        );
    }
    #[test]
    fn manual_and_inventory_histories_are_independent_inputs() {
        let at = 10000 * 86400 + 120;
        for inventory in [false, true] {
            let (s, mut h, r) = setup(at);
            h.request.scan_id.clear();
            if inventory {
                h.request.inventory_snapshot_id = Some("observed".into());
                h.identity_binding = "inventory_snapshot_and_request_parameters".into();
            } else {
                h.request.manual_item = true;
                h.identity_binding = "manual_market_name_and_request_parameters".into();
            }
            assert_eq!(
                evaluate(&h.account_id.clone(), s, r, vec![h], at)
                    .unwrap()
                    .status,
                ResearchStatus::Pass
            );
        }
    }
    #[test]
    fn independent_bindings_reject_wallet_cross_account_duplicate_items_and_forged_origins() {
        let at = 10000 * 86400 + 120;
        let (s, h, r) = setup(at);
        assert!(evaluate("other", s.clone(), r.clone(), vec![h.clone()], at).is_err());
        let mut wallet = s.clone();
        wallet.draft.objective = ResearchObjective::WalletConversion;
        assert!(evaluate(&h.account_id, wallet, r.clone(), vec![h.clone()], at).is_err());
        let mut bad = h.clone();
        bad.identity_binding = "manual_market_name_and_request_parameters".into();
        assert!(evaluate(&h.account_id, s.clone(), r.clone(), vec![bad], at).is_err());
        let mut other = h.clone();
        other.id = "second".into();
        let mut many = r;
        many.history_snapshot_ids.push(other.id.clone());
        assert!(evaluate(&h.account_id, s, many, vec![h.clone(), other], at).is_err());
    }
    #[test]
    fn missing_position_and_stale_or_unknown_history_never_become_pass() {
        let at = 10000 * 86400 + 120;
        let (s, h, r) = setup(at);
        for change in [0, 1, 2] {
            let mut s = s.clone();
            let mut h = h.clone();
            match change {
                0 => s.draft.rules.insert(0, StrategyRule::PositionCapacity),
                1 => h.observed_at = at - 90000,
                _ => h.series.currency = None,
            };
            assert_eq!(
                evaluate(&h.account_id.clone(), s, r.clone(), vec![h], at)
                    .unwrap()
                    .status,
                ResearchStatus::Unknown
            );
        }
    }
    #[test]
    fn independent_request_is_bounded_nonempty_and_unique() {
        let (_, _, mut r) = setup(10000 * 86400 + 120);
        r.history_snapshot_ids.clear();
        assert!(r.validate().is_err());
        r.history_snapshot_ids = vec!["same".into(); 2];
        assert!(r.validate().is_err());
        r.history_snapshot_ids = (0..21).map(|i| format!("id{i}")).collect();
        assert!(r.validate().is_err());
    }
}
