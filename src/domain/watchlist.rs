//! Explicit account-scoped observation membership; no market reads or purchase signals.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchRequest {
    pub item_key: String,
    pub reason: String,
}
impl WatchRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.item_key.is_empty()
            || self.item_key.len() > 500
            || self.item_key.trim() != self.item_key
            || self.item_key.chars().any(char::is_control)
            || self.reason.trim().is_empty()
            || self.reason.len() > 2000
            || self.reason.chars().any(char::is_control)
        {
            return Err("需准确市场名称与1–2000字节关注原因，不含控制字符".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchItem {
    pub id: String,
    pub account_id: String,
    pub app_id: u32,
    pub item_key: String,
    pub reason: String,
    pub created_at: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserveRequest {
    pub research_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchObservation {
    pub id: String,
    pub watch_id: String,
    pub account_id: String,
    pub recorded_at: u64,
    pub research: super::holding_research::HoldingResearchReport,
}
