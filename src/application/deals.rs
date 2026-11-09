use crate::domain::{
    deals::{GameDealComparison, PopularGameItem, RegionPrice, REGIONS},
    selection::now,
};
use super::fx::FxResolver;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct SteamDealsService {
    http: reqwest::Client,
    fx: Arc<dyn FxResolver>,
}

impl SteamDealsService {
    pub fn new(fx: Arc<dyn FxResolver>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self { http, fx }
    }

    pub fn popular_games(&self) -> Vec<PopularGameItem> {
        vec![
            PopularGameItem {
                appid: 2358720,
                title: "黑神话：悟空 (Black Myth: Wukong)".into(),
                cover_image: "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/2358720/header.jpg".into(),
                tag: "国产神作 · 热门".into(),
            },
            PopularGameItem {
                appid: 1245620,
                title: "艾尔登法环 (ELDEN RING)".into(),
                cover_image: "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1245620/header.jpg".into(),
                tag: "TGA 年度 · 骨折常客".into(),
            },
            PopularGameItem {
                appid: 1091500,
                title: "赛博朋克 2077 (Cyberpunk 2077)".into(),
                cover_image: "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1091500/header.jpg".into(),
                tag: "开放世界 · 史低常客".into(),
            },
            PopularGameItem {
                appid: 1086940,
                title: "博德之门 3 (Baldur's Gate 3)".into(),
                cover_image: "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1086940/header.jpg".into(),
                tag: "年度 RPG · 剧情巅峰".into(),
            },
            PopularGameItem {
                appid: 271590,
                title: "侠盗猎车手 5 (Grand Theft Auto V)".into(),
                cover_image: "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/271590/header.jpg".into(),
                tag: "常青神作 · 超大折扣".into(),
            },
            PopularGameItem {
                appid: 1172470,
                title: "Apex 英雄 (Apex Legends)".into(),
                cover_image: "https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1172470/header.jpg".into(),
                tag: "多人竞技 · 免费本体".into(),
            },
        ]
    }

    pub async fn compare_game(&self, appid: u64) -> Result<GameDealComparison, String> {
        let mut set = JoinSet::new();

        for (code, name, curr) in REGIONS {
            let client = self.http.clone();
            let c_code = (*code).to_string();
            let c_name = (*name).to_string();
            let c_curr = (*curr).to_string();
            set.spawn(async move {
                let url = format!(
                    "https://store.steampowered.com/api/appdetails?appids={appid}&cc={c_code}&filters=price_overview,basic&l=schinese"
                );
                let resp = client.get(&url).send().await;
                match resp {
                    Ok(r) if r.status().is_success() => {
                        let json: serde_json::Value = r.json().await.unwrap_or_default();
                        let app_data = &json[appid.to_string()];
                        if app_data["success"].as_bool() == Some(true) {
                            let title = app_data["data"]["name"].as_str().unwrap_or("").to_string();
                            let price = &app_data["data"]["price_overview"];
                            if price.is_object() {
                                let raw_final = price["final"].as_u64().unwrap_or(0);
                                let raw_initial = price["initial"].as_u64().unwrap_or(0);
                                let discount_pct = price["discount_percent"].as_u64().unwrap_or(0) as u32;
                                let formatted = price["final_formatted"].as_str().unwrap_or("").to_string();
                                let currency = price["currency"].as_str().unwrap_or(&c_curr).to_string();
                                return (c_code, c_name, currency, Some((title, raw_final, raw_initial, discount_pct, formatted)));
                            } else {
                                let is_free = app_data["data"]["is_free"].as_bool().unwrap_or(false);
                                let formatted = if is_free { "免费".into() } else { "未开售/锁区".into() };
                                return (c_code, c_name, c_curr, Some((title, 0, 0, 0, formatted)));
                            }
                        }
                    }
                    _ => {}
                }
                (c_code, c_name, c_curr, None)
            });
        }

        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            if let Ok(item) = res {
                results.push(item);
            }
        }

        // 汇率快照获取
        let rates_snap = self.fx.rates().await.ok();

