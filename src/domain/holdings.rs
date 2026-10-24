//! Reconciles observations with reported lots without double counting or inventing zero holdings.
use super::{inventory::InventorySnapshot, ledger::*, money::Money};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HoldingItem {
    pub item_key: String,
    pub pending_units: u64,
    pub held_units: u64,
    pub listed_units: u64,
    pub sold_units: u64,
    pub observed_untracked_units: u64,
    pub open_cost_cents: u64,
    pub sold_cost_cents: u64,
    pub discrepancies: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Holdings {
    pub account_id: String,
    pub generated_at: u64,
    pub provenance: String,
    pub complete: bool,
    pub inventory: Option<InventorySnapshot>,
    pub lots: Vec<PurchaseLot>,
    pub items: Vec<HoldingItem>,
    pub open_cost_cents: u64,
    pub sold_cost_cents: u64,
    pub reported_wallet_receipts: Vec<Money>,
    pub warnings: Vec<String>,
}
pub fn summarize(
    account: &str,
    lots: Vec<PurchaseLot>,
    inventory: Option<InventorySnapshot>,
    at: u64,
) -> Result<Holdings, String> {
    if lots.iter().any(|l| l.account_id != account)
        || inventory
            .as_ref()
            .is_some_and(|i| i.account_id != account || i.observed_at > at)
    {
        return Err("持仓来源账户或时间不匹配".into());
    }
    let mut items: BTreeMap<String, HoldingItem> = BTreeMap::new();
    let mut claimed = HashSet::new();
    let mut receipts: BTreeMap<String, Money> = BTreeMap::new();
    for lot in &lots {
        let row = items
            .entry(lot.request.item_key.clone())
            .or_insert_with(|| HoldingItem {
                item_key: lot.request.item_key.clone(),
                ..Default::default()
            });
        row.pending_units += u64::from(lot.pending_units());
        row.open_cost_cents += lot.pending_cost_cents();
        for a in &lot.assets {
            if !claimed.insert(a.asset.asset_id.clone()) {
                return Err("台账含重复资产身份，拒绝重复汇总".into());
            }
            match a.stage {
                AssetStage::Held => {
                    row.held_units += 1;
                    row.open_cost_cents += a.cost_cents;
                }
                AssetStage::Listed => {
                    row.listed_units += 1;
                    row.open_cost_cents += a.cost_cents;
                }
                AssetStage::Sold => {
                    row.sold_units += 1;
                    row.sold_cost_cents += a.cost_cents;
                    let money = &a.sale.as_ref().ok_or("已售资产缺少净钱包凭据")?.net_wallet;
                    let key = String::from(money.currency.clone());
                    let total = receipts.entry(key).or_insert_with(|| Money {
                        minor: 0,
                        currency: money.currency.clone(),
                    });
                    *total = total.checked_add(money)?;
                }
            }
            if let Some(inv) = &inventory {
                let observed = inv.assets.iter().find(|i| i.asset_id == a.asset.asset_id);
                match observed {
                    Some(i)
                        if i.market_hash_name.as_deref() != Some(&lot.request.item_key)
                            || i.class_id != a.asset.class_id
                            || i.instance_id != a.asset.instance_id =>
                    {
                        row.discrepancies
                            .push(format!("资产 {} 观察身份不符", a.asset.asset_id))
                    }
                    Some(_) if a.stage != AssetStage::Held => row.discrepancies.push(format!(
                        "资产 {} 在库存可见但台账为{}，需对账",
                        a.asset.asset_id,
                        a.stage.label()
                    )),
                    None if a.stage == AssetStage::Held && inv.coverage_complete() => {
                        row.discrepancies.push(format!(
                            "持有资产 {} 未见于完整观察；不自动记卖出",
                            a.asset.asset_id
                        ))
                    }
                    _ => {}
                }
            }
        }
    }
    let mut warnings = vec![
        "台账由人工凭据登记，尚未与真实订单/挂牌/成交自动对账，当前不是完整账户持仓".into(),
        "CNY采购成本与分币种Steam净钱包分别列示，不把钱包余额当可提现现金收益".into(),
    ];
    if let Some(inv) = &inventory {
        if !inv.coverage_complete() {
            warnings.push("库存观察范围不完整，未出现不等于零".into());
        }
        let mut observed = HashSet::new();
        for a in &inv.assets {
            if !observed.insert(&a.asset_id) {
                return Err("观察存在跨区重复资产".into());
            }
            if claimed.contains(&a.asset_id) {
                continue;
            }
            match &a.market_hash_name {
                Some(key) => {
                    let row = items.entry(key.clone()).or_insert_with(|| HoldingItem {
                        item_key: key.clone(),
                        ..Default::default()
                    });
                    row.observed_untracked_units += u64::from(a.amount);
                }
                None => warnings.push(format!("未识别资产 {}，不猜名称/成本", a.asset_id)),
            }
        }
    } else {
        warnings.push("未引用库存观察，只汇总已登记台账；未登记不等于零".into());
    }
    let rows: Vec<_> = items.into_values().collect();
    Ok(Holdings {
        account_id: account.into(),
        generated_at: at,
        provenance: "reported_ledger_with_inventory_observation".into(),
        complete: false,
        open_cost_cents: rows.iter().map(|r| r.open_cost_cents).sum(),
        sold_cost_cents: rows.iter().map(|r| r.sold_cost_cents).sum(),
        inventory,
        lots,
        items: rows,
        reported_wallet_receipts: receipts.into_values().collect(),
        warnings,
    })
}
