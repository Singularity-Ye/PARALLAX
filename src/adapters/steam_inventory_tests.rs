use super::*;
use reqwest::{StatusCode, header::HeaderMap};
use serde_json::json;
fn payload() -> Value {
    json!({"success":1,"total_inventory_count":1,"more_items":0,
        "assets":[{"appid":730,"contextid":"2","assetid":"101","classid":"10","instanceid":"3","amount":"2"}],
        "descriptions":[{"appid":730,"classid":"10","instanceid":"3","market_hash_name":"fixture (Factory New)","tradable":1,"marketable":1,"owner_descriptions":[]}]})
}
fn read(v: &Value) -> Result<InventoryPage, E> {
    parse(&serde_json::to_vec(v).unwrap(), "2")
}
#[test]
fn inventory_parser_keeps_exact_asset_instance_amount_and_unknown_names() {
    let v = payload();
    let p = read(&v).unwrap();
    assert_eq!(p.assets[0].amount, 2);
    assert!(p.assets[0].reported_transferable());
    let mut v = v;
    v["descriptions"][0]
        .as_object_mut()
        .unwrap()
        .remove("market_hash_name");
    let p = read(&v).unwrap();
    assert!(p.assets[0].market_hash_name.is_none());
    v["descriptions"][0]["owner_descriptions"] =
        json!([{"value":"Trade protected until a future date <b>HTML</b>"}]);
    assert!(!read(&v).unwrap().assets[0].reported_transferable());
    v["assets"][0]["contextid"] = json!("16");
    let p = parse(&serde_json::to_vec(&v).unwrap(), "16").unwrap();
    assert!(!p.assets[0].reported_transferable());
    assert!(read(&v).is_err());
}
#[test]
fn inventory_parser_distinguishes_empty_incomplete_and_conflicting_payloads() {
    assert!(
        read(&json!({"success":1,"total_inventory_count":0}))
            .unwrap()
            .assets
            .is_empty()
    );
    for v in [
        json!({"success":0}),
        json!({"success":1}),
        json!({"success":1,"total_inventory_count":1}),
        json!({"success":1,"total_inventory_count":0,"more_items":1}),
    ] {
        assert!(read(&v).is_err());
    }
    for field in ["classid", "instanceid"] {
        let mut v = payload();
        v["descriptions"][0][field] = json!("0");
        assert!(read(&v).is_err());
    }
    let mut v = payload();
    v["more_items"] = json!(1);
    v["last_assetid"] = json!("999");
    assert!(read(&v).is_err());
    v["last_assetid"] = json!("101");
    assert_eq!(read(&v).unwrap().next_asset_id.as_deref(), Some("101"));
    let mut v = payload();
    let mut duplicate = v["assets"][0].clone();
    duplicate["amount"] = json!("4");
    v["assets"].as_array_mut().unwrap().push(duplicate);
    v["total_inventory_count"] = json!(2);
    assert!(read(&v).is_err());
    let mut v = payload();
    let mut d = v["descriptions"][0].clone();
    d["tradable"] = json!(0);
    v["descriptions"].as_array_mut().unwrap().push(d);
    assert!(read(&v).is_err());
    for (field, value) in [
        ("appid", json!(440)),
        ("amount", json!("0")),
        ("assetid", json!("01")),
        ("amount", json!(1)),
    ] {
        let mut v = payload();
        v["assets"][0][field] = value;
        assert!(read(&v).is_err());
    }
}
#[test]
fn inventory_transport_errors_do_not_confuse_privacy_with_login_or_ignore_rotation() {
    let secret = Credentials {
        cookies: "steamLoginSecure=76561198000000001%7C%7Csynthetic; sessionid=synthetic-session"
            .into(),
        user_agent: "fixture".into(),
    };
    let mut h = HeaderMap::new();
    assert_eq!(
        classify(StatusCode::FORBIDDEN, &h, &secret),
        Err(E::Rejected)
    );
    assert_eq!(
        classify(StatusCode::UNAUTHORIZED, &h, &secret),
        Err(E::Expired)
    );
    h.insert("retry-after", "9999999999".parse().unwrap());
    assert_eq!(
        classify(StatusCode::TOO_MANY_REQUESTS, &h, &secret),
        Err(E::RateLimited(604800))
    );
    h.insert(
        "location",
        "https://untrusted.example/login".parse().unwrap(),
    );
    assert_eq!(classify(StatusCode::FOUND, &h, &secret), Err(E::Rejected));
    h.insert("location", "/login/home".parse().unwrap());
    assert_eq!(classify(StatusCode::FOUND, &h, &secret), Err(E::Expired));
    h.insert(
        "set-cookie",
        "sessionid=rotated; Path=/; Secure".parse().unwrap(),
    );
    assert_eq!(
        classify(StatusCode::OK, &h, &secret),
        Err(E::CredentialsChanged)
    );
    h.insert("set-cookie", "steamLoginSecure=; Path=/".parse().unwrap());
    assert_eq!(classify(StatusCode::OK, &h, &secret), Err(E::Expired));
    h.insert(
        "set-cookie",
        "sessionid=synthetic-session; Path=/".parse().unwrap(),
    );
    assert!(classify(StatusCode::OK, &h, &secret).is_ok());
}
