use serde::{Deserialize, Serialize};

pub const REGIONS: &[(&str, &str, &str)] = &[
    ("cn", "中国大陆", "CNY"),
    ("ru", "俄罗斯", "RUB"),
    ("kz", "哈萨克斯坦", "KZT"),
    ("ua", "乌克兰", "UAH"),
    ("pk", "南亚 (巴基斯坦)", "USD"),
    ("tr", "土耳其", "USD"),
    ("ar", "阿根廷", "USD"),
    ("az", "独联体 (阿塞拜疆)", "USD"),
    ("vn", "越南", "VND"),
    ("id", "印度尼西亚", "IDR"),
    ("in", "印度", "INR"),
    ("br", "巴西", "BRL"),
    ("cl", "智利", "CLP"),
    ("jp", "日本", "JPY"),
    ("hk", "中国香港", "HKD"),
    ("ph", "菲律宾", "PHP"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionPrice {
    pub region_code: String,
    pub region_name: String,
    pub currency: String,
    pub raw_final: u64,
    pub raw_initial: u64,
    pub discount_pct: u32,
    pub formatted_price: String,
    pub cny_cents: Option<u64>,
    pub cny_diff_cents: Option<i64>,
    pub value_ratio: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameDealComparison {
    pub appid: u64,
    pub title: String,
    pub header_image: String,
    pub cn_cny_cents: Option<u64>,
    pub lowest_region: Option<String>,
    pub lowest_cny_cents: Option<u64>,
    pub max_savings_pct: Option<f64>,
    pub regions: Vec<RegionPrice>,
    pub queried_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopularGameItem {
    pub appid: u64,
    pub title: String,
    pub cover_image: String,
    pub tag: String,
}
