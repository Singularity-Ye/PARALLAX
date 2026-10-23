//! Observed assets, not purchase lots, listings, or authorization to trade.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct InventoryRequest {
    pub max_pages_per_context: u32,
}
impl Default for InventoryRequest {
    fn default() -> Self {
        Self {
            max_pages_per_context: 4,
        }
    }
}
impl InventoryRequest {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=10).contains(&self.max_pages_per_context) {
            return Err("每个库存区最多读取 1–10 页，每页 75 件".into());
        }
        Ok(())
    }
}
pub(crate) fn numeric_id(s: &str, allow_zero: bool) -> bool {
    !s.is_empty()
        && s.len() <= 20
        && s.bytes().all(|b| b.is_ascii_digit())
        && s.parse::<u64>()
            .is_ok_and(|v| (allow_zero || v > 0) && v.to_string() == s)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InventoryAsset {
    pub app_id: u32,
    pub context_id: String,
    pub asset_id: String,
    pub class_id: String,
    pub instance_id: String,
    pub amount: u32,
    pub market_hash_name: Option<String>,
    pub reported_tradable: bool,
    pub reported_marketable: bool,
    pub restriction_note_present: bool,
}
impl InventoryAsset {
    pub fn validate(&self, context: &str) -> bool {
        self.app_id == 730
            && self.context_id == context
            && ["2", "16"].contains(&context)
            && numeric_id(&self.asset_id, false)
            && numeric_id(&self.class_id, false)
            && numeric_id(&self.instance_id, true)
            && (1..=1_000_000).contains(&self.amount)
            && self
                .market_hash_name
                .as_ref()
                .is_none_or(|n| !n.is_empty() && n.len() <= 500 && !n.chars().any(char::is_control))
    }
    // Advisory flags only. No local timer may turn an asset into an executable listing.
    pub fn reported_transferable(&self) -> bool {
        self.context_id == "2" && self.reported_tradable && !self.restriction_note_present
    }
}
#[derive(Clone, Debug)]
pub struct InventoryPage {
    pub assets: Vec<InventoryAsset>,
    pub total: u32,
    pub next_asset_id: Option<String>,
}
impl InventoryPage {
    pub fn validate(&self, context: &str) -> Result<(), String> {
        let mut ids = HashSet::new();
        if self.assets.len() > 75
            || self.total > 1_000_000
            || self.assets.len() > self.total as usize
            || self
                .assets
                .iter()
                .any(|a| !a.validate(context) || !ids.insert(&a.asset_id))
            || self.next_asset_id.as_ref().is_some_and(|id| {
                !numeric_id(id, false) || !self.assets.iter().any(|a| &a.asset_id == id)
            })
            || (self.next_asset_id.is_some() && self.assets.is_empty())
        {
            return Err("Steam 库存页的身份、数量或游标无效".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InventoryCoverage {
    pub context_id: String,
    pub pages_read: u32,
    pub reported_total: u32,
    pub observed_assets: u32,
    pub exhausted: bool,
    pub stop_reason: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InventorySnapshot {
    pub id: String,
    pub account_id: String,
    pub steam_id: String,
    pub credential_generation: String,
    pub request: InventoryRequest,
    pub started_at: u64,
    pub observed_at: u64,
    pub source: String,
    pub platform_verified: bool,
    pub coverage: Vec<InventoryCoverage>,
    pub assets: Vec<InventoryAsset>,
    pub warnings: Vec<String>,
}
impl InventorySnapshot {
    pub fn coverage_complete(&self) -> bool {
        self.coverage.len() == 2 && self.coverage.iter().all(|c| c.exhausted)
    }
}
