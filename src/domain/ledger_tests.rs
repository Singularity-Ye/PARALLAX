use super::*;
use crate::domain::{
    holdings,
    inventory::{InventoryCoverage, InventoryRequest},
    money::Currency,
};
pub(crate) fn fixture(at: u64) -> (PurchaseLot, InventorySnapshot) {
    let request = LotRequest {
        client_key: "create".into(),
        item_key: "fixture-item".into(),
        quantity: 3,
        cash_cost_cents: 1000,
        purchased_at: at - 100,
        purchase_ref: "fixture-receipt".into(),
    };
    let lot = PurchaseLot::new("fixture-account".into(), request, at).unwrap();
    let assets = (1..=3)
        .map(|i| InventoryAsset {
            app_id: 730,
            context_id: "2".into(),
            asset_id: i.to_string(),
            class_id: "10".into(),
            instance_id: "0".into(),
            amount: 1,
            market_hash_name: Some("fixture-item".into()),
            reported_tradable: true,
            reported_marketable: true,
            restriction_note_present: false,
        })
        .collect();
    let inventory = InventorySnapshot {
        id: "fixture-inventory".into(),
        account_id: lot.account_id.clone(),
        steam_id: "76561198000000001".into(),
        credential_generation: "fixture-generation".into(),
        request: InventoryRequest::default(),
        started_at: at,
        observed_at: at,
        source: "steam_authenticated_inventory".into(),
        platform_verified: false,
        coverage: [("2", 3), ("16", 0)]
            .into_iter()
            .map(|(id, n)| InventoryCoverage {
                context_id: id.into(),
                pages_read: 1,
                reported_total: n,
                observed_assets: n,
                exhausted: true,
                stop_reason: "source_exhausted".into(),
            })
            .collect(),
        assets,
        warnings: vec![],
    };
    (lot, inventory)
}
pub(crate) fn receive(
    lot: &PurchaseLot,
    inv: &InventorySnapshot,
    ids: &[&str],
    at: u64,
) -> PurchaseLot {
    apply(
        lot,
        &LotEventRequest {
            client_key: format!("receive-{}", lot.revision),
            expected_revision: lot.revision,
            action: LotAction::Receive {
                inventory_snapshot_id: inv.id.clone(),
                asset_ids: ids.iter().map(|s| (*s).into()).collect(),
            },
        },
        Some(inv),
        at,
    )
    .unwrap()
}
pub(crate) fn event(lot: &PurchaseLot, action: LotAction, at: u64) -> PurchaseLot {
    apply(
        lot,
        &LotEventRequest {
            client_key: format!("event-{}", lot.revision),
            expected_revision: lot.revision,
            action,
        },
        None,
        at,
    )
    .unwrap()
}
#[test]
fn partial_receipts_conserve_every_cent_and_retain_pending_cost() {
    let (lot, inv) = fixture(10000);
    let lot = receive(&lot, &inv, &["2"], 10000);
    assert_eq!(lot.assets[0].cost_cents, 334);
    assert_eq!(lot.pending_units(), 2);
    assert_eq!(lot.pending_cost_cents(), 666);
    let lot = receive(&lot, &inv, &["1", "3"], 10001);
    assert_eq!(
        lot.assets.iter().map(|a| a.cost_cents).collect::<Vec<_>>(),
        [334, 333, 333]
    );
    assert_eq!(lot.pending_cost_cents(), 0);
    assert!(!lot.platform_verified);
}
#[test]
fn receipt_does_not_bind_same_names_wrong_accounts_or_stacks() {
    let (lot, inv) = fixture(10000);
    for case in 0..5 {
        let mut inv = inv.clone();
        match case {
            0 => inv.account_id = "other".into(),
            1 => inv.assets[0].instance_id = "01".into(),
            2 => inv.assets[0].amount = 2,
            3 => inv.assets[0].context_id = "16".into(),
            _ => inv.assets[0].market_hash_name = Some("different wear".into()),
        };
        let req = LotEventRequest {
            client_key: "r".into(),
            expected_revision: 1,
            action: LotAction::Receive {
                inventory_snapshot_id: inv.id.clone(),
                asset_ids: vec!["1".into()],
            },
        };
        assert!(apply(&lot, &req, Some(&inv), 10000).is_err());
    }
}
#[test]
fn lifecycle_requires_current_listing_exact_currency_and_monotonic_times() {
    let (lot, inv) = fixture(10000);
    let lot = receive(&lot, &inv, &["1"], 10000);
    let gross = Money {
        minor: 5000,
        currency: Currency::try_from("UAH".to_string()).unwrap(),
    };
    let list = LotAction::List {
        asset_id: "1".into(),
        listing_ref: "listing-1".into(),
        gross: gross.clone(),
        listed_at: 10001,
    };
    let lot = event(&lot, list.clone(), 10001);
    assert!(
        apply(
            &lot,
            &LotEventRequest {
                client_key: "duplicate".into(),
                expected_revision: lot.revision,
                action: list
            },
            None,
            10002
        )
        .is_err()
    );
    let mut sale = LotEventRequest {
        client_key: "sale".into(),
        expected_revision: lot.revision,
        action: LotAction::Sell {
            asset_id: "1".into(),
            listing_ref: "listing-1".into(),
            sale_ref: "sale-1".into(),
            net_wallet: Money::cny(4300),
            sold_at: 10002,
        },
    };
    assert!(apply(&lot, &sale, None, 10002).is_err());
    if let LotAction::Sell { net_wallet, .. } = &mut sale.action {
        net_wallet.currency = gross.currency;
    }
    let sold = apply(&lot, &sale, None, 10002).unwrap();
    assert_eq!(sold.assets[0].stage, AssetStage::Sold);
    let holdings = holdings::summarize(&lot.account_id.clone(), vec![sold], None, 10002).unwrap();
    assert_eq!(holdings.sold_cost_cents, 334);
    assert_eq!(holdings.open_cost_cents, 666);
    assert_eq!(holdings.reported_wallet_receipts[0].minor, 4300);
    assert!(!holdings.complete);
    let cancelled = event(
        &lot,
        LotAction::CancelListing {
            asset_id: "1".into(),
            listing_ref: "listing-1".into(),
            cancelled_at: 10002,
        },
        10002,
    );
    assert!(
        apply(
            &cancelled,
            &LotEventRequest {
                client_key: "backdated".into(),
                expected_revision: cancelled.revision,
                action: LotAction::List {
                    asset_id: "1".into(),
                    listing_ref: "listing-2".into(),
                    gross: Money::cny(500),
                    listed_at: 10001
                }
            },
            None,
            10003
        )
        .is_err()
    );
}
#[test]
fn inventory_and_ledger_are_merged_by_identity_without_counting_an_asset_twice() {
    let (lot, inv) = fixture(10000);
    let lot = receive(&lot, &inv, &["1"], 10000);
    let h = holdings::summarize(
        &lot.account_id.clone(),
        vec![lot.clone()],
        Some(inv.clone()),
        10000,
    )
    .unwrap();
    assert_eq!(h.items[0].held_units, 1);
    assert_eq!(h.items[0].pending_units, 2);
    assert_eq!(h.items[0].observed_untracked_units, 2);
    assert_eq!(h.open_cost_cents, 1000);
    assert!(!h.complete);
    assert!(
        holdings::summarize(
            &lot.account_id.clone(),
            vec![lot.clone(), lot],
            Some(inv),
            10000
        )
        .is_err()
    );
}
#[test]
fn missing_from_complete_inventory_reports_discrepancy_without_recording_a_sale() {
    let (lot, mut inv) = fixture(10000);
    let lot = receive(&lot, &inv, &["1"], 10000);
    inv.assets.clear();
    let h = holdings::summarize(
        &lot.account_id.clone(),
        vec![lot.clone()],
        Some(inv.clone()),
        10000,
    )
    .unwrap();
    assert_eq!(h.items[0].held_units, 1);
    assert_eq!(h.items[0].sold_units, 0);
    assert!(!h.items[0].discrepancies.is_empty());
    inv.coverage[0].exhausted = false;
    let h = holdings::summarize(&lot.account_id.clone(), vec![lot], Some(inv), 10000).unwrap();
    assert!(h.items[0].discrepancies.is_empty());
    assert!(h.warnings.iter().any(|w| w.contains("不完整")));
}
#[test]
fn listing_or_sold_assets_visible_in_inventory_are_conflicts_not_extra_positions() {
    let (lot, inv) = fixture(10000);
    let lot = receive(&lot, &inv, &["1"], 10000);
    let lot = event(
        &lot,
        LotAction::List {
            asset_id: "1".into(),
            listing_ref: "l".into(),
            gross: Money::cny(1000),
            listed_at: 10000,
        },
        10000,
    );
    let h = holdings::summarize(&lot.account_id.clone(), vec![lot], Some(inv), 10000).unwrap();
    assert_eq!(h.items[0].listed_units, 1);
    assert_eq!(h.items[0].observed_untracked_units, 2);
    assert_eq!(h.items[0].discrepancies.len(), 1);
}
