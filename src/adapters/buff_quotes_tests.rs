use super::*;
use serde_json::json;
fn payload() -> Value {
    json!({"code":"OK","data":{"page_num":1,"goods_infos":{"123":{"market_hash_name":"fixture (Field-Tested)"}},"items":[
    {"id":"order1","goods_id":123,"price":"2.50"},{"id":"order2","price":"1.25"},{"id":"order3","price":"1.25"},{"id":"order2","price":"1.25"}],"total_count":100}})
}
#[test]
fn decimal_prices_are_exact_and_duplicates_do_not_inflate_visible_quantity() {
    let page = parse(&payload(), 123).unwrap();
    assert_eq!(page.levels.len(), 2);
    assert_eq!(
        page.levels[0],
        PriceLevel {
            price_minor: 125,
            quantity: 2
        }
    );
    assert_eq!(page.reported_total, Some(100));
    for (raw, expected) in [("0.01", 1), ("1", 100), ("1.2", 120), ("001.20", 120)] {
        assert_eq!(cents(&json!(raw)).unwrap(), expected);
    }
    for raw in [
        "1.001",
        "1.",
        "0",
        "-1.2",
        "NaN",
        "1e3",
        " 1.2",
        "9999999999999999999999",
    ] {
        assert!(cents(&json!(raw)).is_err(), "{raw}");
    }
    assert!(
        cents(&json!(1.25)).is_err(),
        "fractional JSON numbers cannot be parsed via floating point"
    );
}
#[test]
fn empty_items_require_positive_identity_and_cannot_hide_protocol_failures() {
    let mut p = payload();
    p["data"]["items"] = json!([]);
    assert!(parse(&p, 123).unwrap().levels.is_empty());
    p["data"]["goods_infos"] = json!({});
    assert!(matches!(parse(&p, 123), Err(Error::IdentityMismatch)));
    let mut p = payload();
    p["data"].as_object_mut().unwrap().remove("items");
    assert!(parse(&p, 123).is_err());
    let mut p = payload();
    p["data"]["items"][0]["goods_id"] = json!(321);
    assert!(parse(&p, 123).is_err());
    let mut p = payload();
    p["data"]["items"][3]["price"] = json!("2.00");
    assert!(parse(&p, 123).is_err());
    let mut p = payload();
    p["data"]["items"][0]["amount"] = json!(2);
    assert!(parse(&p, 123).is_err());
    let mut p = payload();
    p["data"]["currency"] = json!("USD");
    assert!(parse(&p, 123).is_err());
}
#[test]
fn auth_challenge_rate_and_unexpected_redirects_are_distinct() {
    assert!(matches!(
        status_error(403, None, None),
        Some(Error::Challenge)
    ));
    assert!(matches!(
        status_error(412, None, None),
        Some(Error::Challenge)
    ));
    assert!(matches!(
        status_error(401, None, None),
        Some(Error::Expired)
    ));
    assert!(matches!(
        status_error(302, Some("https://evil.test/login"), None),
        Some(Error::Expired)
    ));
    assert!(matches!(
        status_error(302, Some("https://evil.test/"), None),
        Some(Error::Challenge)
    ));
    assert!(matches!(
        status_error(429, None, Some("1")),
        Some(Error::RateLimited(300))
    ));
    assert!(matches!(
        status_error(429, None, Some("999999999")),
        Some(Error::RateLimited(604800))
    ));
    assert!(matches!(
        payload_error(&json!({"code":"Login Required"})),
        Some(Error::Expired)
    ));
    assert!(matches!(
        payload_error(&json!({"code":"RiskControl"})),
        Some(Error::Challenge)
    ));
    assert!(matches!(
        payload_error(&json!({"code":"Unknown","msg":"too many requests"})),
        Some(Error::RateLimited(300))
    ));
}
#[test]
fn rotated_auth_cookie_is_not_silently_discarded_or_exposed() {
    let secret = Credentials {
        cookies: "session=old-canary; csrf_token=same".into(),
        user_agent: "fixture".into(),
    };
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "set-cookie",
        "session=old-canary; Path=/; Secure".parse().unwrap(),
    );
    assert!(rotation(&headers, &secret).is_none());
    headers.insert(
        "set-cookie",
        "session=new-canary; Path=/; HttpOnly".parse().unwrap(),
    );
    assert!(matches!(
        rotation(&headers, &secret),
        Some(Error::CredentialsChanged)
    ));
    headers.insert("set-cookie", "session=; Max-Age=0".parse().unwrap());
    assert!(matches!(rotation(&headers, &secret), Some(Error::Expired)));
    assert!(!Error::CredentialsChanged.message().contains("canary"));
}
