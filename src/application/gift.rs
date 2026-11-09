use crate::domain::{
    accounts::Platform,
    gift::{GameEdition, GameEditionsResponse, GiftCheckRequest, GiftRiskVerdict, SteamFriend},
};
use super::{
    fx::FxResolver,
    ports::{AccountRepository, CredentialStore},
};
use std::sync::Arc;

pub struct SteamGiftService<A, C> {
    accounts: Arc<A>,
    creds: Arc<C>,
    fx: Arc<dyn FxResolver>,
    http: reqwest::Client,
}

impl<A: AccountRepository + 'static, C: CredentialStore + 'static> SteamGiftService<A, C> {
    pub fn new(accounts: Arc<A>, creds: Arc<C>, fx: Arc<dyn FxResolver>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self { accounts, creds, fx, http }
    }

    pub async fn get_friends(&self, account_id: &str) -> Result<Vec<SteamFriend>, String> {
        let acc = self
            .accounts
            .account(account_id.to_string())
            .await
            .map_err(|e| format!("读取账号失败: {e}"))?
            .ok_or_else(|| "指定账号不存在".to_string())?;

        let session = self
            .accounts
            .session(account_id.to_string(), Platform::Steam)
            .await
            .ok()
            .flatten();

        let cookie_str = if let Some(s) = session {
            if let Ok(Some(cred)) = self.creds.get(s.credential_key).await {
                cred.cookies
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let steam_id = acc.steam_id.clone();
        if steam_id.is_empty() && !cookie_str.contains("steamLoginSecure=") {
            return Err("该账号尚未绑定有效 Steam 会话凭据，请先在账号管理中登录".into());
        }

        let url = if !steam_id.is_empty() {
            format!("https://steamcommunity.com/profiles/{steam_id}/friends/?ajax=1")
        } else {
            "https://steamcommunity.com/my/friends/?ajax=1".to_string()
        };

        let mut req = self.http.get(&url);
        if !cookie_str.is_empty() {
            req = req.header("Cookie", cookie_str);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("请求 Steam 好友页面网络失败: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Steam 返回 HTTP 状态异常: {}", resp.status()));
        }

        let html = resp.text().await.unwrap_or_default();
        let friends = self.parse_friends_html(&html);
        Ok(friends)
    }

    fn parse_friends_html(&self, html: &str) -> Vec<SteamFriend> {
        let mut friends = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 查找所有 data-steamid="(\d{17})"
        let marker = "data-steamid=\"";
        let mut cursor = 0;
        while let Some(pos) = html[cursor..].find(marker) {
            let start = cursor + pos + marker.len();
            if let Some(end) = html[start..].find('"') {
                let steamid = &html[start..start + end];
                if steamid.len() == 17 && steamid.chars().all(|c| c.is_ascii_digit()) {
                    if !seen.contains(steamid) {
                        seen.insert(steamid.to_string());

                        // 在该 steamid 附近寻找昵称与头像
                        let window_end = (start + 600).min(html.len());
                        let chunk = &html[start..window_end];

                        let name = if let Some(c_pos) = chunk.find("class=\"friend_block_content\"") {
                            let content_part = &chunk[c_pos..];
                            if let Some(gt) = content_part.find('>') {
                                let after_gt = &content_part[gt + 1..];
                                if let Some(lt) = after_gt.find('<') {
                                    after_gt[..lt].trim().to_string()
                                } else {
                                    steamid.to_string()
                                }
                            } else {
                                steamid.to_string()
                            }
                        } else {
                            steamid.to_string()
                        };

                        let avatar = if let Some(img_pos) = chunk.find("src=\"") {
                            let after_src = &chunk[img_pos + 5..];
                            if let Some(quote_end) = after_src.find('"') {
                                after_src[..quote_end].to_string()
                            } else {
                                "https://avatars.steamstatic.com/fef49e7fa7e1997310d705b2a6158ff8dc1cdfeb_full.jpg".into()
                            }
                        } else {
                            "https://avatars.steamstatic.com/fef49e7fa7e1997310d705b2a6158ff8dc1cdfeb_full.jpg".into()
                        };

                        friends.push(SteamFriend {
                            steamid: steamid.to_string(),
                            name: if name.is_empty() { steamid.to_string() } else { name },
                            avatar,
                        });
                    }
                }
                cursor = start + end;
            } else {
                break;
            }
        }

        friends
    }

    pub async fn get_editions(&self, appid: u64) -> Result<GameEditionsResponse, String> {
        let url = format!("https://store.steampowered.com/app/{appid}/?l=schinese");
        let resp = self
            .http
            .get(&url)
            .header("Cookie", "birthtime=283993201; wants_mature_content=1; Steam_Language=schinese")
            .send()
            .await
            .map_err(|e| format!("请求 Steam 商店详情失败: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Steam 商店返回 HTTP 状态异常: {}", resp.status()));
        }

        let html = resp.text().await.unwrap_or_default();
        let (title, header_image, editions) = self.parse_editions_html(appid, &html);

        Ok(GameEditionsResponse {
            appid,
            title,
            header_image,
            editions,
        })
    }

