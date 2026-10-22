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
        }
    }
}
impl ScanRequest {
    pub fn validate(&self) -> Result<(), String> {
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
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketItem {
    pub key: String,
    pub name: String,
    pub goods_id: Option<u64>,
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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Candidate {
    pub item: MarketItem,
    pub passed: bool,
    pub estimated_wallet_cents: u64,
    pub ratio_bps: Option<u64>,
    pub rules: Vec<RuleResult>,
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
        }
    }
}
