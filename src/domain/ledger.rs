//! Purchase facts and explicit asset identity. Manual reports never become platform verification.
use super::{
    fx,
    inventory::{InventoryAsset, InventorySnapshot},
    money::Money,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub fn text_id(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LotRequest {
    pub client_key: String,
    pub item_key: String,
    pub quantity: u32,
    pub cash_cost_cents: u64,
    pub purchased_at: u64,
    pub purchase_ref: String,
}
impl LotRequest {
    pub fn validate(&self, at: u64) -> Result<(), String> {
        if !text_id(&self.client_key, 64)
            || !text_id(&self.item_key, 500)
            || !text_id(&self.purchase_ref, 128)
            || !(1..=1000).contains(&self.quantity)
            || self.cash_cost_cents < u64::from(self.quantity)
            || self.cash_cost_cents > 1_000_000_000_000
            || self.purchased_at == 0
            || self.purchased_at > at
        {
            return Err("采购需唯一请求键/采购凭据、商品身份、1–1000件、足够按件分摊的CNY总成本及非未来时间".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetStage {
    Held,
    Listed,
    Sold,
}
impl AssetStage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Held => "持有",
            Self::Listed => "已挂",
            Self::Sold => "已售",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListingReport {
    pub listing_ref: String,
    pub gross: Money,
    pub listed_at: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaleReport {
    pub sale_ref: String,
    pub net_wallet: Money,
    pub sold_at: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LotAsset {
    pub asset: InventoryAsset,
    pub inventory_snapshot_id: String,
    pub received_at: u64,
    pub state_observed_at: u64,
    pub cost_cents: u64,
    pub stage: AssetStage,
    pub listing: Option<ListingReport>,
    pub sale: Option<SaleReport>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseLot {
    pub id: String,
    pub account_id: String,
    pub revision: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub provenance: String,
    pub platform_verified: bool,
    pub request: LotRequest,
    pub assets: Vec<LotAsset>,
}
impl PurchaseLot {
    pub fn new(account_id: String, request: LotRequest, at: u64) -> Result<Self, String> {
        request.validate(at)?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            account_id,
            revision: 1,
            created_at: at,
            updated_at: at,
            provenance: "manual_unverified".into(),
            platform_verified: false,
            request,
            assets: vec![],
        })
    }
    pub fn pending_units(&self) -> u32 {
        self.request.quantity - self.assets.len() as u32
    }
    pub fn pending_cost_cents(&self) -> u64 {
        self.request.cash_cost_cents - self.assets.iter().map(|a| a.cost_cents).sum::<u64>()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LotAction {
    Receive {
        inventory_snapshot_id: String,
        asset_ids: Vec<String>,
    },
    List {
        asset_id: String,
        listing_ref: String,
        gross: Money,
        listed_at: u64,
    },
    CancelListing {
        asset_id: String,
        listing_ref: String,
        cancelled_at: u64,
    },
    Sell {
        asset_id: String,
        listing_ref: String,
        sale_ref: String,
        net_wallet: Money,
        sold_at: u64,
    },
}
impl LotAction {
    pub fn external_reference(&self) -> Option<(&'static str, &str)> {
        match self {
            Self::List { listing_ref, .. } => Some(("listing", listing_ref)),
            Self::Sell { sale_ref, .. } => Some(("sale", sale_ref)),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LotEventRequest {
    pub client_key: String,
    pub expected_revision: u32,
    pub action: LotAction,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LotEvent {
    pub id: String,
    pub lot_id: String,
    pub account_id: String,
    pub recorded_at: u64,
    pub request: LotEventRequest,
    pub result_revision: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerWrite {
    pub lot: PurchaseLot,
    pub replayed: bool,
}
pub enum LedgerOutcome {
    Saved(LedgerWrite),
    Missing,
    Conflict(&'static str),
    Invalid(String),
}

/// The storage adapter invokes this inside its atomic revision/claim transaction.
pub fn apply(
    lot: &PurchaseLot,
    req: &LotEventRequest,
    inventory: Option<&InventorySnapshot>,
    at: u64,
) -> Result<PurchaseLot, String> {
    if !text_id(&req.client_key, 64) || req.expected_revision != lot.revision {
        return Err("请求键或批次版本不匹配".into());
    }
    let mut result = lot.clone();
    match &req.action {
        LotAction::Receive {
            inventory_snapshot_id,
            asset_ids,
        } => {
            let inv = inventory.ok_or("缺少显式库存快照")?;
            if inv.id != *inventory_snapshot_id
                || inv.account_id != lot.account_id
                || inv.observed_at > at
                || inv.observed_at < lot.request.purchased_at
                || asset_ids.is_empty()
                || asset_ids.len() > lot.pending_units() as usize
            {
                return Err("收货快照账户、时间或数量不匹配".into());
            }
            let mut seen = HashSet::new();
            for id in asset_ids {
                if !seen.insert(id) || result.assets.iter().any(|a| a.asset.asset_id == *id) {
                    return Err("收货资产重复".into());
                }
                let a = inv
                    .assets
                    .iter()
                    .find(|a| a.asset_id == *id)
                    .ok_or("资产不在所选库存快照")?;
                if !a.validate("2")
                    || a.amount != 1
                    || a.market_hash_name.as_deref() != Some(lot.request.item_key.as_str())
                {
                    return Err(
                        "仅绑定CS2普通库存中精确匹配商品的单件资产，不按同名猜测或拆分堆叠资产"
                            .into(),
                    );
                }
                let index = result.assets.len() as u64;
                let count = u64::from(lot.request.quantity);
                let cost = lot.request.cash_cost_cents / count
                    + u64::from(index < lot.request.cash_cost_cents % count);
                result.assets.push(LotAsset {
                    asset: a.clone(),
                    inventory_snapshot_id: inv.id.clone(),
                    received_at: at,
                    state_observed_at: at,
                    cost_cents: cost,
                    stage: AssetStage::Held,
                    listing: None,
                    sale: None,
                });
            }
        }
        action => {
            let (id, reference, event_at) = match action {
                LotAction::List {
                    asset_id,
                    listing_ref,
                    listed_at,
                    ..
                } => (asset_id, listing_ref, *listed_at),
                LotAction::CancelListing {
                    asset_id,
                    listing_ref,
                    cancelled_at,
                } => (asset_id, listing_ref, *cancelled_at),
                LotAction::Sell {
                    asset_id,
                    listing_ref,
                    sold_at,
                    ..
                } => (asset_id, listing_ref, *sold_at),
                LotAction::Receive { .. } => unreachable!(),
            };
            if !text_id(reference, 128) || event_at > at {
                return Err("挂牌凭据或操作时间无效".into());
            }
            let a = result
                .assets
                .iter_mut()
                .find(|a| a.asset.asset_id == *id)
                .ok_or("资产不属于此采购批次")?;
            if event_at < a.state_observed_at {
                return Err("操作不能早于当前资产状态时间".into());
            }
            match action {
                LotAction::List { gross, .. } => {
                    if a.stage != AssetStage::Held
                        || gross.minor == 0
                        || gross.minor > 1_000_000_000_000
                        || !fx::supported(&gross.currency)
                    {
                        return Err("只有持有资产可以登记挂牌；需明确支持的原币毛价".into());
                    }
                    a.stage = AssetStage::Listed;
                    a.listing = Some(ListingReport {
                        listing_ref: reference.clone(),
                        gross: gross.clone(),
                        listed_at: event_at,
                    });
                }
                LotAction::CancelListing { .. } | LotAction::Sell { .. } => {
                    let listing = a
                        .listing
                        .as_ref()
                        .filter(|l| a.stage == AssetStage::Listed && l.listing_ref == *reference)
                        .ok_or("没有匹配的当前挂牌")?;
                    if event_at < listing.listed_at {
                        return Err("撤单/成交时间不能早于挂牌".into());
                    }
                    if let LotAction::Sell {
                        sale_ref,
                        net_wallet,
                        ..
                    } = action
                    {
                        if !text_id(sale_ref, 128)
                            || net_wallet.currency != listing.gross.currency
                            || net_wallet.minor == 0
                            || net_wallet.minor > listing.gross.minor
                        {
                            return Err(
                                "成交需唯一凭据及同币种实际净钱包额，不能套固定费率或超过挂牌毛价"
                                    .into(),
                            );
                        }
                        a.stage = AssetStage::Sold;
                        a.sale = Some(SaleReport {
                            sale_ref: sale_ref.clone(),
                            net_wallet: net_wallet.clone(),
                            sold_at: event_at,
                        });
                    } else {
                        a.stage = AssetStage::Held;
                        a.listing = None;
                    }
                }
                LotAction::Receive { .. } => unreachable!(),
            }
            a.state_observed_at = event_at;
        }
    }
    result.revision = lot.revision.checked_add(1).ok_or("版本上限")?;
    result.updated_at = at;
    Ok(result)
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
pub(crate) mod tests;
