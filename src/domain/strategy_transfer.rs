//! Portable rules only; no account, credentials, evidence or activation state.
use super::strategy::StrategyDraft;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyPackage {
    pub format: String,
    pub format_version: u32,
    pub draft: StrategyDraft,
}
impl StrategyPackage {
    pub fn new(draft: StrategyDraft) -> Self {
        Self {
            format: "parallax_strategy".into(),
            format_version: 1,
            draft,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "parallax_strategy" || self.format_version != 1 {
            return Err(
                "仅支持Parallax策略包格式v1；原项目Python策略需显式转换，不能直接执行".into(),
            );
        }
        self.draft.validate()
    }
}
