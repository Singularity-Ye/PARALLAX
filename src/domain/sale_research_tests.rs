use super::*;
use crate::domain::{
    accounts::{Account, SessionView},
    exit_valuation::tests::strategy_fixture,
    ledger::tests::{fixture, receive},
    strategy::evaluate_with_order_books,
};
pub(crate) fn research_fixture(
    at: u64,
) -> (
    SaleResearchRequest,
    PurchaseLot,
    StrategyEvaluation,
    InventorySnapshot,
    AccountContext,
) {
    let (s, p, h, b, r) = strategy_fixture(at);
    let e = evaluate_with_order_books(s, p, vec![h], vec![b], r, at).unwrap();
    let (lot, inv) = fixture(at);
    let lot = receive(&lot, &inv, &["1"], at);
    let context = AccountContext {
        account: Account {
            id: lot.account_id.clone(),
            label: "fixture".into(),
            steam_id: inv.steam_id.clone(),
            created_at: at,
        },
        sessions: vec![SessionView {
            platform: Platform::Steam,
            generation: inv.credential_generation.clone(),
            status: SessionStatus::Valid,
            checked_at: Some(at),
            retry_at: 0,
            message: "fixture".into(),
            subject: Some(inv.steam_id.clone()),
        }],
        buff_retry_at: 0,
        buff_blocked: false,
    };
    let req = SaleResearchRequest {
        lot_id: lot.id.clone(),
        asset_id: "1".into(),
        evaluation_id: e.id.clone(),
        inventory_snapshot_id: inv.id.clone(),
        max_inventory_age_secs: 90,
        policy: SellPolicy::Immediate,
    };
    (req, lot, e, inv, context)
}
#[test]
fn immediate_preview_uses_allocated_cost_and_existing_exit_math_without_execution() {
    let at = 1_792_800_000;
    let (r, l, e, i, c) = research_fixture(at);
    let v = evaluate(r, l, e, i, c, at).unwrap();
    assert!(matches!(v.action, SaleAction::ReviewListing));
    assert_eq!(v.unit_cost_cents, 334);
    assert!(!v.execution_allowed);
    assert!(v.estimated_wallet_surplus_cny_cents.unwrap() > 0);
}
#[test]
fn profit_guard_has_exact_inclusive_threshold_and_can_use_stressed_net() {
    let at = 1_792_800_000;
    let (mut r, mut l, e, i, c) = research_fixture(at);
    let net = e.items[0]
        .exit_valuation
        .as_ref()
        .unwrap()
        .stressed_net_cny
        .minor;
    l.assets[0].cost_cents = net;
    r.policy = SellPolicy::ProfitProtect {
        min_wallet_return_bps: 0,
        basis: ValueBasis::Stressed,
        trend: None,
    };
    assert!(matches!(
        evaluate(r.clone(), l.clone(), e.clone(), i.clone(), c.clone(), at)
            .unwrap()
            .action,
        SaleAction::ReviewListing
    ));
    l.assets[0].cost_cents += 1;
    assert!(matches!(
        evaluate(r, l, e, i, c, at).unwrap().action,
        SaleAction::Hold
    ));
}
#[test]
fn rising_trend_waits_and_unknown_fit_does_not_become_flat() {
    let at = 1_792_800_000;
    let (mut r, l, mut e, i, c) = research_fixture(at);
    r.policy = SellPolicy::TrendWait {
        gate: TrendGate {
            min_points: 5,
            min_r_squared_bps: 5000,
            rising_above_bps_per_day: 0,
        },
    };
    assert!(matches!(
        evaluate(r.clone(), l.clone(), e.clone(), i.clone(), c.clone(), at)
            .unwrap()
            .action,
        SaleAction::Hold
    ));
    e.histories[0]
        .analysis
        .stability
        .as_mut()
        .unwrap()
        .trend7
        .r_squared_bps = None;
    assert!(matches!(
        evaluate(r, l, e, i, c, at).unwrap().action,
        SaleAction::Unknown
    ));
}
#[test]
fn stale_inventory_wrong_identity_listed_or_rotated_credentials_block() {
    let at = 1_792_800_000;
    let (r, l, e, i, c) = research_fixture(at);
    for case in 0..4 {
        let (mut l, mut i, mut c) = (l.clone(), i.clone(), c.clone());
        match case {
            0 => i.observed_at = at - 91,
            1 => i.assets[0].instance_id = "2".into(),
            2 => l.assets[0].stage = AssetStage::Listed,
            _ => c.sessions[0].generation = "new".into(),
        };
        assert!(matches!(
            evaluate(r.clone(), l, e.clone(), i, c, at).unwrap().action,
            SaleAction::Blocked
        ));
    }
}
#[test]
fn partial_inventory_absence_missing_exit_and_pause_are_distinct() {
    let at = 1_792_800_000;
    let (mut r, l, mut e, mut i, c) = research_fixture(at);
    i.assets.clear();
    i.coverage[0].exhausted = false;
    e.order_books.clear();
    e.request.order_book_snapshot_ids.clear();
    assert!(matches!(
        evaluate(r.clone(), l.clone(), e.clone(), i.clone(), c.clone(), at)
            .unwrap()
            .action,
        SaleAction::Unknown
    ));
    r.policy = SellPolicy::Pause;
    assert!(matches!(
        evaluate(r, l, e, i, c, at).unwrap().action,
        SaleAction::Hold
    ));
}
#[test]
fn sell_policy_does_not_reuse_purchase_ratio_as_a_sell_gate() {
    let at = 1_792_800_000;
    let (r, l, mut e, i, c) = research_fixture(at);
    if let crate::domain::strategy::StrategyRule::StrictRatio { max_ratio_bps } =
        &mut e.strategy.draft.rules[0]
    {
        *max_ratio_bps = 1;
    }
    let v = evaluate(r, l, e, i, c, at).unwrap();
    assert_eq!(v.rechecked_evaluation.status, ResearchStatus::Fail);
    assert!(matches!(v.action, SaleAction::ReviewListing));
}
#[test]
fn wrong_account_and_asset_references_are_rejected() {
    let at = 1_792_800_000;
    let (mut r, l, e, i, mut c) = research_fixture(at);
    c.account.id = "other".into();
    assert!(evaluate(r.clone(), l.clone(), e.clone(), i.clone(), c, at).is_err());
    r.asset_id = "not-in-lot".into();
    let (_, _, _, _, c) = research_fixture(at);
    assert!(evaluate(r, l, e, i, c, at).is_err());
}
