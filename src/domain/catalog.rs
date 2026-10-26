use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRequest {
    pub query: String,
    #[serde(default)]
    pub start: u32,
}
impl CatalogRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.query.trim().is_empty()
            || self.query.len() > 200
            || self.query.chars().any(char::is_control)
            || self.start > 1000
        {
            return Err("关键词须为1–200字节且无控制字符，起点至多1000；每次只读取一页".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogItem {
    pub market_hash_name: String,
    pub name: String,
    pub sell_listings: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogPage {
    pub query: String,
    pub start: u32,
    pub received: usize,
    pub upstream_total: u64,
    pub next_start: Option<u32>,
    pub items: Vec<CatalogItem>,
    pub observed_at: u64,
    pub source: String,
    pub scope: String,
}
