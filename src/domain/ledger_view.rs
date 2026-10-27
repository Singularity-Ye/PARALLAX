//! Read-only dashboard projection of the existing ledger; never invents market quotes or cash profit.
use super::{holdings::Holdings, ledger::AssetStage, money::Money};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metric<T> {
    pub value: Option<T>,
    pub reason: Option<String>,
}
impl<T> Metric<T> {
    fn unavailable(reason: &str) -> Self {
        Self {
            value: None,
            reason: Some(reason.into()),
        }
    }
    fn known(value: T) -> Self {
        Self {
            value: Some(value),
            reason: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedMoney {
    pub minor: i64,
    pub currency: super::money::Currency,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LedgerCounts {
    pub lots: usize,
    pub pending_units: u64,
    pub held_units: u64,
    pub listed_units: u64,
    pub sold_units: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerSummary {
    pub account_id: String,
    pub generated_at: u64,
    pub scope: String,
    pub complete: bool,
    pub platform_verified: bool,
    pub counts: LedgerCounts,
    pub reported_purchase_cost: Money,
    pub open_purchase_cost: Money,
    pub sold_purchase_cost: Money,
    pub reported_sale_net_wallet: Vec<Money>,
    pub total_cash_profit: Metric<SignedMoney>,
    pub actual_discount_bps: Metric<u64>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerRow {
    pub row_id: String,
    pub lot_id: String,
    pub lot_revision: u32,
    pub purchased_at: u64,
    pub state_observed_at: u64,
    pub item_key: String,
    pub asset_id: Option<String>,
    pub stage: String,
    pub quantity: u64,
    /// Cost of this row's entire quantity, not a rounded per-unit average.
    pub purchase_cost: Money,
    pub purchase_market_price: Metric<Money>,
    pub current_market_price: Metric<Money>,
    pub reported_listing_gross: Option<Money>,
    pub sale_net_wallet: Metric<Money>,
    pub actual_discount_bps: Metric<u64>,
    pub cash_profit: Metric<SignedMoney>,
    pub self_use_profit: Metric<SignedMoney>,
    pub market_change: Metric<SignedMoney>,
    pub provenance: String,
    pub platform_verified: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerRows {
    pub account_id: String,
    pub generated_at: u64,
    pub scope: String,
    pub complete: bool,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
    pub sort: String,
    pub items: Vec<LedgerRow>,
    pub warnings: Vec<String>,
}
pub fn summary(h: &Holdings) -> Result<LedgerSummary, String> {
    let mut counts = LedgerCounts {
        lots: h.lots.len(),
        ..Default::default()
    };
    for item in &h.items {
        counts.pending_units += item.pending_units;
        counts.held_units += item.held_units;
        counts.listed_units += item.listed_units;
        counts.sold_units += item.sold_units;
    }
    let purchase = h.lots.iter().try_fold(0u64, |sum, l| {
        sum.checked_add(l.request.cash_cost_cents)
            .ok_or("登记采购合计溢出")
    })?;
    Ok(LedgerSummary {
        account_id: h.account_id.clone(),
        generated_at: h.generated_at,
        scope: "account_reported_ledger_only".into(),
        complete: false,
        platform_verified: false,
        counts,
        reported_purchase_cost: Money::cny(purchase),
        open_purchase_cost: Money::cny(h.open_cost_cents),
        sold_purchase_cost: Money::cny(h.sold_cost_cents),
        reported_sale_net_wallet: h.reported_wallet_receipts.clone(),
        total_cash_profit: Metric::unavailable(
            "只有采购成本与分币种实报Steam净钱包；缺少可提现现金结算，不能计算现金净利润",
        ),
        actual_discount_bps: Metric::unavailable(
            "缺少统一口径的已实现结算/汇率证据，不以采购总成本除以出售总额",
        ),
        warnings: h.warnings.clone(),
    })
}
pub fn rows(h: &Holdings, offset: usize, limit: usize) -> Result<LedgerRows, String> {
    if limit == 0 || limit > 200 || offset > 10_000_000 {
        return Err("明细limit须为1–200，offset至多10000000".into());
    }
    let mut all = Vec::new();
    for lot in &h.lots {
        let mut make = |asset_id: Option<String>,
                        stage: &str,
                        quantity: u64,
                        cost: u64,
                        state_at: u64,
                        listing: Option<Money>,
                        net: Option<Money>| {
            all.push(LedgerRow {
                row_id: format!("{}:{}", lot.id, asset_id.as_deref().unwrap_or("pending")),
                lot_id: lot.id.clone(),
                lot_revision: lot.revision,
                purchased_at: lot.request.purchased_at,
                state_observed_at: state_at,
                item_key: lot.request.item_key.clone(),
                asset_id,
                stage: stage.into(),
                quantity,
                purchase_cost: Money::cny(cost),
                reported_listing_gross: listing,
                purchase_market_price: Metric::unavailable("采购登记未包含当时市场报价"),
                current_market_price: Metric::unavailable(
                    "未显式引用当前有效报价，不用历史价或挂牌价冒充现价",
                ),
                sale_net_wallet: net
                    .map(Metric::known)
                    .unwrap_or_else(|| Metric::unavailable("未登记成交净钱包；不是税后估值")),
                actual_discount_bps: Metric::unavailable("未引用匹配的结算/汇率证据"),
                cash_profit: Metric::unavailable("Steam净钱包不等于可提现现金"),
                self_use_profit: Metric::unavailable("未定义自用估值与对应费用/汇率"),
                market_change: Metric::unavailable("缺少同口径采购时与当前市场报价"),
                provenance: lot.provenance.clone(),
                platform_verified: lot.platform_verified,
            });
        };
        if lot.pending_units() > 0 {
            make(
                None,
                "pending_receipt",
                u64::from(lot.pending_units()),
                lot.pending_cost_cents(),
                lot.updated_at,
                None,
                None,
            );
        }
        for a in &lot.assets {
            make(
                Some(a.asset.asset_id.clone()),
                match a.stage {
                    AssetStage::Held => "held",
                    AssetStage::Listed => "listed",
                    AssetStage::Sold => "sold",
                },
                1,
                a.cost_cents,
                a.state_observed_at,
                a.listing.as_ref().map(|l| l.gross.clone()),
                a.sale.as_ref().map(|s| s.net_wallet.clone()),
            );
        }
    }
    all.sort_by(|a, b| {
        b.purchased_at
            .cmp(&a.purchased_at)
            .then_with(|| a.row_id.cmp(&b.row_id))
    });
    let total = all.len();
    let end = offset.saturating_add(limit).min(total);
    let items = all.into_iter().skip(offset).take(limit).collect();
    Ok(LedgerRows {
        account_id: h.account_id.clone(),
        generated_at: h.generated_at,
        scope: "account_reported_ledger_only".into(),
        complete: false,
        offset,
        limit,
        total,
        next_offset: if end < total { Some(end) } else { None },
        sort: "purchased_at_desc_row_id_asc".into(),
        items,
        warnings: h.warnings.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        holdings,
        ledger::{
            SaleReport,
            tests::{fixture, receive},
        },
        money::Currency,
    };
    #[test]
    fn dashboard_keeps_mixed_wallet_currencies_and_pending_costs_separate() {
        let (lot, inv) = fixture(1000);
        let mut lot = receive(&lot, &inv, &["1", "2"], 1000);
        lot.assets[0].stage = AssetStage::Sold;
        lot.assets[0].sale = Some(SaleReport {
            sale_ref: "sale-a".into(),
            net_wallet: Money {
                minor: 4300,
                currency: Currency::try_from("UAH".to_string()).unwrap(),
            },
            sold_at: 1000,
        });
        lot.assets[1].stage = AssetStage::Sold;
        lot.assets[1].sale = Some(SaleReport {
            sale_ref: "sale-b".into(),
            net_wallet: Money::cny(450),
            sold_at: 1000,
        });
        let h = holdings::summarize("fixture-account", vec![lot], None, 1000).unwrap();
        let sum = summary(&h).unwrap();
        assert_eq!(sum.reported_purchase_cost.minor, 1000);
        assert_eq!(sum.sold_purchase_cost.minor, 667);
        assert_eq!(sum.open_purchase_cost.minor, 333);
        assert_eq!(sum.counts.pending_units, 1);
        assert_eq!(sum.counts.sold_units, 2);
        assert_eq!(sum.reported_sale_net_wallet.len(), 2);
        assert!(sum.total_cash_profit.value.is_none());
        assert!(sum.actual_discount_bps.value.is_none());
        let all = rows(&h, 0, 200).unwrap();
        assert_eq!(all.total, 3);
        assert_eq!(
            all.items.iter().map(|r| r.purchase_cost.minor).sum::<u64>(),
            1000
        );
        assert!(
            all.items
                .iter()
                .all(|r| r.current_market_price.value.is_none() && r.cash_profit.value.is_none())
        );
        let first = rows(&h, 0, 1).unwrap();
        let second = rows(&h, 1, 1).unwrap();
        assert_eq!(first.next_offset, Some(1));
        assert_ne!(first.items[0].row_id, second.items[0].row_id);
        assert_eq!(rows(&h, 999, 1).unwrap().next_offset, None);
        assert!(rows(&h, 0, 0).is_err());
        assert!(rows(&h, 0, 201).is_err());
    }
    #[test]
    fn empty_ledger_is_zero_registered_cost_not_verified_zero_account_assets() {
        let h = holdings::summarize("a", vec![], None, 1).unwrap();
        let sum = summary(&h).unwrap();
        assert_eq!(sum.reported_purchase_cost.minor, 0);
        assert!(!sum.complete);
        assert!(!sum.platform_verified);
        assert!(sum.total_cash_profit.value.is_none());
        assert!(sum.actual_discount_bps.value.is_none());
        assert!(rows(&h, 0, 50).unwrap().items.is_empty());
    }
}
