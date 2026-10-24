use super::*;
use crate::domain::{
    accounts::*,
    planning::{BuffValuationEvidence, SteamValuationEvidence},
    strategy::{evaluate, tests::fixture},
};
pub(crate) fn inspection_fixture(
    at: u64,
) -> (PreflightRequest, StrategyEvaluation, AccountContext) {
    let (s, mut p, h, r) = fixture(at);
    let account = Account {
        id: h.account_id.clone(),
        label: "fixture".into(),
        steam_id: "76561198000000001".into(),
        created_at: at,
    };
    p.request.buff_account_id = Some(account.id.clone());
    p.buff_valuations.push(BuffValuationEvidence {
        snapshot_id: "fixture-buff".into(),
        account_id: account.id.clone(),
        credential_generation: "buff-generation".into(),
        item_key: p.lines[0].candidate.item.key.clone(),
        goods_id: 1,
        observed_at: at,
        unit_cash_cents: Some(600),
        lowest_tier_visible_units: 2,
    });
    p.steam_valuations.push(SteamValuationEvidence {
        native_ask: None,
        snapshot_id: "fixture-steam".into(),
        market_hash_name: p.lines[0].candidate.item.key.clone(),
        observed_at: at,
        gross_listing_estimate_cents: 1000,
        best_bid_cents: Some(900),
    });
    let steam = SessionView {
        platform: Platform::Steam,
        generation: h.credential_generation.clone(),
        status: SessionStatus::Valid,
        checked_at: Some(at),
        retry_at: 0,
        message: "fixture".into(),
        subject: Some(account.steam_id.clone()),
    };
    let buff = SessionView {
        platform: Platform::Buff,
        generation: "buff-generation".into(),
        subject: Some("fixture-buff-user".into()),
        ..steam.clone()
    };
    let e = evaluate(s, p, vec![h], r, at).unwrap();
    let req = PreflightRequest {
        evaluation_id: e.id.clone(),
        account_id: Some(account.id.clone()),
        selections: vec![Selection {
            item_key: e.plan.lines[0].candidate.item.key.clone(),
            quantity: 1,
        }],
        max_quote_age_secs: 90,
    };
    (
        req,
        e,
        AccountContext {
            account,
            sessions: vec![steam, buff],
            buff_retry_at: 0,
            buff_blocked: false,
        },
    )
}
fn outcome(p: &Preflight, key: &str) -> ResearchStatus {
    p.checks.iter().find(|c| c.module == key).unwrap().status
}
#[test]
fn matching_saved_evidence_is_not_permission_and_quantity_is_bound() {
    let at = 1_792_800_000;
    let (req, e, c) = inspection_fixture(at);
    let p = inspect(req.clone(), e.clone(), Some(c.clone()), at).unwrap();
    assert_eq!(p.evidence_status, ResearchStatus::Pass);
    assert_eq!(p.status, ResearchStatus::Unknown);
    assert!(!p.execution_allowed);
    assert_eq!(p.selected_cash_cents, 600);
    assert_eq!(p.source_evaluation.plan.lines[0].quantity, 2);
    for key in [
        "balance",
        "position_ledger",
        "platform_fees",
        "order_executor",
        "live_quote_refresh",
    ] {
        assert_eq!(outcome(&p, key), ResearchStatus::Unknown);
    }
    for quantity in [0, 3] {
        let mut r = req.clone();
        r.selections[0].quantity = quantity;
        assert!(inspect(r, e.clone(), Some(c.clone()), at).is_err());
    }
    let mut r = req;
    r.selections.push(r.selections[0].clone());
    assert!(inspect(r, e, Some(c), at).is_err());
}
#[test]
fn stale_quotes_never_become_fresh_because_research_was_read_again() {
    let at = 1_792_800_000;
    let (req, e, c) = inspection_fixture(at);
    let original = serde_json::to_value(&e).unwrap();
    let p = inspect(req, e, Some(c), at + 91).unwrap();
    assert_eq!(outcome(&p, "selected_quote_age"), ResearchStatus::Fail);
    assert_eq!(p.status, ResearchStatus::Fail);
    assert_eq!(serde_json::to_value(p.source_evaluation).unwrap(), original);
    assert_eq!(p.rechecked_evaluation.created_at, at + 91);
    assert_ne!(p.rechecked_evaluation.id, original["id"].as_str().unwrap());
}
#[test]
fn rotation_identity_challenge_and_shortened_age_are_rechecked() {
    let at = 1_792_800_000;
    let (mut req, e, mut c) = inspection_fixture(at);
    c.sessions[0].generation = "rotated".into();
    c.sessions[0].subject = Some("other-steam-id".into());
    c.buff_blocked = true;
    c.sessions[1].generation = "rotated-buff".into();
    req.max_quote_age_secs = 300;
    let mut e = e;
    e.plan.request.max_quote_age_secs = 30;
    let p = inspect(req, e, Some(c), at + 31).unwrap();
    for key in [
        "history_generation",
        "buff_binding",
        "steam_session_metadata",
        "buff_policy",
        "selected_quote_age",
    ] {
        assert_eq!(outcome(&p, key), ResearchStatus::Fail);
    }
}
#[test]
fn missing_session_and_quotes_are_unknown_but_wrong_binding_is_rejected() {
    let at = 1_792_800_000;
    let (req, mut e, mut c) = inspection_fixture(at);
    c.sessions.clear();
    e.plan.buff_valuations.clear();
    e.plan.steam_valuations.clear();
    let p = inspect(req.clone(), e.clone(), Some(c.clone()), at).unwrap();
    for key in [
        "buff_binding",
        "steam_quote",
        "steam_session_metadata",
        "buff_session_metadata",
        "history_generation",
    ] {
        assert_eq!(outcome(&p, key), ResearchStatus::Unknown);
    }
    c.account.id = "other".into();
    assert!(inspect(req.clone(), e.clone(), Some(c), at).is_err());
    let mut r = req;
    r.account_id = None;
    assert!(inspect(r, e, None, at).is_err());
}
#[test]
fn price_tier_goods_and_future_timestamp_mismatches_fail() {
    let at = 1_792_800_000;
    let (req, e, c) = inspection_fixture(at);
    for case in 0..4 {
        let mut e = e.clone();
        let b = &mut e.plan.buff_valuations[0];
        match case {
            0 => b.lowest_tier_visible_units = 0,
            1 => b.unit_cash_cents = Some(601),
            2 => b.goods_id = 2,
            _ => b.observed_at = at + 1,
        };
        let p = inspect(req.clone(), e, Some(c.clone()), at).unwrap();
        assert_eq!(outcome(&p, "buff_binding"), ResearchStatus::Fail);
    }
}
#[test]
fn unselected_rule_failure_does_not_taint_selected_research() {
    let at = 1_792_800_000;
    let (req, mut e, c) = inspection_fixture(at);
    let mut line = e.plan.lines[0].clone();
    line.candidate.item.key = "unselected".into();
    line.candidate.item.cash_cents = 999999;
    e.plan.lines.push(line);
    let p = inspect(req, e, Some(c), at).unwrap();
    assert_eq!(p.rechecked_evaluation.status, ResearchStatus::Fail);
    assert_eq!(p.evidence_status, ResearchStatus::Pass);
}

#[test]
fn demo_without_an_account_can_report_missing_conditions_but_is_blocked() {
    let at = 1_792_800_000;
    let (mut req, mut e, _) = inspection_fixture(at);
    e.histories.clear();
    e.request.history_snapshot_ids.clear();
    e.request.account_id = None;
    e.plan.source = SourceKind::Demo;
    e.plan.request.buff_account_id = None;
    e.plan.buff_valuations.clear();
    e.plan.steam_valuations.clear();
    req.account_id = None;
    let p = inspect(req, e, None, at).unwrap();
    assert_eq!(p.status, ResearchStatus::Fail);
    assert_eq!(outcome(&p, "source"), ResearchStatus::Fail);
    assert_eq!(
        outcome(&p, "steam_session_metadata"),
        ResearchStatus::Unknown
    );
    assert!(!p.execution_allowed);
}
