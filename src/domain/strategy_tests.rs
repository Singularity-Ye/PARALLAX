use super::*;
use crate::domain::{analysis, history::*, money::Currency, planning::*, selection::*};

pub(crate) fn fixture(
    at: u64,
) -> (
    StrategyVersion,
    PlanPreview,
    HistorySnapshot,
    StrategyEvaluationRequest,
) {
    let scan_request = ScanRequest {
        source: SourceKind::Steamdt,
        max_ratio_bps: 20000,
        ..Default::default()
    };
    let mut scan = ScanReport::new(scan_request.clone());
    scan.status = "completed".into();
    scan.finished_at = Some(at);
    scan.candidates.push(analysis::evaluate(
        MarketItem {
            key: "fixture-item".into(),
            name: "fixture".into(),
            goods_id: Some(1),
            cash_cents: 600,
            steam_gross_cents: 1000,
            volume: 1000,
            observed_at: at,
        },
        &scan_request,
    ));
    let request = PlanRequest {
        scan_id: scan.id.clone(),
        budget_cents: 10000,
        target_wallet_cents: None,
        max_units_per_item: 2,
        max_positions: 1,
        max_quote_age_secs: 300,
        position_snapshot: None,
        liquidity_policy: None,
        max_position_age_secs: 300,
        steam_snapshot_ids: vec![],
        buff_snapshot_ids: vec![],
        buff_account_id: None,
    };
    let plan = preview(&scan, request, at).unwrap();
    assert_eq!(plan.lines.len(), 1);
    let today = at / 86400;
    let hrequest = HistoryRequest {
        inventory_snapshot_id: None,
        manual_item: false,
        scan_id: plan.request.scan_id.clone(),
        item_key: "fixture-item".into(),
        window_days: 30,
        slippage_bps: 300,
    };
    let series = HistorySeries {
        currency: Some(Currency::cny()),
        currency_basis: "explicit_code_or_unique_marker".into(),
        points: (today - 30..today)
            .enumerate()
            .map(|(i, d)| HistorySample {
                at: d * 86400,
                price_major_1e4: 100000 + i as u64 * 100,
                volume: Some(10),
            })
            .collect(),
        response_rows: 30,
        excluded_before_window: 0,
        excluded_open_day: 0,
        from_utc_day: today - 30,
        until_utc_day_exclusive: today,
    };
    let history = HistorySnapshot {
        id: "fixture-history".into(),
        account_id: "fixture-account".into(),
        credential_generation: "fixture-generation".into(),
        request: hrequest.clone(),
        started_at: at,
        observed_at: at,
        source: "steam_authenticated_pricehistory".into(),
        identity_binding: "scan_candidate_and_request_parameters".into(),
        platform_verified: false,
        analysis: analyze(&series, &hrequest, at).unwrap(),
        series,
    };
    let strategy = StrategyVersion {
        strategy_id: "fixture-strategy".into(),
        revision: 1,
        created_at: at,
        draft: StrategyDraft {
            objective: Default::default(),
            schema_version: 1,
            name: "购入研究".into(),
            description: "合成测试，不访问平台".into(),
            rules: vec![
                StrategyRule::StrictRatio {
                    max_ratio_bps: 9000,
                },
                StrategyRule::HistoryCoverage {
                    min_usable_days: 20,
                    max_missing_days: 2,
                    max_age_secs: 3600,
                    max_reference_lag_days: 1,
                },
                StrategyRule::SampleCv {
                    basis: CvBasis::Raw,
                    max_cv_bps: 1000,
                },
                StrategyRule::Trend7 {
                    min_points: 5,
                    min_slope_bps_per_day: -100,
                    max_slope_bps_per_day: 100,
                    min_r_squared_bps: 5000,
                },
                StrategyRule::CalendarP10 {
                    min_pairs: 10,
                    min_return_bps: -1000,
                    slippage_bps: 300,
                },
            ],
        },
    };
    let evaluation = StrategyEvaluationRequest {
        strategy_id: strategy.strategy_id.clone(),
        revision: 1,
        plan_id: plan.id.clone(),
        account_id: Some(history.account_id.clone()),
        history_snapshot_ids: vec![history.id.clone()],
        order_book_snapshot_ids: vec![],
    };
    (strategy, plan, history, evaluation)
}
fn outcomes(e: &StrategyEvaluation) -> Vec<ResearchStatus> {
    e.items[0].decisions.iter().map(|d| d.status).collect()
}
#[test]
fn full_research_pass_embeds_evidence_but_never_authorizes_execution() {
    let at = 10000 * 86400 + 120;
    let (s, p, h, r) = fixture(at);
    let e = evaluate(s, p, vec![h], r, at).unwrap();
    assert_eq!(e.status, ResearchStatus::Pass);
    assert_eq!(e.histories.len(), 1);
    assert_eq!(e.strategy.revision, 1);
    assert!(e.preview_only);
    assert!(!e.execution_allowed);
    assert!(!e.execution_blockers.is_empty());
}
#[test]
fn strict_ratio_rejects_equality_and_compares_unrounded_amounts() {
    let at = 10000 * 86400 + 120;
    let (mut s, mut p, _, mut r) = fixture(at);
    s.draft.rules = vec![StrategyRule::StrictRatio {
        max_ratio_bps: 9000,
    }];
    r.history_snapshot_ids.clear();
    p.lines[0].candidate.item.cash_cents = 9000;
    p.lines[0].candidate.estimated_wallet_cents = 10000;
    assert_eq!(
        evaluate(s.clone(), p.clone(), vec![], r.clone(), at)
            .unwrap()
            .status,
        ResearchStatus::Fail
    );
    p.lines[0].candidate.item.cash_cents = 899999;
    p.lines[0].candidate.estimated_wallet_cents = 1000000;
    // Ceil displayed ratio is 9000, but exact ratio is strictly below 90%.
    assert_eq!(
        evaluate(s, p, vec![], r, at).unwrap().status,
        ResearchStatus::Pass
    );
}
#[test]
fn stale_quotes_and_missing_or_legacy_history_remain_unknown() {
    let at = 10000 * 86400 + 120;
    let (s, p, mut h, mut r) = fixture(at);
    h.analysis.stability = None;
    let e = evaluate(s.clone(), p.clone(), vec![h], r.clone(), at).unwrap();
    assert_eq!(e.status, ResearchStatus::Unknown);
    assert_eq!(outcomes(&e)[2], ResearchStatus::Unknown);
    r.history_snapshot_ids.clear();
    let e = evaluate(s, p, vec![], r, at + 301).unwrap();
    assert_eq!(outcomes(&e)[0], ResearchStatus::Unknown);
    assert_eq!(outcomes(&e)[3], ResearchStatus::Unknown);
}
#[test]
fn insufficient_coverage_blocks_metrics_and_future_or_previous_day_is_unknown() {
    let at = 10000 * 86400 + 120;
    let (s, p, mut h, r) = fixture(at);
    h.analysis.stability.as_mut().unwrap().usable_days = 2;
    let e = evaluate(s.clone(), p.clone(), vec![h.clone()], r.clone(), at).unwrap();
    assert_eq!(e.status, ResearchStatus::Fail);
    assert_eq!(outcomes(&e)[3], ResearchStatus::Unknown);
    h.observed_at = at + 1;
    assert_eq!(
        outcomes(&evaluate(s.clone(), p.clone(), vec![h.clone()], r.clone(), at).unwrap())[2],
        ResearchStatus::Unknown
    );
    h.observed_at = at;
    h.series.until_utc_day_exclusive -= 1;
    assert_eq!(
        outcomes(&evaluate(s, p, vec![h], r, at).unwrap())[2],
        ResearchStatus::Unknown
    );
}
#[test]
fn history_binding_rejects_other_account_scan_item_and_multiple_snapshots() {
    let at = 10000 * 86400 + 120;
    let (s, p, h, r) = fixture(at);
    for field in ["account", "scan", "inventory", "item", "id"] {
        let mut bad = h.clone();
        match field {
            "account" => bad.account_id = "other".into(),
            "scan" => bad.request.scan_id = "other".into(),
            "inventory" => {
                bad.request.scan_id.clear();
                bad.request.inventory_snapshot_id = Some("inventory".into());
                bad.identity_binding = "inventory_snapshot_and_request_parameters".into();
            }
            "item" => bad.request.item_key = "other".into(),
            _ => bad.id = "other".into(),
        }
        assert!(evaluate(s.clone(), p.clone(), vec![bad], r.clone(), at).is_err());
    }
    let mut r = r;
    let mut h2 = h.clone();
    h2.id = "second".into();
    r.history_snapshot_ids.push(h2.id.clone());
    assert!(evaluate(s, p, vec![h, h2], r, at).is_err());
}
#[test]
fn p10_sample_slippage_and_constant_trend_require_real_defined_metrics() {
    let at = 10000 * 86400 + 120;
    let (s, p, mut h, r) = fixture(at);
    h.analysis.risk.as_mut().unwrap().slippage_bps = 0;
    h.analysis.stability.as_mut().unwrap().trend7.r_squared_bps = None;
    let e = evaluate(s.clone(), p.clone(), vec![h.clone()], r.clone(), at).unwrap();
    assert_eq!(outcomes(&e)[4], ResearchStatus::Unknown);
    assert_eq!(outcomes(&e)[5], ResearchStatus::Unknown);
    h.analysis.risk.as_mut().unwrap().slippage_bps = 300;
    h.analysis.risk.as_mut().unwrap().sample_count = 1;
    assert_eq!(
        outcomes(&evaluate(s, p, vec![h], r, at).unwrap())[5],
        ResearchStatus::Unknown
    );
}
#[test]
fn strategy_rejects_dependencies_duplicate_order_ranges_and_unknown_fields() {
    let (s, _, _, _) = fixture(10000 * 86400);
    for rules in [
        vec![StrategyRule::PositionCapacity],
        vec![StrategyRule::StrictRatio { max_ratio_bps: 0 }],
        vec![
            StrategyRule::StrictRatio {
                max_ratio_bps: 9000,
            },
            StrategyRule::SampleCv {
                basis: CvBasis::Raw,
                max_cv_bps: 1000,
            },
        ],
        vec![
            StrategyRule::StrictRatio {
                max_ratio_bps: 9000,
            },
            StrategyRule::StrictRatio {
                max_ratio_bps: 9000,
            },
        ],
    ] {
        let mut d = s.draft.clone();
        d.rules = rules;
        assert!(d.validate().is_err());
    }
    let mut v = serde_json::to_value(s.draft).unwrap();
    v["cookies"] = json!("forbidden");
    assert!(serde_json::from_value::<StrategyDraft>(v).is_err());
    assert!(
        serde_json::from_value::<StrategyRule>(json!({"module":"execute_python","code":"test"}))
            .is_err()
    );
}
#[test]
fn position_rule_reuses_capacity_and_never_treats_absence_as_zero() {
    use crate::domain::positions::*;
    let at = 10000 * 86400 + 120;
    let (mut s, mut p, _, mut r) = fixture(at);
    s.draft.rules = vec![
        StrategyRule::StrictRatio {
            max_ratio_bps: 9000,
        },
        StrategyRule::PositionCapacity,
    ];
    r.history_snapshot_ids.clear();
    assert_eq!(
        evaluate(s.clone(), p.clone(), vec![], r.clone(), at)
            .unwrap()
            .status,
        ResearchStatus::Unknown
    );
    p.request.position_snapshot = Some(PositionSnapshot {
        account_id: r.account_id.clone().unwrap(),
        observed_at: at,
        reserved_cash_cents: 0,
        entries: vec![PositionEntry {
            key: "fixture-item".into(),
            held_units: 1,
            pending_buy_units: 0,
            available_units: None,
            daily_volume: None,
        }],
    });
    assert_eq!(
        evaluate(s.clone(), p.clone(), vec![], r.clone(), at)
            .unwrap()
            .status,
        ResearchStatus::Fail
    );
    p.request
        .position_snapshot
        .as_mut()
        .unwrap()
        .entries
        .clear();
    assert_eq!(
        evaluate(s.clone(), p.clone(), vec![], r.clone(), at)
            .unwrap()
            .status,
        ResearchStatus::Unknown
    );
    r.account_id = None;
    assert!(evaluate(s, p, vec![], r, at).is_err());
}
#[test]
fn cv_basis_is_explicit_and_empty_plan_cannot_pass() {
    let at = 10000 * 86400 + 120;
    let (mut s, mut p, mut h, r) = fixture(at);
    s.draft.rules.truncate(3);
    h.analysis.stability.as_mut().unwrap().raw_sample_cv_bps = Some(5000);
    h.analysis.stability.as_mut().unwrap().clean_sample_cv_bps = Some(100);
    assert_eq!(
        evaluate(s.clone(), p.clone(), vec![h.clone()], r.clone(), at)
            .unwrap()
            .status,
        ResearchStatus::Fail
    );
    s.draft.rules[2] = StrategyRule::SampleCv {
        basis: CvBasis::IqrCleaned,
        max_cv_bps: 1000,
    };
    assert_eq!(
        evaluate(s.clone(), p.clone(), vec![h], r.clone(), at)
            .unwrap()
            .status,
        ResearchStatus::Pass
    );
    p.lines.clear();
    let mut r = r;
    r.history_snapshot_ids.clear();
    assert_eq!(
        evaluate(s, p, vec![], r, at).unwrap().status,
        ResearchStatus::Unknown
    );
}