        let mut title = format!("Steam 游戏 (AppID: {appid})");
        let header_image = format!("https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{appid}/header.jpg");
        let mut region_prices = Vec::new();
        let mut cn_cny_cents = None;

        for (code, name, currency, data) in &results {
            if let Some((t, raw_final, raw_init, disc, fmt)) = data {
                if !t.is_empty() && title.starts_with("Steam 游戏") {
                    title = t.clone();
                }

                let cny_cents = if code == "cn" {
                    cn_cny_cents = Some(*raw_final);
                    Some(*raw_final)
                } else if *raw_final == 0 {
                    Some(0)
                } else {
                    self.convert_to_cny_cents(currency, *raw_final, rates_snap.as_ref())
                };

                region_prices.push(RegionPrice {
                    region_code: code.clone(),
                    region_name: name.clone(),
                    currency: currency.to_string(),
                    raw_final: *raw_final,
                    raw_initial: *raw_init,
                    discount_pct: *disc,
                    formatted_price: fmt.clone(),
                    cny_cents,
                    cny_diff_cents: None,
                    value_ratio: None,
                });
            } else {
                region_prices.push(RegionPrice {
                    region_code: code.clone(),
                    region_name: name.clone(),
                    currency: currency.to_string(),
                    raw_final: 0,
                    raw_initial: 0,
                    discount_pct: 0,
                    formatted_price: "网络异常/不可查".into(),
                    cny_cents: None,
                    cny_diff_cents: None,
                    value_ratio: None,
                });
            }
        }

        // 排序：国区第一，随后按折算人民币由低到高排列
        region_prices.sort_by(|a, b| {
            if a.region_code == "cn" {
                std::cmp::Ordering::Less
            } else if b.region_code == "cn" {
                std::cmp::Ordering::Greater
            } else {
                let a_val = a.cny_cents.unwrap_or(u64::MAX);
                let b_val = b.cny_cents.unwrap_or(u64::MAX);
                a_val.cmp(&b_val)
            }
        });

        // 计算差价与性价比
        let mut lowest_region = None;
        let mut lowest_cny_cents = None;

        if let Some(cn_val) = cn_cny_cents {
            if cn_val > 0 {
                for r in &mut region_prices {
                    if let Some(r_cny) = r.cny_cents {
                        if r_cny > 0 {
                            let diff = cn_val as i64 - r_cny as i64;
                            r.cny_diff_cents = Some(diff);
                            if diff > 0 {
                                let ratio = diff as f64 / r_cny as f64;
                                r.value_ratio = Some((ratio * 100.0).round() / 100.0);
                            }
                            if r.region_code != "cn"
                                && (lowest_cny_cents.is_none() || r_cny < lowest_cny_cents.unwrap())
                            {
                                lowest_cny_cents = Some(r_cny);
                                lowest_region = Some(r.region_name.clone());
                            }
                        }
                    }
                }
            }
        }

        let max_savings_pct = match (cn_cny_cents, lowest_cny_cents) {
            (Some(cn), Some(low)) if cn > low && cn > 0 => {
                Some(((cn - low) as f64 / cn as f64 * 100.0).round())
            }
            _ => None,
        };

        Ok(GameDealComparison {
            appid,
            title,
            header_image,
            cn_cny_cents,
            lowest_region,
            lowest_cny_cents,
            max_savings_pct,
            regions: region_prices,
            queried_at: now(),
        })
    }

    fn convert_to_cny_cents(
        &self,
        currency: &str,
        raw_amount: u64,
        snap: Option<&crate::domain::fx::FxSnapshot>,
    ) -> Option<u64> {
        let curr_upper = currency.to_ascii_uppercase();
        if curr_upper == "CNY" {
            return Some(raw_amount);
        }

        if let Some(s) = snap {
            if let Some(rate_info) = s.rates.get(&curr_upper) {
                let rate = rate_info.units_per_cny_1e8 as f64 / 1e8;
                if rate > 0.0 {
                    let cny_cents = (raw_amount as f64 / rate).round() as u64;
                    return Some(cny_cents);
                }
            }
        }
        None
    }
}
