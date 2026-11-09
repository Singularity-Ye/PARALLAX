use crate::domain::notify::{PushPlusRequest, PushPlusResponse};
use anyhow::{Context, ensure};
use std::time::Duration;

pub struct PushPlusClient {
    http: reqwest::Client,
}

impl PushPlusClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_default();
        Self { http }
    }

    pub async fn send(&self, token: &str, title: &str, content: &str) -> anyhow::Result<()> {
        let token = token.trim();
        ensure!(!token.is_empty(), "PushPlus Token 不能为空");
        let req = PushPlusRequest {
            token: token.to_owned(),
            title: title.to_owned(),
            content: content.to_owned(),
            template: "html".into(),
        };
        let resp = self
            .http
            .post("http://www.pushplus.plus/send")
            .json(&req)
            .send()
            .await
            .context("调用 PushPlus 接口网络失败")?;
        let status = resp.status();
        let body: PushPlusResponse = resp
            .json()
            .await
            .with_context(|| format!("PushPlus 返回非 JSON 格式响应 (HTTP {status})"))?;
        if body.code != 200 {
            anyhow::bail!("PushPlus 拒绝发送 (code={}): {}", body.code, body.msg);
        }
        Ok(())
    }
}

impl Default for PushPlusClient {
    fn default() -> Self {
        Self::new()
    }
}
