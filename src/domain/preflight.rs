//! A durable inspection of saved evidence, never an execution grant or a price refresh.
use super::{
    accounts::{Account, Platform, SessionStatus, SessionView},
    selection::SourceKind,
    strategy::{ResearchStatus, RuleDecision, StrategyEvaluation, evaluate_with_order_books},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub item_key: String,
    pub quantity: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreflightRequest {
    pub evaluation_id: String,
    pub account_id: Option<String>,
    pub selections: Vec<Selection>,
    /// Tighter than, and never an extension of, the plan's original quote age.
    pub max_quote_age_secs: u64,
}
impl PreflightRequest {
    pub fn validate(&self) -> Result<(), String> {
        let id =
            |s: &str| !s.trim().is_empty() && s.len() <= 128 && !s.chars().any(char::is_control);
        let mut keys = std::collections::HashSet::new();
        if !id(&self.evaluation_id)
            || self.account_id.as_deref().is_some_and(|s| !id(s))
            || self.selections.is_empty()
            || self.selections.len() > 100
            || !(1..=300).contains(&self.max_quote_age_secs)
            || self
                .selections
                .iter()
                .any(|s| !id(&s.item_key) || s.quantity == 0 || !keys.insert(&s.item_key))
        {
            return Err("需研究记录、1–100件唯一商品及正整数数量；报价期限为1–300秒".into());
        }
        Ok(())
    }
}
/// One database read transaction. No credential-store keys, cookies or platform calls.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountContext {
    pub account: Account,
    pub sessions: Vec<SessionView>,
    pub buff_retry_at: u64,
    pub buff_blocked: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preflight {
    pub id: String,
    pub created_at: u64,
    pub algorithm: String,
    pub mode: String,
    pub status: ResearchStatus,
    pub evidence_status: ResearchStatus,
    pub execution_allowed: bool,
    pub request: PreflightRequest,
    pub context: Option<AccountContext>,
    pub source_evaluation: StrategyEvaluation,
    /// A separate evaluation at inspection time; the original is never overwritten.
    pub rechecked_evaluation: StrategyEvaluation,
    pub selected_cash_cents: u64,
    pub checks: Vec<RuleDecision>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreflightSummary {
    pub id: String,
    pub created_at: u64,
    pub evaluation_id: String,
    pub account_id: Option<String>,
    pub status: ResearchStatus,
    pub evidence_status: ResearchStatus,
    pub selected_cash_cents: u64,
}
impl From<Preflight> for PreflightSummary {
    fn from(p: Preflight) -> Self {
        Self {
            id: p.id,
            created_at: p.created_at,
            evaluation_id: p.request.evaluation_id,
            account_id: p.request.account_id,
            status: p.status,
            evidence_status: p.evidence_status,
            selected_cash_cents: p.selected_cash_cents,
        }
    }
}
fn aggregate(checks: &[RuleDecision]) -> ResearchStatus {
    if checks.iter().any(|c| c.status == ResearchStatus::Fail) {
        ResearchStatus::Fail
    } else if checks.is_empty() || checks.iter().any(|c| c.status == ResearchStatus::Unknown) {
        ResearchStatus::Unknown
    } else {
        ResearchStatus::Pass
    }
}
fn fresh(t: u64, at: u64, age: u64) -> bool {
    t <= at && at - t <= age
}
fn check(
    checks: &mut Vec<RuleDecision>,
    key: &str,
    status: ResearchStatus,
    reason: &str,
    observed: serde_json::Value,
) {
    checks.push(RuleDecision {
        module: key.into(),
        status,
        reason: reason.into(),
        observed,
    });
}
pub fn inspect(
    request: PreflightRequest,
    source: StrategyEvaluation,
    context: Option<AccountContext>,
    at: u64,
) -> Result<Preflight, String> {
    use ResearchStatus::{Fail, Pass, Unknown};
    request.validate()?;
    if request.evaluation_id != source.id
        || context.as_ref().map(|c| &c.account.id) != request.account_id.as_ref()
        || source
            .request
            .account_id
            .as_ref()
            .is_some_and(|id| Some(id) != request.account_id.as_ref())
        || source
            .plan
            .request
            .buff_account_id
            .as_ref()
            .is_some_and(|id| Some(id) != request.account_id.as_ref())
    {
        return Err("研究、账户或报价账户绑定不匹配".into());
    }
    let rechecked = evaluate_with_order_books(
        source.strategy.clone(),
        source.plan.clone(),
        source.histories.clone(),
        source.order_books.clone(),
        source.request.clone(),
        at,
    )?;
    let mut checks = vec![];
    let mut cash = 0u64;
    let age = request
        .max_quote_age_secs
        .min(source.plan.request.max_quote_age_secs);
    check(
        &mut checks,
        "source",
        if source.plan.source == SourceKind::Demo {
            Fail
        } else {
            Pass
        },
        "演示计划不能进入真实交易；真实来源仍需分别核验平台证据",
        json!({"source":source.plan.source}),
    );
    for selected in &request.selections {
        let line = source
            .plan
            .lines
            .iter()
            .find(|l| l.candidate.item.key == selected.item_key)
            .ok_or("所选商品不属于研究计划")?;
        if selected.quantity > line.quantity {
            return Err("数量超过原计划；请生成新计划和研究".into());
        }
        cash = cash
            .checked_add(
                line.candidate
                    .item
                    .cash_cents
                    .checked_mul(u64::from(selected.quantity))
                    .ok_or("金额溢出")?,
            )
            .ok_or("金额溢出")?;
        let item = &line.candidate.item;
        let result = rechecked
            .items
            .iter()
            .find(|r| r.item_key == selected.item_key)
            .ok_or("研究缺少所选商品")?;
        check(
            &mut checks,
            "selected_strategy",
            result.status,
            "按当前时间重算所选商品的原策略版本；不刷新来源",
            json!({"item_key":item.key,"quantity":selected.quantity,"decisions":result.decisions}),
        );
        check(
            &mut checks,
            "selected_quote_age",
            if fresh(item.observed_at, at, age) {
                Pass
            } else {
                Fail
            },
            "原计划报价必须在期限内，未来时间亦拒绝",
            json!({"item_key":item.key,"observed_at":item.observed_at,"max_age_secs":age}),
        );
        let buff = source
            .plan
            .buff_valuations
            .iter()
            .find(|b| b.item_key == item.key);
        let buff_session = context
            .as_ref()
            .and_then(|c| c.sessions.iter().find(|s| s.platform == Platform::Buff));
        let buff_status = match buff {
            None => Unknown,
            Some(b)
                if Some(&b.account_id) != request.account_id.as_ref()
                    || item.goods_id != Some(b.goods_id)
                    || b.unit_cash_cents != Some(item.cash_cents)
                    || selected.quantity > b.lowest_tier_visible_units
                    || !fresh(b.observed_at, at, age) =>
            {
                Fail
            }
            Some(b) => match buff_session {
                Some(s) if s.generation == b.credential_generation => Pass,
                Some(_) => Fail,
                None => Unknown,
            },
        };
        check(
            &mut checks,
            "buff_binding",
            buff_status,
            "校验账户、凭据代次、goods_id、原价、首屏最低档数量与时间；可见量不是锁单",
            json!({"item_key":item.key,"evidence":buff}),
        );
        let steam = source
            .plan
            .steam_valuations
            .iter()
            .find(|b| b.market_hash_name == item.key);
        check(
            &mut checks,
            "steam_quote",
            match steam {
                None => Unknown,
                Some(b) if fresh(b.observed_at, at, age) => Pass,
                Some(_) => Fail,
            },
            "需要计划显式引用的同名 Steam 盘口；保存的盘口未作网络刷新",
            json!({"item_key":item.key,"evidence":steam}),
        );
        let session = context
            .as_ref()
            .and_then(|c| c.sessions.iter().find(|s| s.platform == Platform::Steam));
        for h in source
            .histories
            .iter()
            .filter(|h| h.request.item_key == item.key)
        {
            check(
                &mut checks,
                "history_generation",
                match session {
                    Some(s) if h.credential_generation == s.generation => Pass,
                    Some(_) => Fail,
                    None => Unknown,
                },
                "价格历史必须属于当前 Steam 凭据代次",
                json!({"item_key":item.key,"history_id":h.id,"credential_generation":h.credential_generation}),
            );
        }
    }
    let evidence_status = aggregate(&checks);
    for platform in [Platform::Steam, Platform::Buff] {
        let view = context
            .as_ref()
            .and_then(|c| c.sessions.iter().find(|s| s.platform == platform));
        let status = match view {
            None => Unknown,
            Some(s)
                if matches!(
                    s.status,
                    SessionStatus::Invalid
                        | SessionStatus::Missing
                        | SessionStatus::RateLimited
                        | SessionStatus::VerificationRequired
                ) || s.retry_at > at =>
            {
                Fail
            }
            Some(s)
                if s.status == SessionStatus::Valid
                    && s.subject.as_deref().is_some_and(|s| !s.is_empty())
                    && s.checked_at.is_some_and(|t| fresh(t, at, 300)) =>
            {
                if platform == Platform::Steam
                    && context
                        .as_ref()
                        .is_some_and(|c| Some(c.account.steam_id.as_str()) != s.subject.as_deref())
                {
                    Fail
                } else {
                    Pass
                }
            }
            Some(_) => Unknown,
        };
        check(
            &mut checks,
            &format!("{}_session_metadata", platform.key()),
            status,
            "仅校验本地身份验证元数据（300秒）；不读取密钥，不代表当前平台仍允许交易",
            json!(view),
        );
    }
    check(
        &mut checks,
        "buff_policy",
        match &context {
            Some(c) if c.buff_blocked || c.buff_retry_at > at => Fail,
            Some(_) => Pass,
            None => Unknown,
        },
        "沿用已保存的 BUFF 挑战与冷却，不为预检绕过访问限制",
        json!({"retry_at":context.as_ref().map(|c|c.buff_retry_at),"blocked":context.as_ref().map(|c|c.buff_blocked)}),
    );
    for (key, reason) in [
        ("live_quote_refresh", "尚未执行交易前实时刷新及价格容差复核"),
        ("balance", "尚未接入账户真实可用余额与资金预留"),
        (
            "position_ledger",
            "尚未完成持有、挂牌和未决订单的统一持仓核对",
        ),
        (
            "platform_fees",
            "费用是研究模型，尚未按账户区域与平台实测核验",
        ),
        ("order_executor", "尚未接入持久化订单、幂等提交及成交对账"),
    ] {
        check(&mut checks, key, Unknown, reason, json!(null));
    }
    Ok(Preflight {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: at,
        algorithm: "saved_evidence_preflight_v1".into(),
        mode: "saved_evidence_only".into(),
        status: aggregate(&checks),
        evidence_status,
        execution_allowed: false,
        request,
        context,
        source_evaluation: source,
        rechecked_evaluation: rechecked,
        selected_cash_cents: cash,
        checks,
    })
}

#[cfg(test)]
#[path = "preflight_tests.rs"]
pub(crate) mod tests;
