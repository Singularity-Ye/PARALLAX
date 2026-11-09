use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SteamFriend {
    pub steamid: String,
    pub name: String,
    pub avatar: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameEdition {
    pub id: String,
    pub edition_type: String, // "subid" or "bundleid"
    pub name: String,
    pub price: String,
    pub original_price: String,
    pub discount_pct: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameEditionsResponse {
    pub appid: u64,
    pub title: String,
    pub header_image: String,
    pub editions: Vec<GameEdition>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GiftCheckRequest {
    pub appid: u64,
    pub edition_id: String,
    pub sender_account: String,
    pub recipient_steamid: String,
    pub recipient_region: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GiftRiskVerdict {
    pub verdict: String, // "PASS", "REJECT_RISK", "SAME_REGION", "UNKNOWN"
    pub can_gift: bool,
    pub sender_region: String,
    pub recipient_region: String,
    pub sender_price_cny: f64,
    pub recipient_price_cny: f64,
    pub diff_pct: f64,
    pub message: String,
    pub store_url: String,
}
