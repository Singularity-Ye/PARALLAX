//! Local, durable, single-flight checkout preparation. There is no platform sender here.
use super::{
    funding::FundingSnapshot, ledger::text_id, preflight::Preflight, strategy::ResearchStatus,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareRequest {
    pub client_key: String,
    pub preflight_id: String,
    pub funding_snapshot_id: String,
    pub max_funding_age_secs: u64,
}
impl PrepareRequest {
    pub fn validate(&self) -> Result<(), String> {
        if [
            &self.client_key,
            &self.preflight_id,
            &self.funding_snapshot_id,
        ]
        .iter()
        .any(|s| !text_id(s, 64))
            || !(1..=300).contains(&self.max_funding_age_secs)
        {
            return Err("意图需请求键、预检/资金快照及1–300秒资金期限".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    Reserved,
    Cancelled,
}
impl IntentState {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseIntent {
    pub id: String,
    pub account_id: String,
    pub principal: String,
    pub revision: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub state: IntentState,
    pub mode: String,
    pub execution_allowed: bool,
    pub request: PrepareRequest,
    pub funding_snapshot: FundingSnapshot,
    pub rechecked_preflight: Preflight,
    pub reserved_cash_cents: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelRequest {
    pub client_key: String,
    pub expected_revision: u32,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentEvent {
    pub id: String,
    pub intent_id: String,
    pub account_id: String,
    pub recorded_at: u64,
    pub request: CancelRequest,
    pub result_revision: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentSummary {
    pub id: String,
    pub account_id: String,
    pub principal: String,
    pub revision: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub state: IntentState,
    pub reserved_cash_cents: u64,
    pub preflight_id: String,
}
impl From<PurchaseIntent> for IntentSummary {
    fn from(v: PurchaseIntent) -> Self {
        Self {
            id: v.id,
            account_id: v.account_id,
            principal: v.principal,
            revision: v.revision,
            created_at: v.created_at,
            updated_at: v.updated_at,
            state: v.state,
            reserved_cash_cents: v.reserved_cash_cents,
            preflight_id: v.request.preflight_id,
        }
    }
}
pub fn prepare(
    account: String,
    principal: String,
    request: PrepareRequest,
    funding: FundingSnapshot,
    preflight: Preflight,
    at: u64,
) -> Result<PurchaseIntent, String> {
    request.validate()?;
    if funding.id != request.funding_snapshot_id
        || funding.principal != principal
        || preflight.request.account_id.as_ref() != Some(&account)
        || preflight.selected_cash_cents == 0
    {
        return Err("资金/预检账户绑定或金额不匹配".into());
    }
    if funding.request.observed_at > at
        || at - funding.request.observed_at > request.max_funding_age_secs
    {
        return Err("资金观察已过期；重试不刷新时间".into());
    }
    if preflight.evidence_status != ResearchStatus::Pass
        || [
            "steam_session_metadata",
            "buff_session_metadata",
            "buff_policy",
        ]
        .iter()
        .any(|key| {
            !preflight
                .checks
                .iter()
                .any(|c| c.module == *key && c.status == ResearchStatus::Pass)
        })
    {
        return Err("本地意图需要所选证据和当前会话/访问条件满足；演示、未知或过期不占资金".into());
    }
    if preflight.selected_cash_cents > funding.request.available_cash_cents {
        return Err("资金观察不足以覆盖本次选择".into());
    }
    Ok(PurchaseIntent {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: account,
        principal,
        revision: 1,
        created_at: at,
        updated_at: at,
        state: IntentState::Reserved,
        mode: "local_preparation_only".into(),
        execution_allowed: false,
        reserved_cash_cents: preflight.selected_cash_cents,
        request,
        funding_snapshot: funding,
        rechecked_preflight: preflight,
    })
}
pub fn cancel(
    intent: &PurchaseIntent,
    request: &CancelRequest,
    at: u64,
) -> Result<PurchaseIntent, String> {
    if !text_id(&request.client_key, 64)
        || !text_id(&request.reason, 500)
        || request.expected_revision != intent.revision
        || intent.state != IntentState::Reserved
    {
        return Err("取消需有效原因、当前版本及尚未提交的本地预留态".into());
    }
    let mut v = intent.clone();
    v.state = IntentState::Cancelled;
    v.revision = v.revision.checked_add(1).ok_or("版本上限")?;
    v.updated_at = at;
    Ok(v)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckoutWrite<T> {
    pub item: T,
    pub replayed: bool,
}
pub enum CheckoutOutcome<T> {
    Saved(CheckoutWrite<T>),
    Missing(&'static str),
    Conflict(&'static str),
    Invalid(String),
}
