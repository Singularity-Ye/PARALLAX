//! Fixed-host, read-only inventory protocol. No raw response, item HTML, or credential logging.
use super::steam_read::classify;
use crate::{
    application::ports::{InventoryProvider, SteamReadError as E},
    domain::{accounts::Credentials, inventory::*},
};
use reqwest::Client;
use serde_json::Value;
use std::{collections::HashMap, time::Duration};

pub struct SteamInventory {
    client: Client,
}
impl SteamInventory {
    pub fn new() -> anyhow::Result<Self> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
}
fn id(value: &Value, zero: bool) -> Result<String, E> {
    value
        .as_str()
        .filter(|s| numeric_id(s, zero))
        .map(str::to_owned)
        .ok_or(E::InvalidPayload)
}
fn flag(value: &Value) -> Result<bool, E> {
    match value.as_u64() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(E::InvalidPayload),
    }
}
#[derive(PartialEq, Eq)]
struct Description {
    name: Option<String>,
    tradable: bool,
    marketable: bool,
    restricted: bool,
}

fn parse(body: &[u8], context: &str) -> Result<InventoryPage, E> {
    let data: Value = serde_json::from_slice(body).map_err(|_| E::InvalidPayload)?;
    if data["success"].as_u64() != Some(1) {
        return Err(E::Rejected);
    }
    let total = data["total_inventory_count"]
        .as_u64()
        .filter(|n| *n <= 1_000_000)
        .ok_or(E::InvalidPayload)? as u32;
    let more = match data.get("more_items") {
        None => false,
        Some(v) => flag(v)?,
    };
    let empty = Vec::new();
    let raw_assets = match data.get("assets") {
        Some(v) => v.as_array().ok_or(E::InvalidPayload)?,
        None if total == 0 && !more => &empty,
        _ => return Err(E::InvalidPayload),
    };
    if raw_assets.len() > 75 {
        return Err(E::InvalidPayload);
    }
    let mut descriptions = HashMap::new();
    let raw_descriptions = match data.get("descriptions") {
        Some(v) => v
            .as_array()
            .filter(|a| a.len() <= 1000)
            .ok_or(E::InvalidPayload)?,
        None if raw_assets.is_empty() => &empty,
        _ => return Err(E::InvalidPayload),
    };
    for d in raw_descriptions {
        if d["appid"].as_u64() != Some(730) {
            return Err(E::InvalidPayload);
        }
        let key = (id(&d["classid"], false)?, id(&d["instanceid"], true)?);
        let name = match d.get("market_hash_name") {
            None | Some(Value::Null) => None,
            Some(v) => Some(
                v.as_str()
                    .filter(|n| !n.is_empty() && n.len() <= 500 && !n.chars().any(char::is_control))
                    .ok_or(E::InvalidPayload)?
                    .into(),
            ),
        };
        // Preserve the existence of owner restrictions, without storing HTML or inferring unlock time.
        let restricted = match d.get("owner_descriptions") {
            None => false,
            Some(v) => !v
                .as_array()
                .filter(|a| a.len() <= 100)
                .ok_or(E::InvalidPayload)?
                .is_empty(),
        };
        let desc = Description {
            name,
            tradable: flag(&d["tradable"])?,
            marketable: flag(&d["marketable"])?,
            restricted,
        };
        // Equal duplicates are allowed; conflicting metadata may not override silently.
        if descriptions.get(&key).is_some_and(|old| old != &desc) {
            return Err(E::InvalidPayload);
        }
        descriptions.insert(key, desc);
    }
    let mut assets = Vec::new();
    for a in raw_assets {
        if a["appid"].as_u64() != Some(730) || a["contextid"].as_str() != Some(context) {
            return Err(E::InvalidPayload);
        }
        let class_id = id(&a["classid"], false)?;
        let instance_id = id(&a["instanceid"], true)?;
        // Never match by name alone or fall back from an instance to instance zero.
        let desc = descriptions
            .get(&(class_id.clone(), instance_id.clone()))
            .ok_or(E::InvalidPayload)?;
        let amount = id(&a["amount"], false)?
            .parse::<u32>()
            .map_err(|_| E::InvalidPayload)?;
        assets.push(InventoryAsset {
            app_id: 730,
            context_id: context.into(),
            asset_id: id(&a["assetid"], false)?,
            class_id,
            instance_id,
            amount,
            market_hash_name: desc.name.clone(),
            reported_tradable: desc.tradable,
            reported_marketable: desc.marketable,
            restriction_note_present: desc.restricted,
        });
    }
    let next_asset_id = if more {
        Some(id(&data["last_assetid"], false)?)
    } else {
        None
    };
    let page = InventoryPage {
        assets,
        total,
        next_asset_id,
    };
    page.validate(context).map_err(|_| E::InvalidPayload)?;
    Ok(page)
}
impl InventoryProvider for SteamInventory {
    async fn page(
        &self,
        steam_id: &str,
        context: &str,
        cursor: Option<&str>,
        secret: &Credentials,
    ) -> Result<InventoryPage, E> {
        if steam_id.len() != 17
            || !numeric_id(steam_id, false)
            || !["2", "16"].contains(&context)
            || cursor.is_some_and(|s| !numeric_id(s, false))
        {
            return Err(E::InvalidPayload);
        }
        let mut params = vec![
            ("l", "english"),
            ("count", "75"),
            ("preserve_bbcode", "1"),
            ("raw_asset_properties", "1"),
        ];
        if let Some(cursor) = cursor {
            params.push(("start_assetid", cursor));
        }
        let mut response = self
            .client
            .get(format!(
                "https://steamcommunity.com/inventory/{steam_id}/730/{context}"
            ))
            .query(&params)
            .header("Cookie", &secret.cookies)
            .header(
                "User-Agent",
                if secret.user_agent.is_empty() {
                    "Mozilla/5.0 Parallax/0.1"
                } else {
                    &secret.user_agent
                },
            )
            .send()
            .await
            .map_err(|_| E::Network)?;
        classify(response.status(), response.headers(), secret)?;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| E::Network)? {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err(E::InvalidPayload);
            }
            bytes.extend_from_slice(&chunk);
        }
        parse(&bytes, context)
    }
}

#[cfg(test)]
#[path = "steam_inventory_tests.rs"]
mod tests;
