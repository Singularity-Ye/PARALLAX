use crate::domain::{
    notify::NotifyConfig,
    selection::now,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;

pub struct NotifyService {
    config_path: PathBuf,
    client: crate::adapters::notify::PushPlusClient,
    config: Mutex<NotifyConfig>,
    alert_throttles: Mutex<HashMap<String, u64>>,
}

impl NotifyService {
    pub fn new(data_dir: &Path) -> Arc<Self> {
        let config_path = data_dir.join("notify_config.json");
        let initial = if let Ok(content) = std::fs::read_to_string(&config_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            NotifyConfig::default()
        };
        Arc::new(Self {
            config_path,
            client: crate::adapters::notify::PushPlusClient::new(),
            config: Mutex::new(initial),
            alert_throttles: Mutex::new(HashMap::new()),
        })
    }

    pub async fn get_config(&self) -> NotifyConfig {
        self.config.lock().await.clone()
    }

    pub async fn get_masked_config(&self) -> NotifyConfig {
        let mut cfg = self.config.lock().await.clone();
        if !cfg.pushplus_token.is_empty() {
            let t = &cfg.pushplus_token;
            cfg.pushplus_token = if t.len() > 8 {
                format!("{}****{}", &t[..4], &t[t.len() - 4..])
            } else {
                "****".into()
            };
        }
        cfg
    }

    pub async fn update_config(&self, mut new_cfg: NotifyConfig) -> anyhow::Result<NotifyConfig> {
        let mut current = self.config.lock().await;
        // If frontend didn't modify the masked token, retain existing token
        if new_cfg.pushplus_token.contains("****") {
            new_cfg.pushplus_token = current.pushplus_token.clone();
        }
        *current = new_cfg.clone();
        if let Some(parent) = self.config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let raw = serde_json::to_string_pretty(&*current)?;
        std::fs::write(&self.config_path, raw)?;
        Ok(current.clone())
    }

    pub async fn send_test(&self) -> anyhow::Result<()> {
        let cfg = self.config.lock().await.clone();
        if cfg.pushplus_token.trim().is_empty() {
            anyhow::bail!("尚未配置 PushPlus Token，请在输入框填写后再测试");
        }
        let ts = now();
        let time_str = chrono::DateTime::from_timestamp(ts as i64, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| ts.to_string());
        let title = "【Parallax 异观】微信推送测试通知";
        let content = format!(
            "<div style='font-family:sans-serif;padding:8px 0;'>\
            <h3 style='color:#2563eb;margin:0 0 10px 0;'>✓ PushPlus 微信通知通道连接成功！</h3>\
            <p style='color:#334155;margin:4px 0;'>您的 Steam 自动化交易网关 Parallax 已接入微信实时推送。</p>\
            <p style='color:#64748b;font-size:12px;margin:12px 0 0 0;'>测试触发时间戳：{}</p>\
            </div>",
            time_str
        );
        self.client.send(&cfg.pushplus_token, title, &content).await
    }

    pub async fn notify_sold(
        &self,
        account_name: &str,
        item_name: &str,
        asset_id: &str,
        net_amount_display: &str,
    ) {
        let cfg = self.config.lock().await.clone();
        if !cfg.enabled || !cfg.notify_on_sold || cfg.pushplus_token.trim().is_empty() {
            return;
        }
        let title = format!("【Parallax 成交播报】饰品已在 Steam 市场售出");
        let content = format!(
            "<div style='font-family:sans-serif;padding:8px 0;'>\
            <h3 style='color:#16a34a;margin:0 0 10px 0;'>🎉 饰品成功售出并变现结算</h3>\
            <p style='margin:4px 0;'><b>所属账户：</b>{}</p>\
            <p style='margin:4px 0;'><b>饰品名称：</b>{}</p>\
            <p style='margin:4px 0;'><b>AssetID：</b><code>{}</code></p>\
            <p style='margin:4px 0;'><b>税后实得：</b><span style='color:#16a34a;font-weight:700;'>{}</span></p>\
            <p style='color:#64748b;font-size:12px;margin:12px 0 0 0;'>由 Parallax 后台自动化调度器核验入账</p>\
            </div>",
            account_name, item_name, asset_id, net_amount_display
        );
        let _ = self.client.send(&cfg.pushplus_token, &title, &content).await;
    }

    pub async fn notify_auth_expired(&self, account_name: &str, reason: &str) {
        let cfg = self.config.lock().await.clone();
        if !cfg.enabled || !cfg.notify_on_auth_expired || cfg.pushplus_token.trim().is_empty() {
            return;
        }
        let throttle_key = format!("auth_expired:{}", account_name);
        {
            let mut throttles = self.alert_throttles.lock().await;
            let last = throttles.get(&throttle_key).copied().unwrap_or(0);
            if now().saturating_sub(last) < 14400 {
                return; // Throttle: 4 hours
            }
            throttles.insert(throttle_key, now());
        }
        let title = format!("【Parallax 报警】Steam 账号 [{}] 登录态需手动介入", account_name);
        let content = format!(
            "<div style='font-family:sans-serif;padding:8px 0;'>\
            <h3 style='color:#dc2626;margin:0 0 10px 0;'>⚠️ 账号登录态失效 / 需手动验证</h3>\
            <p style='margin:4px 0;'><b>报警账户：</b>{}</p>\
            <p style='margin:4px 0;'><b>原因说明：</b><span style='color:#dc2626;'>{}</span></p>\
            <p style='color:#334155;margin:8px 0;'>为了避免错过行情或挂单失败，请前往 Parallax 账号管理页面点击「⚡ 验证」完成免密重新登录。</p>\
            </div>",
            account_name, reason
        );
        let _ = self.client.send(&cfg.pushplus_token, &title, &content).await;
    }
}
