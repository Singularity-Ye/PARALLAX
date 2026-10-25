//! CNY cash observations are distinct from Steam wallet value and execution permission.
use super::{
    accounts::{Platform, SessionStatus},
    ledger::text_id,
    preflight::AccountContext,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FundingRequest {
    pub client_key: String,
    pub available_cash_cents: u64,
    pub observed_at: u64,
    pub evidence_ref: String,
}
impl FundingRequest {
    pub fn validate(&self, at: u64) -> Result<(), String> {
        if !text_id(&self.client_key, 64)
            || !text_id(&self.evidence_ref, 128)
            || self.available_cash_cents > 1_000_000_000_000
            || self.observed_at == 0
            || self.observed_at > at
        {
            return Err("资金观察需请求键、凭据、非未来观察时间及0–1e12的CNY分金额".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FundingSnapshot {
    pub id: String,
    pub principal: String,
    pub observed_by_account_id: String,
    pub created_at: u64,
    pub provenance: String,
    pub currency: String,
    pub platform_verified: bool,
    pub request: FundingRequest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FundingView {
    pub principal: String,
    pub snapshot: Option<FundingSnapshot>,
    pub reserved_cash_cents: u64,
    pub available_after_reservations_cents: Option<u64>,
    pub deficit_cents: Option<u64>,
    pub fresh: bool,
    pub execution_allowed: bool,
}
pub fn buff_principal(context: &AccountContext, at: u64) -> Result<String, String> {
    let s = context
        .sessions
        .iter()
        .find(|s| s.platform == Platform::Buff)
        .ok_or("需要已核验的BUFF主体才能归属资金")?;
    if s.status != SessionStatus::Valid
        || s.checked_at.is_none_or(|t| t > at || at - t > 300)
        || s.retry_at > at
    {
        return Err("BUFF身份元数据过期或不可用；资金未核验，不猜账户归属".into());
    }
    s.subject
        .as_ref()
        .filter(|s| text_id(s, 128))
        .cloned()
        .ok_or("BUFF主体未知".into())
}
pub fn view(
    principal: String,
    snapshot: Option<FundingSnapshot>,
    reserved: u64,
    at: u64,
) -> FundingView {
    let value = snapshot.as_ref().map(|s| s.request.available_cash_cents);
    let fresh = snapshot
        .as_ref()
        .is_some_and(|s| s.request.observed_at <= at && at - s.request.observed_at <= 300);
    FundingView {
        principal,
        snapshot,
        reserved_cash_cents: reserved,
        available_after_reservations_cents: value.map(|v| v.saturating_sub(reserved)),
        deficit_cents: value.map(|v| reserved.saturating_sub(v)),
        fresh,
        execution_allowed: false,
    }
}
