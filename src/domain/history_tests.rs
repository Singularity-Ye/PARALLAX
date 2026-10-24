use super::*;
use crate::domain::money::Currency;
fn req() -> HistoryRequest {
    HistoryRequest {
        scan_id: "fixture".into(),
        item_key: "fixture".into(),
        window_days: 8,
        slippage_bps: 300,
    }
}
fn sample(day: u64, hour: u64, price: u64, volume: Option<u64>) -> HistorySample {
    HistorySample {
        at: day * 86400 + hour * 3600,
        price_major_1e4: price,
        volume,
    }
}
fn series(points: Vec<HistorySample>) -> HistorySeries {
    HistorySeries {
        currency: Some(Currency::cny()),
        currency_basis: "explicit_code_or_unique_marker".into(),
        response_rows: points.len() as u32,
        points,
        excluded_before_window: 0,
        excluded_open_day: 0,
        from_utc_day: 392,
        until_utc_day_exclusive: 400,
    }
}
#[test]
fn native_precision_is_kept_until_vwap_and_only_exact_calendar_pairs_count() {
    let s = series(vec![
        sample(392, 0, 12345, Some(1)),
        sample(392, 1, 23456, Some(1)),
        sample(392, 2, 999999, Some(0)),
        sample(392, 3, 999999, None),
        sample(399, 0, 10000, Some(2)),
    ]);
    let a = analyze(&s, &req(), 400 * 86400).unwrap();
    assert_eq!(a.daily_bars[0].vwap.as_ref().unwrap().minor, 179);
    assert_eq!(a.daily_bars[0].volume, 2);
    assert_eq!(a.missing_calendar_days, 6);
    let risk = a.risk.unwrap();
    assert_eq!(risk.sample_count, 1);
    assert_eq!(risk.return_p10_bps, Some(-4414));
    assert_eq!(a.reference_utc_day, Some(399));
    assert!(risk.conservative_gross.unwrap().minor < 100);
    let sparse = analyze(
        &series(vec![
            sample(392, 0, 10000, Some(1)),
            sample(398, 0, 10000, Some(1)),
        ]),
        &req(),
        400 * 86400,
    )
    .unwrap();
    assert_eq!(sparse.status, "insufficient_pairs");
    assert_eq!(sparse.risk.unwrap().conservative_gross, None);
}
#[test]
fn unknown_currency_empty_and_subminor_prices_remain_unknown_without_fallback() {
    let mut s = series(vec![sample(392, 0, 1, Some(10))]);
    let a = analyze(&s, &req(), 400 * 86400).unwrap();
    assert_eq!(a.below_minor_unit_days, 1);
    assert_eq!(a.status, "no_usable_daily_prices");
    assert!(a.risk.is_none());
    s.currency = None;
    s.currency_basis = "ambiguous_or_unsupported".into();
    let a = analyze(&s, &req(), 400 * 86400).unwrap();
    assert_eq!(a.status, "unknown_currency");
    assert!(a.daily_bars.is_empty() && a.risk.is_none());
    let a = analyze(&series(vec![]), &req(), 400 * 86400).unwrap();
    assert_eq!(a.status, "no_usable_daily_prices");
    assert_eq!(a.missing_calendar_days, 8);
}
#[test]
fn history_bounds_duplicates_and_currency_evidence_cannot_be_bypassed_by_a_provider() {
    let original = series(vec![sample(392, 0, 10000, Some(1))]);
    for variant in 0..6 {
        let mut s = original.clone();
        match variant {
            0 => s.points.push(s.points[0].clone()),
            1 => s.points[0].at = 400 * 86400,
            2 => s.points[0].price_major_1e4 = 0,
            3 => s.from_utc_day -= 1,
            4 => s.currency_basis = "assumed_cny".into(),
            _ => s.points[0].volume = Some(u64::MAX),
        };
        assert!(analyze(&s, &req(), 400 * 86400).is_err());
    }
}
#[test]
fn stability_is_optional_in_old_json_and_never_changes_raw_daily_prices_or_p10() {
    let s = series(vec![
        sample(392, 0, 10000, Some(1)),
        sample(399, 0, 20000, Some(1)),
    ]);
    let result = analyze(&s, &req(), 400 * 86400).unwrap();
    let metrics = result.stability.as_ref().unwrap();
    assert_eq!(metrics.usable_days, 2);
    assert!(metrics.ema7.ema.is_none());
    assert_eq!(result.risk.as_ref().unwrap().return_p10_bps, Some(10000));
    let mut json = serde_json::to_value(&result).unwrap();
    json.as_object_mut().unwrap().remove("stability");
    let old: HistoryAnalysis = serde_json::from_value(json).unwrap();
    assert!(old.stability.is_none());
    assert_eq!(old.daily_bars[1].vwap.as_ref().unwrap().minor, 200);
    assert_eq!(old.risk.unwrap().return_p10_bps, Some(10000));
}
