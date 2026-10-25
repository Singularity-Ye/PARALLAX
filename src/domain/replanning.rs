//! Selection provenance only. Allocation remains in planning; old pass is not reauthorization.
use super::{
    planning::PlanRequest,
    strategy::{ItemDecision, ResearchStatus, StrategyEvaluation, StrategyVersion},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplanRequest {
    pub evaluation_id: String,
    pub request: PlanRequest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResearchOrigin {
    pub evaluation_id: String,
    pub parent_plan_id: String,
    pub account_id: Option<String>,
    pub research_at: u64,
    pub strategy: StrategyVersion,
    pub selected_decisions: Vec<ItemDecision>,
    pub new_plan_research_required: bool,
}
pub fn origin(
    source: &StrategyEvaluation,
    request: &PlanRequest,
) -> Result<ResearchOrigin, String> {
    request.validate()?;
    if source.plan.request.scan_id != request.scan_id {
        return Err("重新规划须沿用原研究的扫描来源；新扫描需重新研究后选择".into());
    }
    let keys = request
        .selected_item_keys
        .as_ref()
        .ok_or("重新规划必须明确选择商品")?;
    let mut selected = Vec::new();
    for key in keys {
        let matches = source
            .items
            .iter()
            .filter(|i| &i.item_key == key)
            .collect::<Vec<_>>();
        if matches.len() != 1 || matches[0].status != ResearchStatus::Pass {
            return Err(
                "只能选择原研究中逐项满足的商品；未满足/未知或未研究商品不能自动纳入".into(),
            );
        }
        selected.push(matches[0].clone());
    }
    for account in [
        request
            .position_snapshot
            .as_ref()
            .map(|s| s.account_id.as_str()),
        request.buff_account_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if source.request.account_id.as_deref() != Some(account) {
            return Err("新仓位/报价账户必须与原研究账户一致".into());
        }
    }
    Ok(ResearchOrigin {
        evaluation_id: source.id.clone(),
        parent_plan_id: source.plan.id.clone(),
        account_id: source.request.account_id.clone(),
        research_at: source.created_at,
        strategy: source.strategy.clone(),
        selected_decisions: selected,
        new_plan_research_required: true,
    })
}