    fn parse_editions_html(&self, appid: u64, html: &str) -> (String, String, Vec<GameEdition>) {
        let title = if let Some(pos) = html.find("class=\"apphub_AppName\">") {
            let start = pos + "class=\"apphub_AppName\">".len();
            if let Some(end) = html[start..].find('<') {
                html[start..start + end].trim().to_string()
            } else {
                format!("Steam 游戏 (AppID: {appid})")
            }
        } else {
            format!("Steam 游戏 (AppID: {appid})")
        };

        let header_image = format!("https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{appid}/header.jpg");

        let mut editions = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 提取带有 name="subid" 的购买版本
        let sub_marker = "name=\"subid\" value=\"";
        let mut cursor = 0;
        while let Some(pos) = html[cursor..].find(sub_marker) {
            let start = cursor + pos + sub_marker.len();
            if let Some(end) = html[start..].find('"') {
                let subid = &html[start..start + end];
                if !seen.contains(subid) && !subid.is_empty() {
                    seen.insert(subid.to_string());

                    // 向前向后抓取 600 字符提取名字与价格
                    let chunk_start = if start > 600 { start - 600 } else { 0 };
                    let chunk_end = (start + 800).min(html.len());
                    let chunk = &html[chunk_start..chunk_end];

                    let mut name = if let Some(h_pos) = chunk.find("<h1>") {
                        let h_start = h_pos + 4;
                        if let Some(h_end) = chunk[h_start..].find("</h1>") {
                            chunk[h_start..h_start + h_end].trim().to_string()
                        } else {
                            format!("购买 SubID: {subid}")
                        }
                    } else if let Some(h_pos) = chunk.find("<h2>") {
                        let h_start = h_pos + 4;
                        if let Some(h_end) = chunk[h_start..].find("</h2>") {
                            chunk[h_start..h_start + h_end].trim().to_string()
                        } else {
                            format!("购买 SubID: {subid}")
                        }
                    } else {
                        format!("购买 SubID: {subid}")
                    };

                    if name.to_lowercase().starts_with("buy ") {
                        name = name[4..].trim().to_string();
                    }

                    let price = if let Some(p_pos) = chunk.find("discount_final_price\">") {
                        let p_start = p_pos + "discount_final_price\">".len();
                        if let Some(p_end) = chunk[p_start..].find('<') {
                            chunk[p_start..p_start + p_end].trim().to_string()
                        } else {
                            "详见商店".into()
                        }
                    } else if let Some(p_pos) = chunk.find("game_purchase_price") {
                        let after = &chunk[p_pos..];
                        if let Some(gt) = after.find('>') {
                            let p_start = gt + 1;
                            if let Some(p_end) = after[p_start..].find('<') {
                                after[p_start..p_start + p_end].trim().to_string()
                            } else {
                                "详见商店".into()
                            }
                        } else {
                            "详见商店".into()
                        }
                    } else {
                        "详见商店".into()
                    };

                    let discount_pct = if let Some(d_pos) = chunk.find("discount_pct\">") {
                        let d_start = d_pos + "discount_pct\">".len();
                        if let Some(d_end) = chunk[d_start..].find('<') {
                            chunk[d_start..d_start + d_end].trim().to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    editions.push(GameEdition {
                        id: subid.to_string(),
                        edition_type: "subid".into(),
                        name,
                        price,
                        original_price: String::new(),
                        discount_pct,
                    });
                }
                cursor = start + end;
            } else {
                break;
            }
        }

        if editions.is_empty() {
            editions.push(GameEdition {
                id: format!("{appid}"),
                edition_type: "subid".into(),
                name: "标准游戏本体".into(),
                price: "详见商店".into(),
                original_price: "".into(),
                discount_pct: "".into(),
            });
        }

        (title, header_image, editions)
    }

    pub async fn check_gift_risk(&self, req: GiftCheckRequest) -> Result<GiftRiskVerdict, String> {
        let sender_acc = self
            .accounts
            .account(req.sender_account.clone())
            .await
            .map_err(|e| format!("读取赠送者账号失败: {e}"))?
            .ok_or_else(|| "赠送者账号不存在".to_string())?;

        let sender_region = sender_acc
            .steam_id
            .as_str()
            .is_empty()
            .then(|| "CN".to_string())
            .unwrap_or_else(|| "CN".into())
            .to_ascii_uppercase();

        let recipient_region = req.recipient_region.unwrap_or_else(|| "CN".into()).to_ascii_uppercase();
        let store_url = format!("https://store.steampowered.com/app/{}/", req.appid);

        if sender_region == recipient_region {
            return Ok(GiftRiskVerdict {
                verdict: "SAME_REGION".into(),
                can_gift: true,
                sender_region: sender_region.clone(),
                recipient_region: recipient_region.clone(),
                sender_price_cny: 0.0,
                recipient_price_cny: 0.0,
                diff_pct: 0.0,
                message: format!("双方同属 [{sender_region}] 区域，完全无跨区赠礼限制，直接绿灯通过！"),
                store_url,
            });
        }

        let sender_price = self.query_region_price(req.appid, &sender_region).await?;
        let recipient_price = self.query_region_price(req.appid, &recipient_region).await?;

        let rates_snap = self.fx.rates().await.ok();
        let sender_cny = self.to_cny_float(&sender_price.1, sender_price.0, rates_snap.as_ref());
        let recipient_cny = self.to_cny_float(&recipient_price.1, recipient_price.0, rates_snap.as_ref());

        if sender_cny <= 0.0 || recipient_cny <= 0.0 {
            return Ok(GiftRiskVerdict {
                verdict: "UNKNOWN".into(),
                can_gift: true,
                sender_region,
                recipient_region,
                sender_price_cny: sender_cny,
                recipient_price_cny: recipient_cny,
                diff_pct: 0.0,
                message: "该商品为免费或部分区域未开售，请直接在 Steam 官方客户端尝试加车。".into(),
                store_url,
            });
        }

        // 跨区 10% 规则核验：
        let diff_ratio = (recipient_cny - sender_cny) / recipient_cny;
        let diff_pct = (diff_ratio * 100.0 * 10.0).round() / 10.0;

        if diff_ratio > 0.10 {
            Ok(GiftRiskVerdict {
                verdict: "REJECT_RISK".into(),
                can_gift: false,
                sender_region: sender_region.clone(),
                recipient_region: recipient_region.clone(),
                sender_price_cny: sender_cny,
                recipient_price_cny: recipient_cny,
                diff_pct,
                message: format!(
                    "⚠️ 触发 Steam 10% 跨区风控红线！赠送方折算价 (¥{:.2}) 比受赠方当地原价 (¥{:.2}) 便宜 {:.1}%（超出 10% 容许阈值），Steam 官方网关极大概率报错拒单！建议让好友反向赠礼或由好友自行购买。",
                    sender_cny, recipient_cny, diff_pct
                ),
                store_url,
            })
        } else if diff_ratio < -0.10 {
            Ok(GiftRiskVerdict {
                verdict: "PASS".into(),
                can_gift: true,
                sender_region,
                recipient_region,
                sender_price_cny: sender_cny,
                recipient_price_cny: recipient_cny,
                diff_pct,
                message: format!(
                    "✓ 跨区合规通过！赠送方当地价格 (¥{:.2}) 高于受赠方当地价格 (¥{:.2})，Valve 官方对“多花钱赠礼”完全放行，安全无阻断。",
                    sender_cny, recipient_cny
                ),
                store_url,
            })
        } else {
            Ok(GiftRiskVerdict {
                verdict: "PASS".into(),
                can_gift: true,
                sender_region,
                recipient_region,
                sender_price_cny: sender_cny,
                recipient_price_cny: recipient_cny,
                diff_pct,
                message: format!(
                    "✓ 跨区安全放行！双方折算价差仅为 {:.1}%，在 Steam 官方允许的 10% 浮动红线安全区间内，赠礼成功率 100%！",
                    diff_pct.abs()
                ),
                store_url,
            })
        }
    }

    async fn query_region_price(&self, appid: u64, region: &str) -> Result<(u64, String), String> {
        let cc = region.to_ascii_lowercase();
        let url = format!(
            "https://store.steampowered.com/api/appdetails?appids={appid}&cc={cc}&filters=price_overview"
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("查询区域 [{region}] 价格网络失败: {e}"))?;

        if !resp.status().is_success() {
            return Ok((0, "USD".into()));
        }

        let json: serde_json::Value = resp.json().await.unwrap_or_default();
        let app_obj = &json[appid.to_string()];
        if app_obj["success"].as_bool() == Some(true) {
            let price = &app_obj["data"]["price_overview"];
            if price.is_object() {
                let raw_final = price["final"].as_u64().unwrap_or(0);
                let currency = price["currency"].as_str().unwrap_or("USD").to_string();
                return Ok((raw_final, currency));
            }
        }
        Ok((0, "USD".into()))
    }

    fn to_cny_float(
        &self,
        currency: &str,
        raw_amount: u64,
        snap: Option<&crate::domain::fx::FxSnapshot>,
    ) -> f64 {
        let curr_upper = currency.to_ascii_uppercase();
        if curr_upper == "CNY" {
            return raw_amount as f64 / 100.0;
        }

        if let Some(s) = snap {
            if let Some(rate_info) = s.rates.get(&curr_upper) {
                let rate = rate_info.units_per_cny_1e8 as f64 / 1e8;
                if rate > 0.0 {
                    return (raw_amount as f64 / rate) / 100.0;
                }
            }
        }
        raw_amount as f64 / 100.0
    }
}
