//! Sell policy research over reported cost and explicitly referenced immutable evidence.
use super::{
    accounts::{Platform, SessionStatus},
    exit_valuation::ExitValuation,
    inventory::InventorySnapshot,
    ledger::{AssetStage, PurchaseLot},
    preflight::AccountContext,
    strategy::{ResearchStatus, RuleDecision, StrategyEvaluation, evaluate_with_order_books},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrendGate {
    pub min_points: u32,
    pub min_r_squared_bps: u32,
    pub rising_above_bps_per_day: i64,
}
impl TrendGate {
    fn validate(&self) -> bool {
        (3..=7).contains(&self.min_points)
            && self.min_r_squared_bps <= 10000
            && (-10000..=10000).contains(&self.rising_above_bps_per_day)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueBasis {
    Normal,
    Stressed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum SellPolicy {
    Pause,
    Immediate,
    TrendWait {
        gate: TrendGate,
    },
    ProfitProtect {
        min_wallet_return_bps: i64,
        basis: ValueBasis,
        trend: Option<TrendGate>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleResearchRequest {
    pub lot_id: String,
    pub asset_id: String,
    pub evaluation_id: String,
    pub inventory_snapshot_id: String,
    pub max_inventory_age_secs: u64,
    pub policy: SellPolicy,
}
impl SaleResearchRequest {
    pub fn validate(&self) -> Result<(), String> {
        if [
            &self.lot_id,
            &self.asset_id,
            &self.evaluation_id,
            &self.inventory_snapshot_id,
        ]
        .iter()
        .any(|s| !super::ledger::text_id(s, 128))
            || !(1..=300).contains(&self.max_inventory_age_secs)
        {
            return Err("出售研究需批次/资产/研究/库存引用及1–300秒观察期限".into());
        }
        let valid = match &self.policy {
            SellPolicy::TrendWait { gate } => gate.validate(),
            SellPolicy::ProfitProtect {
                min_wallet_return_bps,
                trend,
                ..
            } => {
                (-10000..=100000).contains(min_wallet_return_bps)
                    && trend.as_ref().is_none_or(TrendGate::validate)
            }
            _ => true,
        };
        if !valid {
            return Err("出售收益/趋势阈值越界".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaleAction {
    Hold,
    ReviewListing,
    Unknown,
    Blocked,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaleResearch {
    pub id: String,
    pub account_id: String,
    pub created_at: u64,
    pub algorithm: String,
    pub action: SaleAction,
    pub execution_allowed: bool,
    pub request: SaleResearchRequest,
    pub lot: PurchaseLot,
    pub inventory: InventorySnapshot,
    pub context: AccountContext,
    pub rechecked_evaluation: StrategyEvaluation,
    pub unit_cost_cents: u64,
    pub estimated_wallet_surplus_cny_cents: Option<i64>,
    pub estimated_wallet_return_bps: Option<i64>,
    pub valuation: Option<ExitValuation>,
    pub checks: Vec<RuleDecision>,
    pub execution_blockers: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaleResearchSummary {
    pub id: String,
    pub account_id: String,
    pub created_at: u64,
    pub lot_id: String,
    pub asset_id: String,
    pub action: SaleAction,
}
impl From<SaleResearch> for SaleResearchSummary {
    fn from(v: SaleResearch) -> Self {
        Self {
            id: v.id,
            account_id: v.account_id,
            created_at: v.created_at,
            lot_id: v.request.lot_id,
            asset_id: v.request.asset_id,
            action: v.action,
        }
    }
}
fn check(
    checks: &mut Vec<RuleDecision>,
    key: &str,
    result: Option<bool>,
    reason: &str,
    value: serde_json::Value,
) {
    checks.push(RuleDecision {
        module: key.into(),
        status: match result {
            Some(true) => ResearchStatus::Pass,
            Some(false) => ResearchStatus::Fail,
            None => ResearchStatus::Unknown,
        },
        reason: reason.into(),
        observed: value,
    });
}
pub fn evaluate(
    request: SaleResearchRequest,
    lot: PurchaseLot,
    source: StrategyEvaluation,
    inventory: InventorySnapshot,
    context: AccountContext,
    at: u64,
) -> Result<SaleResearch, String> {
    request.validate()?;
    if request.lot_id != lot.id
        || request.evaluation_id != source.id
        || request.inventory_snapshot_id != inventory.id
        || inventory.account_id != lot.account_id
        || context.account.id != lot.account_id
        || inventory.steam_id != context.account.steam_id
        || source.request.account_id.as_deref() != Some(&lot.account_id)
    {
        return Err("出售研究的账户或证据引用不一致".into());
    }
    let asset = lot
        .assets
        .iter()
        .find(|a| a.asset.asset_id == request.asset_id)
        .ok_or("出售资产不属于此批次")?;
    if !source
        .plan
        .lines
        .iter()
        .any(|l| l.candidate.item.key == lot.request.item_key)
    {
        return Err("研究证据未覆盖批次商品".into());
    }
    let rechecked = evaluate_with_order_books(
        source.strategy,
        source.plan,
        source.histories,
        source.order_books,
        source.request,
        at,
    )?;
    let mut checks = vec![];
    check(
        &mut checks,
        "asset_stage",
        Some(asset.stage == AssetStage::Held),
        "只有台账持有态可研究新挂牌；已挂/已售不重复建议",
        json!({"stage":asset.stage,"lot_revision":lot.revision}),
    );
    let fresh =
        inventory.observed_at <= at && at - inventory.observed_at <= request.max_inventory_age_secs;
    check(
        &mut checks,
        "inventory_age",
        Some(fresh),
        "使用显式库存观察原时间，不通过读取刷新",
        json!({"observed_at":inventory.observed_at,"max_age_secs":request.max_inventory_age_secs}),
    );
    let observed = inventory
        .assets
        .iter()
        .find(|a| a.asset_id == request.asset_id);
    let identity = observed.map(|a| {
        a.class_id == asset.asset.class_id
            && a.instance_id == asset.asset.instance_id
            && a.market_hash_name.as_deref() == Some(&lot.request.item_key)
            && a.context_id == "2"
            && a.amount == 1
    });
    check(
        &mut checks,
        "asset_identity",
        identity.or_else(|| inventory.coverage_complete().then_some(false)),
        "按账户/资产/class/instance/商品精确关联；部分观察未出现是未知",
        json!(observed),
    );
    check(
        &mut checks,
        "reported_marketable",
        observed.map(|a| a.reported_marketable && a.reported_transferable()),
        "仅为平台观察标志，不推算七天解锁或替代真实挂牌响应",
        json!(observed),
    );
    let steam = context
        .sessions
        .iter()
        .find(|s| s.platform == Platform::Steam);
    let valid = steam.map(|s| {
        s.status == SessionStatus::Valid
            && s.subject.as_deref() == Some(context.account.steam_id.as_str())
            && s.retry_at <= at
            && s.checked_at.is_some_and(|t| t <= at && at - t <= 300)
    });
    check(
        &mut checks,
        "steam_session",
        valid,
        "核对当前本地Steam身份/300秒元数据及冷却，不读取凭据库",
        json!(steam),
    );
    let history = rechecked
        .histories
        .iter()
        .find(|h| h.request.item_key == lot.request.item_key);
    check(
        &mut checks,
        "history_generation",
        history
            .zip(steam)
            .map(|(h, s)| h.credential_generation == s.generation),
        "历史必须属于当前凭据代次；轮换后重新获取证据",
        json!({"history_id":history.map(|h|&h.id)}),
    );
    let valuation = rechecked
        .items
        .iter()
        .find(|i| i.item_key == lot.request.item_key)
        .and_then(|i| i.exit_valuation.clone());
    check(
        &mut checks,
        "exit_evidence",
        valuation.as_ref().map(|_| true),
        "复用v2历史覆盖/壁垒/P10/原币费用/FX；不把购入折算率的通过/失败当出售规则",
        json!({"evaluation_algorithm":rechecked.algorithm,"item_key":lot.request.item_key}),
    );
    let blocked = checks.iter().any(|c| c.status == ResearchStatus::Fail);
    let unknown = checks.iter().any(|c| c.status == ResearchStatus::Unknown);
    let cost = asset.cost_cents;
    let basis = match &request.policy {
        SellPolicy::ProfitProtect { basis, .. } => basis.clone(),
        _ => ValueBasis::Normal,
    };
    let net = valuation.as_ref().map(|v| match basis {
        ValueBasis::Normal => v.normal_net_cny.minor,
        ValueBasis::Stressed => v.stressed_net_cny.minor,
    });
    let surplus = net.map(|n| n as i64 - cost as i64);
    let returns = surplus.map(|v| (i128::from(v) * 10000 / i128::from(cost)) as i64);
    let mut wait = matches!(request.policy, SellPolicy::Pause);
    let mut policy_unknown = false;
    let trend = match &request.policy {
        SellPolicy::TrendWait { gate } => Some(gate),
        SellPolicy::ProfitProtect { trend, .. } => trend.as_ref(),
        _ => None,
    };
    if let Some(gate) = trend {
        let t = history
            .and_then(|h| h.analysis.stability.as_ref())
            .map(|s| &s.trend7);
        let rising = t
            .filter(|t| t.usable_days >= gate.min_points as usize)
            .and_then(|t| t.r_squared_bps.zip(t.relative_slope_bps_per_day))
            .filter(|(r, _)| *r >= gate.min_r_squared_bps)
            .map(|(_, s)| s > gate.rising_above_bps_per_day);
        check(
            &mut checks,
            "trend_wait",
            rising.map(|r| !r),
            "可靠上升趋势时等待；缺点数/斜率/R²保持未知，不当平稳",
            json!({"gate":gate,"trend":t}),
        );
        wait |= rising == Some(true);
        policy_unknown |= rising.is_none();
    }
    if let SellPolicy::ProfitProtect {
        min_wallet_return_bps,
        ..
    } = request.policy
    {
        let meets = net.map(|n| {
            u128::from(n) * 10000 >= u128::from(cost) * (10000 + min_wallet_return_bps) as u128
        });
        check(
            &mut checks,
            "wallet_return",
            meets,
            "按分摊成本与当前净钱包估值精确比较；这是钱包价值差额，不是可提现现金利润",
            json!({"cost_cents":cost,"net_cny_cents":net,"min_wallet_return_bps":min_wallet_return_bps,"basis":basis}),
        );
        wait |= meets == Some(false);
        policy_unknown |= meets.is_none();
    }
    let action = if matches!(request.policy, SellPolicy::Pause) {
        SaleAction::Hold
    } else if blocked || net == Some(0) {
        SaleAction::Blocked
    } else if unknown || policy_unknown {
        SaleAction::Unknown
    } else if wait {
        SaleAction::Hold
    } else {
        SaleAction::ReviewListing
    };
    Ok(SaleResearch {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: lot.account_id.clone(),
        created_at: at,
        algorithm: "reported_cost_sell_policy_v1".into(),
        action,
        execution_allowed: false,
        request,
        lot,
        inventory,
        context,
        rechecked_evaluation: rechecked,
        unit_cost_cents: cost,
        estimated_wallet_surplus_cny_cents: surplus,
        estimated_wallet_return_bps: returns,
        valuation,
        checks,
        execution_blockers: vec![
            "采购/挂牌/成交台账仍为手动未核验凭据".into(),
            "需重新核验实际资产、解锁、原币费用与平台权限".into(),
            "没有接入挂牌/确认/撤单和持久化幂等订单".into(),
        ],
    })
}
#[cfg(test)]
#[path = "sale_research_tests.rs"]
pub(crate) mod tests;
