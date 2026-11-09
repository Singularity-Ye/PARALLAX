use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotifyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub pushplus_token: String,
    #[serde(default = "default_true")]
    pub notify_on_sold: bool,
    #[serde(default = "default_true")]
    pub notify_on_auth_expired: bool,
    #[serde(default)]
    pub notify_on_large_price_drop: bool,
    #[serde(default = "default_drop_threshold")]
    pub price_drop_threshold_pct: f64,
}

fn default_true() -> bool {
    true
}

fn default_drop_threshold() -> f64 {
    20.0
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            pushplus_token: String::new(),
            notify_on_sold: true,
            notify_on_auth_expired: true,
            notify_on_large_price_drop: false,
            price_drop_threshold_pct: 20.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PushPlusRequest {
    pub token: String,
    pub title: String,
    pub content: String,
    #[serde(default = "default_template")]
    pub template: String,
}

fn default_template() -> String {
    "html".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PushPlusResponse {
    pub code: i64,
    pub msg: String,
    #[serde(default)]
    pub data: Option<String>,
}
