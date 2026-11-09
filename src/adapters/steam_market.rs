use crate::domain::{selection::now, steam_guard::GuardSecret};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const MOBILE_UA: &str = "Steam Mobile/10372190 CFNetwork/3860.100.1 Darwin/25.0.0";
const DESKTOP_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingConfirmation {
    pub id: String,
    pub nonce: String,
    pub headline: String,
    pub summary: String,
    pub type_name: String,
    pub creation_time: u64,
    pub icon: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketActiveListing {
    pub listing_id: String,
    pub asset_id: String,
    pub market_hash_name: String,
    pub item_name: String,
    pub icon_url: String,
    pub price_minor: i64,
    pub fee_minor: i64,
    pub total_minor: i64,
    pub time_created: u64,
    pub need_confirmation: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketListingsSnapshot {
    pub total_count: usize,
    pub active: Vec<MarketActiveListing>,
    pub to_confirm: Vec<MarketActiveListing>,
    pub observed_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SellItemResponse {
    pub success: bool,
    pub requires_confirmation: bool,
    pub auto_confirmed: usize,
    pub buyer_pays_minor: i64,
    pub seller_receives_minor: i64,
    pub currency_code: String,
    pub message: String,
}

fn extract_session_id(cookies: &str) -> Option<String> {
    for part in cookies.split(';') {
        let s = part.trim();
        if let Some((k, v)) = s.split_once('=') {
            if k.trim().eq_ignore_ascii_case("sessionid") && !v.trim().is_empty() {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

async fn query_steam_time(client: &reqwest::Client) -> u64 {
    let fallback = now();
    let Ok(resp) = client
        .post("https://api.steampowered.com/ITwoFactorService/QueryTime/v0001")
        .header("content-type", "application/x-www-form-urlencoded")
        .body("steamid=0")
        .send()
        .await
    else {
        return fallback;
    };
    let Ok(data) = resp.json::<Value>().await else {
        return fallback;
    };
    data["response"]["server_time"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| data["response"]["server_time"].as_u64())
        .unwrap_or(fallback)
}

/// Converts buyer payment total (in minor units, e.g. cents / kopeks) into seller net receive (`price` param for `market/sellitem`).
/// Matches Steam's `GetItemPriceFromTotal` / AetherSwap `get_item_price_from_total` with 10% publisher fee + 5% Steam fee.
pub fn buyer_total_to_seller_net(total_minor: i64, _currency_code: &str) -> i64 {
    let total = total_minor.max(3);
    let fee_min = 1;
    let calc_total = |base: i64| -> i64 {
        let pub_fee = ((base as f64 * 0.10).floor() as i64).max(fee_min);
        let steam_fee = ((base as f64 * 0.05).floor() as i64).max(fee_min);
        base + pub_fee + steam_fee
    };
    let mut base = ((total as f64 / 1.15).floor() as i64)
        .min(total - 2 * fee_min)
        .max(1);
    for _ in 0..5 {
        let t = calc_total(base);
        if t == total {
            return base.max(1);
        }
        if t < total {
            base += 1;
        } else {
            base -= 1;
            break;
        }
    }
    base.max(1)
}

/// Fetches pending Steam Guard mobile confirmations (`mobileconf/getlist`).
pub async fn get_mobile_confirmations(
    identity_secret: &str,
    device_id: &str,
    steam_id: &str,
    cookies: &str,
) -> anyhow::Result<Vec<PendingConfirmation>> {
    ensure!(
        cookies.contains("steamLoginSecure="),
        "Steam 会话未包含 steamLoginSecure，请先在账号管理中点击验证登录"
    );
    let client = super::network::client_builder()
        .user_agent(MOBILE_UA)
        .timeout(Duration::from_secs(18))
        .build()?;
    let ts = query_steam_time(&client).await;
    let sig = GuardSecret::signature(identity_secret, "conf", ts)
        .map_err(|e| anyhow::anyhow!(e))?;
    let resp = client
        .get("https://steamcommunity.com/mobileconf/getlist")
        .query(&[
            ("p", device_id),
            ("a", steam_id),
            ("k", &sig),
            ("t", &ts.to_string()),
            ("m", "react"),
            ("tag", "conf"),
        ])
        .header("Cookie", cookies)
        .header("Accept", "application/json, text/plain, */*")
        .send()
        .await
        .context("请求 Steam mobileconf/getlist 失败")?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text).with_context(|| {
        format!("Steam mobileconf 返回非 JSON (HTTP {status}): {}", &text[..text.len().min(120)])
    })?;
    if !data["success"].as_bool().unwrap_or(false) {
        let msg = data["message"]
            .as_str()
            .or_else(|| data["detail"].as_str())
            .unwrap_or("Steam 拒绝了待确认列表请求（请检查 identity_secret 与登录态）");
        anyhow::bail!("{msg}");
    }
    let mut out = Vec::new();
    if let Some(arr) = data["conf"].as_array() {
        for item in arr {
            let id = item["id"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| item["id"].as_u64().map(|n| n.to_string()))
                .unwrap_or_default();
            let nonce = item["nonce"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| item["nonce"].as_u64().map(|n| n.to_string()))
                .unwrap_or_default();
            if id.is_empty() || nonce.is_empty() {
                continue;
            }
            let summary = item["summary"]
                .as_array()
                .map(|lines| {
                    lines
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" · ")
                })
                .or_else(|| item["summary"].as_str().map(str::to_owned))
                .unwrap_or_default();
            out.push(PendingConfirmation {
                id,
                nonce,
                headline: item["headline"].as_str().unwrap_or("市场挂牌 / 交易确认").to_owned(),
                summary,
                type_name: item["type_name"].as_str().unwrap_or("确认项").to_owned(),
                creation_time: item["creation_time"].as_u64().unwrap_or(0),
                icon: item["icon"].as_str().unwrap_or_default().to_owned(),
            });
        }
    }
    Ok(out)
}

/// Accepts a batch of pending Steam Guard confirmations (`mobileconf/multiajaxop`).
pub async fn accept_mobile_confirmations(
    identity_secret: &str,
    device_id: &str,
    steam_id: &str,
    cookies: &str,
    items: &[PendingConfirmation],
) -> anyhow::Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }
    let client = super::network::client_builder()
        .user_agent(MOBILE_UA)
        .timeout(Duration::from_secs(22))
        .build()?;
    let ts = query_steam_time(&client).await;
    let sig = GuardSecret::signature(identity_secret, "accept", ts)
        .map_err(|e| anyhow::anyhow!(e))?;
    let mut pairs: Vec<(&str, String)> = vec![
        ("op", "allow".into()),
        ("p", device_id.into()),
        ("a", steam_id.into()),
        ("k", sig),
        ("t", ts.to_string()),
        ("m", "react".into()),
        ("tag", "accept".into()),
    ];
    for c in items {
        pairs.push(("cid[]", c.id.clone()));
        pairs.push(("ck[]", c.nonce.clone()));
    }
    let body = {
        let mut enc = form_urlencoded::Serializer::new(String::new());
        for (k, v) in &pairs {
            enc.append_pair(k, v);
        }
        enc.finish()
    };
    let resp = client
        .post("https://steamcommunity.com/mobileconf/multiajaxop")
        .header("Cookie", cookies)
        .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
        .header("Accept", "application/json, text/plain, */*")
        .body(body)
        .send()
        .await
        .context("请求 Steam mobileconf/multiajaxop 失败")?;
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text)
        .with_context(|| format!("Steam multiajaxop 响应非 JSON: {}", &text[..text.len().min(120)]))?;
    ensure!(
        data["success"].as_bool().unwrap_or(false),
        "Steam 拒绝批量确认操作: {}",
        data["message"].as_str().unwrap_or(&text[..text.len().min(120)])
    );
    Ok(items.len())
}

/// Fetches active and pending-confirmation market listings (`market/mylistings`).
pub async fn fetch_my_listings(cookies: &str) -> anyhow::Result<MarketListingsSnapshot> {
    ensure!(
        cookies.contains("steamLoginSecure="),
        "Steam 会话未包含 steamLoginSecure，请先登录"
    );
    let client = super::network::client_builder()
        .user_agent(DESKTOP_UA)
        .timeout(Duration::from_secs(18))
        .build()?;
    let resp = client
        .get("https://steamcommunity.com/market/mylistings/")
        .query(&[("norender", "1"), ("start", "0"), ("count", "100")])
        .header("Cookie", cookies)
        .header("Accept", "application/json")
        .header("Referer", "https://steamcommunity.com/market/")
        .send()
        .await
        .context("读取 Steam 我的在售列表失败")?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text).with_context(|| {
        format!("Steam mylistings 返回非 JSON (HTTP {status})")
    })?;
    ensure!(
        data["success"].as_bool().unwrap_or(false),
        "Steam mylistings 返回 success=false"
    );
    let assets_root = &data["assets"];
    let parse_group = |arr_val: &Value, need_conf: bool| -> Vec<MarketActiveListing> {
        let mut list = Vec::new();
        let items: Vec<&Value> = if let Some(a) = arr_val.as_array() {
            a.iter().collect()
        } else if let Some(obj) = arr_val.as_object() {
            obj.values().collect()
        } else {
            Vec::new()
        };
        for item in items {
            let listing_id = item["listingid"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| item["listingid"].as_u64().map(|n| n.to_string()))
                .unwrap_or_default();
            let asset = &item["asset"];
            let asset_id = asset["id"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| asset["id"].as_u64().map(|n| n.to_string()))
                .unwrap_or_default();
            if listing_id.is_empty() && asset_id.is_empty() {
                continue;
            }
            let appid = asset["appid"]
                .as_u64()
                .map(|n| n.to_string())
                .or_else(|| asset["appid"].as_str().map(str::to_owned))
                .unwrap_or_else(|| "730".into());
            let contextid = asset["contextid"]
                .as_str()
                .map(str::to_owned)
                .or_else(|| asset["contextid"].as_u64().map(|n| n.to_string()))
                .unwrap_or_else(|| "2".into());
            let desc = &assets_root[&appid][&contextid][&asset_id];
            let market_hash_name = desc["market_hash_name"]
                .as_str()
                .or_else(|| asset["market_hash_name"].as_str())
                .unwrap_or("CS2 挂牌物品")
                .to_owned();
            let item_name = desc["name"]
                .as_str()
                .unwrap_or(&market_hash_name)
                .to_owned();
            let icon_raw = desc["icon_url"].as_str().unwrap_or_default();
            let icon_url = if icon_raw.is_empty() {
                String::new()
            } else if icon_raw.starts_with("http") {
                icon_raw.to_owned()
            } else {
                format!("https://community.fastly.steamstatic.com/economy/image/{icon_raw}/96fx96f")
            };
            let price_minor = item["price"].as_i64().unwrap_or(0);
            let fee_minor = item["fee"].as_i64().unwrap_or(0);
            list.push(MarketActiveListing {
                listing_id,
                asset_id,
                market_hash_name,
                item_name,
                icon_url,
                price_minor,
                fee_minor,
                total_minor: price_minor + fee_minor,
                time_created: item["time_created"].as_u64().unwrap_or(0),
                need_confirmation: need_conf
                    || item["need_confirmation"].as_bool().unwrap_or(false),
            });
        }
        list
    };
    let active = parse_group(&data["listings"], false);
    let to_confirm = parse_group(&data["listings_to_confirm"], true);
    let total_count = active.len() + to_confirm.len();
    Ok(MarketListingsSnapshot {
        total_count,
        active,
        to_confirm,
        observed_at: now(),
    })
}

/// Lists an item on the Steam Community Market (`market/sellitem/`).
pub async fn sell_item_on_market(
    steam_id: &str,
    cookies: &str,
    asset_id: &str,
    buyer_pays_minor: i64,
    currency_code: &str,
    identity: Option<(String, String)>,
) -> anyhow::Result<SellItemResponse> {
    ensure!(
        cookies.contains("steamLoginSecure="),
        "Steam 会话未包含 steamLoginSecure，请先验证登录"
    );
    let session_id = extract_session_id(cookies)
        .context("Steam Cookie 中缺少 sessionid，请点击验证重新登录")?;
    let seller_receives_minor = buyer_total_to_seller_net(buyer_pays_minor, currency_code);
    let client = super::network::client_builder()
        .user_agent(DESKTOP_UA)
        .timeout(Duration::from_secs(18))
        .build()?;
    let body = {
        let mut enc = form_urlencoded::Serializer::new(String::new());
        enc.extend_pairs([
            ("sessionid", session_id.as_str()),
            ("appid", "730"),
            ("contextid", "2"),
            ("assetid", asset_id),
            ("amount", "1"),
            ("price", &seller_receives_minor.to_string()),
        ]);
        enc.finish()
    };
    let resp = client
        .post("https://steamcommunity.com/market/sellitem/")
        .header("Cookie", cookies)
        .header("Origin", "https://steamcommunity.com")
        .header(
            "Referer",
            format!("https://steamcommunity.com/profiles/{steam_id}/inventory"),
        )
        .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
        .body(body)
        .send()
        .await
        .context("调用 Steam market/sellitem 失败")?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let data: Value = serde_json::from_str(&text).with_context(|| {
        format!("Steam sellitem 返回非 JSON (HTTP {status}): {}", &text[..text.len().min(120)])
    })?;
    let msg = data["message"].as_str().unwrap_or_default().to_owned();
    let msg_lower = msg.to_lowercase();
    let ok = status.is_success()
        && (data["success"].as_bool().unwrap_or(false)
            || msg_lower.contains("pending confirmation")
            || msg_lower.contains("already have a listing"));
    if !ok {
        anyhow::bail!(
            "Steam 拒绝上架 assetid={asset_id}: {}",
            if msg.is_empty() {
                &text[..text.len().min(140)]
            } else {
                &msg
            }
        );
    }
    let requires_confirmation = data["requires_confirmation"].as_i64().unwrap_or(0) != 0
        || data["requires_confirmation"].as_bool().unwrap_or(false)
        || msg_lower.contains("confirmation");
    let mut auto_confirmed = 0;
    if requires_confirmation {
        if let Some((identity_secret, device_id)) = identity {
            tokio::time::sleep(Duration::from_millis(900)).await;
            if let Ok(confs) =
                get_mobile_confirmations(&identity_secret, &device_id, steam_id, cookies).await
            {
                if !confs.is_empty() {
                    if let Ok(n) = accept_mobile_confirmations(
                        &identity_secret,
                        &device_id,
                        steam_id,
                        cookies,
                        &confs,
                    )
                    .await
                    {
                        auto_confirmed = n;
                    }
                }
            }
        }
    }
    Ok(SellItemResponse {
        success: true,
        requires_confirmation,
        auto_confirmed,
        buyer_pays_minor,
        seller_receives_minor,
        currency_code: currency_code.to_owned(),
        message: if auto_confirmed > 0 {
            format!("已上架并自动完成 Steam 令牌确认 ({auto_confirmed} 项)")
        } else if requires_confirmation {
            "已提交 Steam 市场，待令牌确认（可配置 identity_secret 免手机自动确认）".into()
        } else {
            "已在 Steam 市场直接上架生效".into()
        },
    })
}

/// Removes an active market listing (`market/removelisting/{listing_id}`).
pub async fn remove_market_listing(cookies: &str, listing_id: &str) -> anyhow::Result<()> {
    ensure!(
        cookies.contains("steamLoginSecure="),
        "Steam 会话未包含 steamLoginSecure，请先验证登录"
    );
    let listing_id = listing_id.trim();
    ensure!(
        !listing_id.is_empty() && listing_id.bytes().all(|b| b.is_ascii_digit()),
        "listing_id 需为纯数字编号"
    );
    let session_id = extract_session_id(cookies)
        .context("Steam Cookie 中缺少 sessionid，请点击验证重新登录")?;
    let client = super::network::client_builder()
        .user_agent(DESKTOP_UA)
        .timeout(Duration::from_secs(18))
        .build()?;
    let body = format!("sessionid={session_id}");
    let resp = client
        .post(format!(
            "https://steamcommunity.com/market/removelisting/{listing_id}"
        ))
        .header("Cookie", cookies)
        .header("Origin", "https://steamcommunity.com")
        .header("Referer", "https://steamcommunity.com/market/")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
        .body(body)
        .send()
        .await
        .context("调用 Steam market/removelisting 失败")?;
    ensure!(
        resp.status().is_success(),
        "Steam 撤单请求返回 HTTP {}",
        resp.status()
    );
    Ok(())
}
