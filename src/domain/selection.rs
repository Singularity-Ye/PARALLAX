use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Demo,
    Steamdt,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScanRequest {
    pub source: SourceKind,
    pub max_pages: usize,
    pub page_size: usize,
    pub max_items: usize,
    pub min_price_cents: u64,
    pub max_price_cents: u64,
    pub min_volume: u64,
    pub max_ratio_bps: u64,
    pub exclude_keywords: Vec<String>,
    pub wallet_policy: super::money::WalletPolicy,
    /// Max SteamDT-passed candidates to verify one by one on Steam; 0 disables phase 2.
    pub deep_verify_limit: usize,
    /// Seconds between two candidates (Steam pacing).
    pub deep_interval_secs: u64,
    pub deep_max_cv_bps: u64,
    pub deep_min_slope_bps: i64,
    pub deep_history_days: u32,
}
impl Default for ScanRequest {
    fn default() -> Self {
        Self {
            source: SourceKind::Demo,
            max_pages: 3,
            page_size: 10,
            max_items: 100,
            min_price_cents: 200,
            max_price_cents: 500_000,
            min_volume: 200,
            max_ratio_bps: 9000,
            exclude_keywords: vec!["印花".into()],
            wallet_policy: super::money::WalletPolicy::default(),
            deep_verify_limit: 10,
            deep_interval_secs: 3,
            deep_max_cv_bps: 1000,
            deep_min_slope_bps: -100,
            deep_history_days: 30,
        }
    }
}
impl ScanRequest {
    pub fn validate(&self) -> Result<(), String> {
        self.wallet_policy.validate_cny()?;
        if !(1..=20).contains(&self.max_pages)
            || !(1..=200).contains(&self.page_size)
            || !(1..=2000).contains(&self.max_items)
        {
            return Err("页数须为 1–20，每页 1–200，分析预算 1–2000；0 不表示无限".into());
        }
        if self.min_price_cents > self.max_price_cents
            || self.max_price_cents > 100_000_000
            || self.max_ratio_bps == 0
            || self.max_ratio_bps > 20_000
        {
            return Err("价格范围或折算率无效".into());
        }
        if self.exclude_keywords.len() > 30 || self.exclude_keywords.iter().any(|s| s.len() > 120) {
            return Err("排除关键词过多或过长".into());
        }
        if self.deep_verify_limit > 50
            || !(2..=30).contains(&self.deep_interval_secs)
            || !(1..=10_000).contains(&self.deep_max_cv_bps)
            || !(-10_000..=10_000).contains(&self.deep_min_slope_bps)
            || !(8..=365).contains(&self.deep_history_days)
        {
            return Err("深度复核参数无效：件数 0–50，冷却 2–30 秒，历史 8–365 日".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketItem {
    pub key: String,
    pub name: String,
    pub goods_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub cash_cents: u64,
    pub steam_gross_cents: u64,
    pub volume: u64,
    pub observed_at: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RuleResult {
    pub rule: String,
    pub passed: bool,
    pub reason: String,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct CandidateDeepAnalysis {
    /// True only when both the Steam order book and price history were read for real.
    pub verified: bool,
    pub ob_verified: bool,
    pub history_verified: bool,
    pub real_ratio_bps: Option<u64>,
    pub slope_bps_per_day: Option<i64>,
    pub history_days: u32,
    pub history_snapshot_id: Option<String>,
    pub smart_price_cents: u64,
    pub sell_pressure: f64,
    pub sell_pressure_ok: bool,
    pub cv_30d: f64,
    pub r_squared: f64,
    pub trend_status: String,
    pub ema7_cents: u64,
    pub ema30_cents: u64,
    pub price_percentile: u8,
    pub held_same_units: u32,
    pub safe_purchase_limit: u32,
    pub stability_passed: bool,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Candidate {
    pub item: MarketItem,
    pub passed: bool,
    pub estimated_wallet_cents: u64,
    pub ratio_bps: Option<u64>,
    pub rules: Vec<RuleResult>,
    /// Absent for historical v1 reports; their original estimates remain unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valuation: Option<super::money::FeeQuote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deep_analysis: Option<CandidateDeepAnalysis>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Coverage {
    pub pages_requested: usize,
    pub pages_received: usize,
    pub rows_received: usize,
    pub duplicates: usize,
    pub unique_received: usize,
    pub examined: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub unexamined_received: usize,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScanReport {
    pub id: String,
    pub request: ScanRequest,
    pub status: String,
    pub stop_reason: String,
    pub source_exhausted: bool,
    pub created_at: u64,
    pub finished_at: Option<u64>,
    pub coverage: Coverage,
    pub candidates: Vec<Candidate>,
    pub error: Option<String>,
    /// "fetch" (SteamDT list) → "deep_verify" (per-item Steam checks) → "done".
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub deep_total: usize,
    #[serde(default)]
    pub deep_done: usize,
    #[serde(default)]
    pub logs: Vec<ScanLog>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScanLog {
    pub at: u64,
    pub level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    pub message: String,
}
impl ScanReport {
    pub fn new(request: ScanRequest) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            request,
            status: "running".into(),
            stop_reason: String::new(),
            source_exhausted: false,
            created_at: now(),
            finished_at: None,
            coverage: Coverage::default(),
            candidates: vec![],
            error: None,
            phase: "fetch".into(),
            deep_total: 0,
            deep_done: 0,
            logs: vec![],
        }
    }
}