#[test]
fn expired_fx_cannot_be_silently_reused_in_a_passing_preview() {
    let at = 10000 * 86400 + 120;
    let (s, mut p, h, r) = fixture(at);
    let mut fx = crate::domain::fx::tests::fixture(at);
    fx.fetched_at = at - 6 * 3600;
    p.fx = Some(fx);
    let e = evaluate(s, p, vec![h], r, at + 1).unwrap();
    assert_eq!(e.status, ResearchStatus::Unknown);
    assert_eq!(
        e.items[0]
            .decisions
            .iter()
            .find(|d| d.module == "fx_freshness")
            .unwrap()
            .status,
        ResearchStatus::Unknown
    );
}

#[test]
fn demo_evaluation_remains_demo_and_rejects_platform_history() {
    let at = 10000 * 86400 + 120;
    let (s, mut p, h, mut r) = fixture(at);
    p.source = SourceKind::Demo;
    assert!(evaluate(s.clone(), p.clone(), vec![h], r.clone(), at).is_err());
    r.history_snapshot_ids.clear();
    let e = evaluate(s, p, vec![], r, at).unwrap();
    assert_eq!(e.plan.source, SourceKind::Demo);
    assert!(!e.execution_allowed);
}

#[test]
fn holding_objective_rejects_wallet_rules_and_requires_explicit_v3_history() {
    let (mut s, _, _, _) = fixture(10000 * 86400 + 120);
    s.draft.objective = ResearchObjective::Holding;
    assert!(s.draft.validate().is_err());
    s.draft.schema_version = 3;
    assert!(s.draft.validate().is_err());
    s.draft.rules.remove(0);
    assert!(s.draft.validate().is_ok());
    let mut missing = s.draft.clone();
    missing.rules.remove(0);
    assert!(missing.validate().is_err());
    s.draft.objective = ResearchObjective::WalletConversion;
    assert!(s.draft.validate().is_err());
}
#[test]
fn holding_rule_match_does_not_use_wallet_value_or_establish_fair_cash_value() {
    let at = 10000 * 86400 + 120;
    let (mut s, mut p, h, mut r) = fixture(at);
    s.draft.schema_version = 3;
    s.draft.objective = ResearchObjective::Holding;
    s.draft.rules.remove(0);
    p.lines[0].candidate.estimated_wallet_cents = 0;
    let e = evaluate(s.clone(), p.clone(), vec![h], r.clone(), at).unwrap();
    assert_eq!(e.algorithm, "purchase_research_v3");
    assert_eq!(e.status, ResearchStatus::Pass);
    assert!(!e.execution_allowed);
    assert!(
        !e.items[0]
            .decisions
            .iter()
            .any(|d| d.module == "strict_ratio")
    );
    let review = e.items[0].review.as_ref().unwrap();
    assert_eq!(review.objective, ResearchObjective::Holding);
    assert!(!review.holding_value_established);
    assert_eq!(review.cash_price_basis, "steamdt_reference");
    assert!(!review.unassessed.is_empty());
    let inspection = crate::domain::preflight::PreflightRequest {
        evaluation_id: e.id.clone(),
        account_id: None,
        selections: vec![crate::domain::preflight::Selection {
            item_key: e.items[0].item_key.clone(),
            quantity: 1,
        }],
        max_quote_age_secs: 300,
    };
    assert!(
        crate::domain::preflight::inspect(inspection, e, None, at)
            .unwrap_err()
            .contains("持有观察")
    );
    r.history_snapshot_ids.clear();
    assert_eq!(
        evaluate(s, p, vec![], r, at).unwrap().status,
        ResearchStatus::Unknown
    );
}
#[test]
fn legacy_drafts_and_saved_results_are_not_silently_reinterpreted() {
    let at = 10000 * 86400 + 120;
    let (s, p, h, r) = fixture(at);
    let old = serde_json::to_value(&s.draft).unwrap();
    assert!(old.get("objective").is_none());
    let draft: StrategyDraft = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(draft.objective, ResearchObjective::WalletConversion);
    assert_eq!(serde_json::to_value(draft).unwrap(), old);
    let mut old_result = serde_json::to_value(evaluate(s, p, vec![h], r, at).unwrap()).unwrap();
    old_result["items"][0]
        .as_object_mut()
        .unwrap()
        .remove("review");
    let decoded: StrategyEvaluation = serde_json::from_value(old_result.clone()).unwrap();
    assert!(decoded.items[0].review.is_none());
    assert_eq!(serde_json::to_value(decoded).unwrap(), old_result);
}
#[test]
fn reference_price_review_keeps_unchecked_risks_visible_even_for_passing_rules() {
    let at = 10000 * 86400 + 120;
    let (mut s, p, _, mut r) = fixture(at);
    s.draft.rules.truncate(1);
    r.history_snapshot_ids.clear();
    let e = evaluate(s, p, vec![], r, at).unwrap();
    assert_eq!(e.status, ResearchStatus::Pass);
    let review = e.items[0].review.as_ref().unwrap();
    assert!(review.unchecked_rules.contains(&"history_coverage".into()));
    assert!(review.unchecked_rules.contains(&"position_capacity".into()));
    assert_eq!(
        review.conclusion_scope,
        "configured_rules_on_saved_plan_only"
    );
    assert_eq!(review.cash_snapshot_id, None);
    assert!(!review.holding_value_established);
}
#[test]
fn price_review_uses_snapshot_lineage_without_calling_it_an_executable_quote() {
    use crate::domain::planning::BuffValuationEvidence;
    let (s, mut p, _, _) = fixture(10000 * 86400 + 120);
    p.buff_valuations.push(BuffValuationEvidence {
        snapshot_id: "quote".into(),
        account_id: "account".into(),
        credential_generation: "generation".into(),
        item_key: p.lines[0].candidate.item.key.clone(),
        goods_id: 1,
        observed_at: 50,
        unit_cash_cents: Some(100),
        lowest_tier_visible_units: 2,
    });
    let review = crate::domain::strategy_review::review(&s.draft, &p, &p.lines[0]);
    assert_eq!(review.cash_price_basis, "buff_visible_sell_snapshot");
    assert_eq!(review.price_evidence_at, 50);
    assert_eq!(review.cash_snapshot_id.as_deref(), Some("quote"));
    p.source = crate::domain::selection::SourceKind::Demo;
    assert_eq!(
        crate::domain::strategy_review::review(&s.draft, &p, &p.lines[0]).cash_price_basis,
        "demo"
    );
}
