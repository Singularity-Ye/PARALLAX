use super::*;
use crate::domain::{
    orderbook::{OrderBookRequest, OrderBookSnapshot, PriceLevel},
    strategy::tests::fixture,
    strategy::*,
};
pub(crate) fn policy() -> ExitValuationPolicy {
    ExitValuationPolicy {
        pricing: WallGapPolicy::default(),
        fees: FeeSchedule {
            currency: Currency::cny(),
            platform_bps: 0,
            publisher_bps: 0,
            platform_min_minor: 0,
            publisher_min_minor: 0,
            increment_minor: 1,
        },
        min_pairs: 10,
        slippage_bps: 300,
        max_book_age_secs: 300,
    }
}
fn book() -> OrderBook {
    OrderBook {
        currency: Currency::cny(),
        asks: vec![PriceLevel {
            price_minor: 10000,
            quantity: 30,
        }],
        bids: vec![],
        reported_buy_orders: None,
        reported_sell_orders: None,
    }
}
fn risk() -> SevenDayRisk {
    SevenDayRisk {
        algorithm: "calendar_p10_v1".into(),
        sample_count: 20,
        return_p10_bps: Some(-2000),
        conservative_gross: Some(Money::cny(1)),
        slippage_bps: 300,
    }
}
#[test]
fn stress_rebases_on_current_wall_price_applies_slippage_once_and_deducts_native_fees() {
    let mut p = policy();
    p.fees.platform_bps = 500;
    p.fees.publisher_bps = 1000;
    p.fees.platform_min_minor = 7;
    p.fees.publisher_min_minor = 7;
    let v = estimate(&book(), &Currency::cny(), &risk(), &p, None, 100).unwrap();
    assert_eq!(v.pricing.native_gross_listing_estimate.unwrap().minor, 9999);
    assert_eq!(v.stressed_gross_native.minor, 7759);
    assert_eq!(v.stressed_net_cny.minor, 6748);
    assert_eq!(v.stressed_fees.unwrap().fees_minor, 1011);
    assert!(v.normal_net_cny.minor > v.stressed_net_cny.minor);
}
#[test]
fn positive_returns_never_inflate_price_and_total_slippage_is_a_known_zero_net() {
    let mut r = risk();
    r.return_p10_bps = Some(9000);
    let mut p = policy();
    p.slippage_bps = 0;
    r.slippage_bps = 0;
    assert_eq!(
        estimate(&book(), &Currency::cny(), &r, &p, None, 100)
            .unwrap()
            .stressed_gross_native
            .minor,
        9999
    );
    p.slippage_bps = 10000;
    r.slippage_bps = 10000;
    let v = estimate(&book(), &Currency::cny(), &r, &p, None, 100).unwrap();
    assert_eq!(v.stressed_net_cny.minor, 0);
    assert!(v.stressed_fees.is_none());
    r.return_p10_bps = Some(-10001);
    assert!(estimate(&book(), &Currency::cny(), &r, &p, None, 100).is_err());
}
#[test]
fn uah_fees_are_deducted_before_integer_conversion_and_stale_or_missing_fx_is_unknown() {
    let at = 1_792_800_000;
    let uah: Currency = "UAH".to_string().try_into().unwrap();
    let mut b = book();
    b.currency = uah.clone();
    b.asks[0].price_minor = 60000;
    let mut p = policy();
    p.fees.currency = uah.clone();
    let mut fx = crate::domain::fx::tests::fixture(at);
    fx.rates.get_mut("UAH").unwrap().units_per_cny_1e8 = 6 * fx::SCALE;
    let v = estimate(&b, &uah, &risk(), &p, Some(&fx), at).unwrap();
    assert_eq!(
        v.pricing.native_gross_listing_estimate.unwrap().minor,
        59994
    );
    assert_eq!(v.stressed_gross_native.currency, uah);
    assert_eq!(v.stressed_gross_native.minor, 46555);
    assert_eq!(v.stressed_net_cny, Money::cny(7759));
    assert!(estimate(&b, &uah, &risk(), &p, None, at).is_err());
    assert!(estimate(&b, &uah, &risk(), &p, Some(&fx), at + 21601).is_err());
    p.fees.currency = Currency::cny();
    assert!(estimate(&b, &uah, &risk(), &p, Some(&fx), at).is_err());
}
#[test]
fn insufficient_or_mismatched_history_and_empty_books_are_not_valuations() {
    let mut r = risk();
    r.sample_count = 1;
    assert!(estimate(&book(), &Currency::cny(), &r, &policy(), None, 100).is_err());
    r = risk();
    r.slippage_bps = 0;
    assert!(estimate(&book(), &Currency::cny(), &r, &policy(), None, 100).is_err());
    r = risk();
    r.return_p10_bps = None;
    assert!(estimate(&book(), &Currency::cny(), &r, &policy(), None, 100).is_err());
    let mut b = book();
    b.asks.clear();
    assert!(estimate(&b, &Currency::cny(), &risk(), &policy(), None, 100).is_err());
    assert!(
        estimate(
            &book(),
            &"UAH".to_string().try_into().unwrap(),
            &risk(),
            &policy(),
            None,
            100
        )
        .is_err()
    );
}
pub(crate) fn strategy_fixture(
    at: u64,
) -> (
    StrategyVersion,
    crate::domain::planning::PlanPreview,
    crate::domain::history::HistorySnapshot,
    OrderBookSnapshot,
    StrategyEvaluationRequest,
) {
    let (mut s, mut plan, mut h, mut request) = fixture(at);
    s.draft.schema_version = 2;
    s.draft.rules.truncate(2);
    s.draft.rules.push(StrategyRule::StressExitRatio {
        max_ratio_bps: 10000,
        parameters: policy(),
    });
    plan.lines[0].candidate.item.cash_cents = 600;
    plan.lines[0].candidate.estimated_wallet_cents = 10000;
    h.analysis.risk = Some(risk());
    let b = OrderBookSnapshot::new(
        OrderBookRequest {
            app_id: 730,
            market_hash_name: plan.lines[0].candidate.item.key.clone(),
            currency: Currency::cny(),
        },
        book(),
        at,
    )
    .unwrap();
    request.order_book_snapshot_ids = vec![b.id.clone()];
    (s, plan, h, b, request)
}
#[test]
fn stress_ratio_boundary_is_strict_and_evidence_is_embedded_without_execution() {
    let at = 10000 * 86400 + 120;
    let (s, mut p, h, b, r) = strategy_fixture(at);
    p.lines[0].candidate.item.cash_cents = 7759;
    assert_eq!(
        evaluate_with_order_books(
            s.clone(),
            p.clone(),
            vec![h.clone()],
            vec![b.clone()],
            r.clone(),
            at
        )
        .unwrap()
        .status,
        ResearchStatus::Fail
    );
    p.lines[0].candidate.item.cash_cents = 7758;
    let e = evaluate_with_order_books(s, p, vec![h], vec![b], r, at).unwrap();
    assert_eq!(e.status, ResearchStatus::Pass);
    assert_eq!(e.algorithm, "purchase_research_v2");
    assert_eq!(
        e.items[0]
            .exit_valuation
            .as_ref()
            .unwrap()
            .stressed_net_cny
            .minor,
        7759
    );
    assert_eq!(e.order_books.len(), 1);
    assert!(!e.execution_allowed);
}
#[test]
fn missing_stale_books_and_failed_coverage_remain_unknown_without_erasing_other_checks() {
    let at = 10000 * 86400 + 120;
    let (s, p, mut h, mut b, mut r) = strategy_fixture(at);
    b.observed_at = at - 301;
    let e = evaluate_with_order_books(
        s.clone(),
        p.clone(),
        vec![h.clone()],
        vec![b],
        r.clone(),
        at,
    )
    .unwrap();
    assert_eq!(e.status, ResearchStatus::Unknown);
    assert!(e.items[0].exit_valuation.is_none());
    r.order_book_snapshot_ids.clear();
    h.analysis.stability.as_mut().unwrap().usable_days = 1;
    let e = evaluate_with_order_books(s, p, vec![h], vec![], r, at).unwrap();
    assert_eq!(e.status, ResearchStatus::Fail);
    assert_eq!(
        e.items[0].decisions.last().unwrap().status,
        ResearchStatus::Unknown
    );
}
#[test]
fn schema_one_rejects_new_rule_and_wrong_duplicate_or_unused_book_bindings_fail() {
    let at = 10000 * 86400 + 120;
    let (mut s, p, h, b, r) = strategy_fixture(at);
    s.draft.schema_version = 1;
    assert!(s.draft.validate().is_err());
    s.draft.schema_version = 2;
    let mut wrong = b.clone();
    wrong.request.market_hash_name = "wrong".into();
    assert!(
        evaluate_with_order_books(
            s.clone(),
            p.clone(),
            vec![h.clone()],
            vec![wrong],
            r.clone(),
            at
        )
        .is_err()
    );
    let mut duplicate = b.clone();
    duplicate.id = "duplicate".into();
    let mut rr = r.clone();
    rr.order_book_snapshot_ids.push(duplicate.id.clone());
    assert!(
        evaluate_with_order_books(
            s.clone(),
            p.clone(),
            vec![h.clone()],
            vec![b.clone(), duplicate],
            rr,
            at
        )
        .is_err()
    );
    s.draft.rules.pop();
    s.draft.schema_version = 1;
    assert!(evaluate_with_order_books(s, p, vec![h], vec![b], r, at).is_err());
}
