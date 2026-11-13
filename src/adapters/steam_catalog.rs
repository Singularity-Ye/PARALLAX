use crate::{
    application::ports::{CatalogError, CatalogProvider},
    domain::{catalog::*, selection::now},
};
use serde_json::Value;
pub struct SteamCatalog {
    client: reqwest::Client,
}
impl SteamCatalog {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: super::network::client_builder()
                .timeout(std::time::Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("Mozilla/5.0 Parallax/0.1")
                .build()?,
        })
    }
}
impl CatalogProvider for SteamCatalog {
    async fn search(&self, request: &CatalogRequest) -> Result<CatalogPage, CatalogError> {
        let response = self
            .client
            .get("https://steamcommunity.com/market/search/render/")
            .query(&[
                ("appid", "730".to_string()),
                ("query", request.query.trim().into()),
                ("start", request.start.to_string()),
                ("count", "10".into()),
                ("norender", "1".into()),
                ("search_descriptions", "0".into()),
                ("sort_column", "name".into()),
                ("sort_dir", "asc".into()),
            ])
            .send()
            .await
            .map_err(|_| CatalogError::Upstream("Steam检索网络失败或超时".into()))?;
        if response.status().as_u16() == 429 {
            return Err(CatalogError::RateLimited(now() + 300));
        }
        if !response.status().is_success() {
            return Err(CatalogError::Upstream(format!(
                "Steam检索HTTP {}，未当作空结果",
                response.status()
            )));
        }
        let value = response
            .json::<Value>()
            .await
            .map_err(|_| CatalogError::Upstream("Steam检索未返回JSON".into()))?;
        parse(&value, request).map_err(CatalogError::Upstream)
    }
}
fn parse(v: &Value, request: &CatalogRequest) -> Result<CatalogPage, String> {
    if v["success"].as_bool() != Some(true) || v["start"].as_u64() != Some(request.start.into()) {
        return Err("Steam检索失败或分页起点不匹配".into());
    }
    let total = v["total_count"].as_u64().ok_or("Steam检索缺少总数")?;
    let rows = v["results"].as_array().ok_or("Steam检索缺少结果数组")?;
    if rows.len() > 100 || (rows.is_empty() && u64::from(request.start) < total) {
        return Err("Steam检索分页不一致，未当作已遍历".into());
    }
    let mut keys = std::collections::HashSet::new();
    let mut items = Vec::new();
    for row in rows {
        let asset = &row["asset_description"];
        let key = row["hash_name"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.len() <= 500)
            .ok_or("Steam检索缺少市场名")?;
        if asset["appid"].as_u64() != Some(730)
            || asset["market_hash_name"].as_str() != Some(key)
            || !keys.insert(key)
        {
            return Err("Steam检索商品身份不一致或重复".into());
        }
        let icon_url = asset["icon_url"]
            .as_str()
            .or_else(|| asset["icon_url_large"].as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|raw| {
                if raw.starts_with("http://") || raw.starts_with("https://") {
                    raw.to_string()
                } else {
                    format!(
                        "https://community.fastly.steamstatic.com/economy/image/{raw}/96fx96f"
                    )
                }
            });
        items.push(CatalogItem {
            market_hash_name: key.into(),
            name: row["name"].as_str().unwrap_or(key).into(),
            sell_listings: row["sell_listings"].as_u64(),
            icon_url,
        });
    }
    let next = request.start + rows.len() as u32;
    Ok(CatalogPage {
        query: request.query.trim().into(),
        start: request.start,
        received: rows.len(),
        upstream_total: total,
        next_start: if u64::from(next) < total && next <= 1000 {
            Some(next)
        } else {
            None
        },
        items,
        observed_at: now(),
        source: "steam_community_market_search".into(),
        scope: "cs2_keyword_single_page_not_full_catalog_no_cash_quotes".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn catalog_preserves_scope_identity_and_pagination_without_prices() {
        let r = CatalogRequest {
            query: " Case ".into(),
            start: 0,
        };
        let mut value = json!({"success":true,"start":0,"total_count":12,"results":[{"hash_name":"Prisma 2 Case","name":"棱彩2号武器箱","sell_price":123,"asset_description":{"appid":730,"market_hash_name":"Prisma 2 Case"}}]});
        let page = parse(&value, &r).unwrap();
        assert_eq!(page.next_start, Some(1));
        assert_eq!(page.query, "Case");
        assert_eq!(page.items[0].sell_listings, None);
        assert!(
            serde_json::to_value(page).unwrap()["items"][0]
                .get("sell_price")
                .is_none()
        );
        value["results"][0]["asset_description"]["appid"] = json!(753);
        assert!(parse(&value, &r).is_err());
        assert!(
            parse(
                &json!({"success":false,"start":0,"total_count":0,"results":[]}),
                &r
            )
            .is_err()
        );
        assert!(
            parse(
                &json!({"success":true,"start":0,"total_count":2,"results":[]}),
                &r
            )
            .is_err()
        );
        assert_eq!(
            parse(
                &json!({"success":true,"start":0,"total_count":0,"results":[]}),
                &r
            )
            .unwrap()
            .next_start,
            None
        );
    }
}
