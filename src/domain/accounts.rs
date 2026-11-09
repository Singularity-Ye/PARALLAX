use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Steam,
    Buff,
}
impl Platform {
    pub fn key(self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Buff => "buff",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Account {
    pub id: String,
    pub label: String,
    pub steam_id: String,
    pub created_at: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountProfile {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub avatar_url: String,
    #[serde(default)]
    pub currency_code: String,
    #[serde(default)]
    pub region_code: String,
    #[serde(default)]
    pub wallet_balance_display: String,
    #[serde(default)]
    pub wallet_balance_raw: Option<i64>,
    #[serde(default)]
    pub wallet_currency_id: Option<u32>,
    #[serde(default)]
    pub has_saved_password: bool,
    #[serde(default)]
    pub updated_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Unverified,
    Valid,
    Invalid,
    Unknown,
    RateLimited,
    VerificationRequired,
    Missing,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionView {
    pub platform: Platform,
    pub generation: String,
    pub status: SessionStatus,
    pub checked_at: Option<u64>,
    pub retry_at: u64,
    pub message: String,
    pub subject: Option<String>,
}

// Intentionally neither Debug nor Serialize: only the vault adapter may persist this.
pub struct Credentials {
    pub cookies: String,
    pub user_agent: String,
}
impl Credentials {
    pub fn validate(&self, platform: Platform, steam_id: &str) -> Result<(), String> {
        if self.cookies.is_empty()
            || self.cookies.len() + self.user_agent.len() > 2300
            || !self.cookies.is_ascii()
            || self.cookies.bytes().any(|b| b < 32 || b == 127)
            || self.user_agent.bytes().any(|b| b < 32 || b == 127)
        {
            return Err(
                "Cookie 必须是单行 ASCII 请求头；Cookie 与 User-Agent 合计最多 2300 字节".into(),
            );
        }
        let mut names = std::collections::HashSet::new();
        for part in self.cookies.split(';').filter(|s| !s.trim().is_empty()) {
            let (name, _) = part.trim().split_once('=').ok_or("Cookie 格式不正确")?;
            if name.is_empty() || !names.insert(name) {
                return Err("Cookie 存在空名称或重复名称".into());
            }
        }
        match platform {
            Platform::Steam => {
                let value = self
                    .cookie("steamLoginSecure")
                    .ok_or("缺少 steamLoginSecure")?;
                let id = value.split(['%', '|']).next().unwrap_or_default();
                if id != steam_id {
                    return Err("Steam Cookie 与账户 Steam ID 不匹配".into());
                }
            }
            Platform::Buff if self.cookie("session").is_none() => {
                return Err("缺少 BUFF session Cookie".into());
            }
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies
            .split(';')
            .filter_map(|s| s.trim().split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v)
    }
    pub fn extract_steam_id(&self) -> Option<String> {
        let value = self.cookie("steamLoginSecure")?;
        let id = value.split(['%', '|']).next().unwrap_or_default().trim();
        if id.len() == 17 && id.bytes().all(|b| b.is_ascii_digit()) {
            Some(id.to_owned())
        } else {
            None
        }
    }
}

// Private repository representation; credential_key is never returned by HTTP.
#[derive(Clone)]
pub struct StoredSession {
    pub view: SessionView,
    pub credential_key: String,
}

pub struct Verification {
    pub status: SessionStatus,
    pub message: &'static str,
    pub subject: Option<String>,
    pub retry_secs: u64,
}
