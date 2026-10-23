use super::*;
use serde_json::json;
fn request() -> HistoryRequest {
    HistoryRequest {
        scan_id: "fixture".into(),
        item_key: "fixture".into(),
        window_days: 8,
        slippage_bps: 300,
    }
}
fn at() -> u64 {
    timestamp("Oct 23 2026 12: +0").unwrap()
}
#[test]
fn steam_dates_preserve_offsets_leap_days_and_reject_unknown_timezones() {
    assert_eq!(
        timestamp("Mar 01 2024 00: +0800").unwrap(),
        chrono::DateTime::parse_from_rfc3339("2024-02-29T16:00:00Z")
            .unwrap()
            .timestamp() as u64
    );
    assert_eq!(
        timestamp("Oct 22 2026 23: -5").unwrap(),
        timestamp("Oct 23 2026 04: +0").unwrap()
    );
    for bad in [
        "Feb 29 2023 00: +0",
        "Oct 23 2026 25: +0",
        "Oct 23 2026 00",
        "Oct 23 2026 00: +1460",
        "Oct 23 2026 00: +1401",
        "Oct 23 2026 00: UTC",
    ] {
        assert!(timestamp(bad).is_err(), "{bad}");
    }
}
#[test]
fn historical_prices_are_decimal_exact_and_do_not_invent_volumes_or_currency() {
    let raw: Value = serde_json::from_str("0.1234").unwrap();
    assert_eq!(price(&raw).unwrap(), 1234);
    for bad in ["1.00001", "1e2", "-1", "0", "1.", "NaN"] {
        assert!(price(&json!(bad)).is_err());
    }
    assert!(price(&serde_json::from_str::<Value>("1.000000000000000001").unwrap()).is_err());
    assert_eq!(volume(None).unwrap(), None);
    assert_eq!(volume(Some(&Value::Null)).unwrap(), None);
    assert_eq!(volume(Some(&json!("0"))).unwrap(), Some(0));
    assert!(volume(Some(&json!(-1))).is_err());
    for ambiguous in ["$", "¥", "", "JPY", "CHF"] {
        assert!(
            currency(&json!({"price_prefix":ambiguous,"price_suffix":""}))
                .unwrap()
                .is_none()
        );
    }
    for (prefix, suffix, code) in [
        ("HK$", "", "HKD"),
        ("NT$", "", "TWD"),
        ("¥", " CNY", "CNY"),
        ("$", " USD", "USD"),
        ("€", "", "EUR"),
    ] {
        assert_eq!(
            String::from(
                currency(&json!({"price_prefix":prefix,"price_suffix":suffix}))
                    .unwrap()
                    .unwrap()
            ),
            code
        );
    }
}
#[test]
fn history_windows_exclude_today_but_never_hide_bad_rows_duplicates_or_failed_responses() {
    let v = json!({"success":true,"price_prefix":"HK$","price_suffix":"","prices":[["Oct 23 2026 00: +0",1.01,"2"],["Oct 22 2026 00: +0",1.2345,"0"],["Oct 01 2026 00: +0",2,"3"]]});
    let series = parse(&serde_json::to_vec(&v).unwrap(), &request(), at()).unwrap();
    assert_eq!(series.points.len(), 1);
    assert_eq!(series.excluded_before_window, 1);
    assert_eq!(series.excluded_open_day, 1);
    assert_eq!(series.points[0].price_major_1e4, 12345);
    let mut duplicate = v.clone();
    duplicate["prices"]
        .as_array_mut()
        .unwrap()
        .push(v["prices"][1].clone());
    assert!(parse(&serde_json::to_vec(&duplicate).unwrap(), &request(), at()).is_err());
    for bad in [
        json!({"success":false,"prices":[]}),
        json!({"success":true}),
        json!({"success":true,"prices":[["Oct 24 2026 00: +0",1,"1"]]}),
        json!({"success":true,"prices":[["Oct 01 2026 00: +0",-1,"1"]]}),
    ] {
        assert!(parse(&serde_json::to_vec(&bad).unwrap(), &request(), at()).is_err());
    }
    assert!(
        parse(br#"{"success":true,"prices":[]}"#, &request(), at())
            .unwrap()
            .points
            .is_empty()
    );
}
