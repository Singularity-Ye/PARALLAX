use super::*;
use crate::{
    adapters::sqlite::Store,
    application::{ports::*, sale_research::SaleResearchService},
    domain::{
        accounts::{Account, StoredSession},
        ledger::tests::fixture,
        sale_research::tests::research_fixture,
        selection::now,
    },
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
async fn call(app: Router, method: &str, url: &str, value: Value) -> (StatusCode, Value) {
    let res = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(url)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.headers()["cache-control"], "no-store");
    let status = res.status();
    (
        status,
        serde_json::from_slice(&to_bytes(res.into_body(), 8_000_000).await.unwrap()).unwrap(),
    )
}
async fn seed(
    store: &Store,
    at: u64,
) -> (PurchaseLot, crate::domain::inventory::InventorySnapshot) {
    let (lot, inv) = fixture(at);
    store
        .add_account(Account {
            id: lot.account_id.clone(),
            label: "fixture".into(),
            steam_id: inv.steam_id.clone(),
            created_at: at,
        })
        .await
        .unwrap();
    store.insert_inventory(inv.clone()).await.unwrap();
    (lot, inv)
}
fn saved(outcome: LedgerOutcome) -> PurchaseLot {
    match outcome {
        LedgerOutcome::Saved(s) => s.lot,
        _ => panic!("expected saved ledger write"),
    }
}
#[tokio::test]
async fn lifecycle_http_records_cost_and_native_wallet_once_survives_reopen() {
    let path = std::env::temp_dir().join(format!("parallax-ledger-{}.db", uuid::Uuid::new_v4()));
    let store = Store::open(&path).unwrap();
    let at = now();
    let (template, inv) = seed(&store, at).await;
    let app = router(PurchaseLedger::new(store.clone()));
    let base = "/api/accounts/fixture-account/purchase-lots";
    let (status, w) = call(app.clone(), "POST", base, json!(template.request)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(w["replayed"], false);
    let id = w["lot"]["id"].as_str().unwrap();
    let events = format!("{base}/{id}/events");
    assert_eq!(
        call(app.clone(), "POST", base, json!(template.request))
            .await
            .1["replayed"],
        true
    );
    let mut wrong = json!(template.request);
    wrong["cash_cost_cents"] = json!(1001);
    assert_eq!(
        call(app.clone(), "POST", base, wrong).await.0,
        StatusCode::CONFLICT
    );
    let receive = json!({"client_key":"r","expected_revision":1,"action":{"kind":"receive","inventory_snapshot_id":inv.id,"asset_ids":["1"]}});
    assert_eq!(
        call(app.clone(), "POST", &events, receive.clone()).await.1["lot"]["revision"],
        2
    );
    let list = json!({"client_key":"l","expected_revision":2,"action":{"kind":"list","asset_id":"1","listing_ref":"listing-1","gross":{"minor":5000,"currency":"UAH"},"listed_at":now()}});
    assert_eq!(
        call(app.clone(), "POST", &events, list).await.0,
        StatusCode::OK
    );
    // Retried commands do not replay side effects, and return the current projection.
    let replay = call(app.clone(), "POST", &events, receive).await.1;
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["lot"]["revision"], 3);
    let sell = json!({"client_key":"s","expected_revision":3,"action":{"kind":"sell","asset_id":"1","listing_ref":"listing-1","sale_ref":"sale-1","net_wallet":{"minor":4300,"currency":"UAH"},"sold_at":now()}});
    assert_eq!(
        call(app.clone(), "POST", &events, sell).await.1["lot"]["revision"],
        4
    );
    let h = call(
        app.clone(),
        "POST",
        "/api/accounts/fixture-account/holdings-preview",
        json!({"inventory_snapshot_id":null}),
    )
    .await
    .1;
    assert_eq!(h["open_cost_cents"], 666);
    assert_eq!(h["sold_cost_cents"], 334);
    assert_eq!(
        h["reported_wallet_receipts"][0],
        json!({"minor":4300,"currency":"UAH"})
    );
    assert_eq!(h["complete"], false);
    assert_eq!(
        call(app.clone(), "GET", &events, Value::Null).await.1["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let final_lot = call(app.clone(), "GET", &format!("{base}/{id}"), Value::Null)
        .await
        .1;
    assert_eq!(
        call(
            app.clone(),
            "GET",
            &format!("/api/accounts/other/purchase-lots/{id}"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    drop(app);
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        json!(
            store
                .lot("fixture-account".into(), id.into())
                .await
                .unwrap()
                .unwrap()
        ),
        final_lot
    );
    assert_eq!(
        store
            .lot_events("fixture-account".into(), id.into())
            .await
            .unwrap()
            .len(),
        3
    );
    drop(store);
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn concurrent_revision_and_duplicate_asset_claims_are_atomic_across_connections() {
    let path =
        std::env::temp_dir().join(format!("parallax-ledger-cas-{}.db", uuid::Uuid::new_v4()));
    let a = Store::open(&path).unwrap();
    let b = Store::open(&path).unwrap();
    let at = now();
    let (template, inv) = seed(&a, at).await;
    let l = saved(
        a.create_lot(template.account_id.clone(), template.request.clone(), at)
            .await
            .unwrap(),
    );
    let command = LotEventRequest {
        client_key: "race-a".into(),
        expected_revision: 1,
        action: LotAction::Receive {
            inventory_snapshot_id: inv.id.clone(),
            asset_ids: vec!["1".into()],
        },
    };
    let mut other = command.clone();
    other.client_key = "race-b".into();
    let (left, right) = tokio::join!(
        a.append_lot_event(
            l.account_id.clone(),
            l.id.clone(),
            command,
            Some(inv.clone()),
            at
        ),
        b.append_lot_event(
            l.account_id.clone(),
            l.id.clone(),
            other,
            Some(inv.clone()),
            at
        )
    );
    assert_ne!(
        matches!(left.unwrap(), LedgerOutcome::Saved(_)),
        matches!(right.unwrap(), LedgerOutcome::Saved(_))
    );
    let mut request = template.request;
    request.client_key = "second".into();
    request.purchase_ref = "second-purchase".into();
    let second = saved(
        b.create_lot(l.account_id.clone(), request, at)
            .await
            .unwrap(),
    );
    let mut command = LotEventRequest {
        client_key: "claim".into(),
        expected_revision: 1,
        action: LotAction::Receive {
            inventory_snapshot_id: inv.id.clone(),
            asset_ids: vec!["2".into(), "1".into()],
        },
    };
    assert!(matches!(
        b.append_lot_event(
            second.account_id.clone(),
            second.id.clone(),
            command.clone(),
            Some(inv.clone()),
            at
        )
        .await
        .unwrap(),
        LedgerOutcome::Conflict(_)
    ));
    // The earlier insert for asset 2 must have rolled back with the later conflicting asset 1.
    command.action = LotAction::Receive {
        inventory_snapshot_id: inv.id.clone(),
        asset_ids: vec!["2".into()],
    };
    let next = saved(
        b.append_lot_event(
            second.account_id.clone(),
            second.id.clone(),
            command,
            Some(inv),
            at,
        )
        .await
        .unwrap(),
    );
    assert_eq!(next.assets.len(), 1);
    drop(a);
    drop(b);
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn references_and_repeated_client_keys_cannot_silently_duplicate_financial_facts() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let at = now();
    let (template, inv) = seed(&store, at).await;
    let app = PurchaseLedger::new(store.clone());
    let l = app
        .create(template.account_id.clone(), template.request.clone())
        .await
        .unwrap()
        .lot;
    let mut duplicate = template.request;
    duplicate.client_key = "another-key".into();
    assert!(matches!(
        app.create(l.account_id.clone(), duplicate).await,
        Err(crate::application::AppError::Conflict(_))
    ));
    let l = app
        .append(
            l.account_id.clone(),
            l.id.clone(),
            LotEventRequest {
                client_key: "receive".into(),
                expected_revision: 1,
                action: LotAction::Receive {
                    inventory_snapshot_id: inv.id,
                    asset_ids: vec!["1".into(), "2".into()],
                },
            },
        )
        .await
        .unwrap()
        .lot;
    let command = LotEventRequest {
        client_key: "list".into(),
        expected_revision: l.revision,
        action: LotAction::List {
            asset_id: "1".into(),
            listing_ref: "unique-listing".into(),
            gross: crate::domain::money::Money::cny(600),
            listed_at: now(),
        },
    };
    let l = app
        .append(l.account_id.clone(), l.id.clone(), command.clone())
        .await
        .unwrap()
        .lot;
    let mut command = command;
    command.client_key = "other-list".into();
    command.expected_revision = l.revision;
    if let LotAction::List { asset_id, .. } = &mut command.action {
        *asset_id = "2".into();
    }
    assert!(matches!(
        app.append(l.account_id.clone(), l.id.clone(), command)
            .await,
        Err(crate::application::AppError::Conflict(_))
    ));
    assert_eq!(
        store
            .lot(l.account_id, l.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        3
    );
}
#[tokio::test]
async fn unknown_fields_and_cross_site_mutations_never_create_lots() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let (l, _) = seed(&store, now()).await;
    let app = router(PurchaseLedger::new(store.clone()));
    let url = "/api/accounts/fixture-account/purchase-lots";
    let mut invalid = json!(l.request);
    invalid["platform_verified"] = json!(true);
    assert_eq!(
        call(app.clone(), "POST", url, invalid).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(url)
                .header("content-type", "application/json")
                .header("origin", "https://external.example")
                .header("host", "127.0.0.1:38480")
                .body(Body::from(json!(l.request).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(store.lots(l.account_id).await.unwrap().is_empty());
}
#[tokio::test]
async fn sale_research_is_immutable_account_bound_and_independent_of_later_lot_edits() {
    let path = std::env::temp_dir().join(format!("parallax-sale-{}.db", uuid::Uuid::new_v4()));
    let store = Store::open(&path).unwrap();
    let at = now();
    let (mut req, template, e, inv, context) = research_fixture(at);
    store.add_account(context.account.clone()).await.unwrap();
    store.insert_inventory(inv.clone()).await.unwrap();
    store.insert_evaluation(e.clone()).await.unwrap();
    for view in context.sessions {
        store
            .save_session(
                template.account_id.clone(),
                StoredSession {
                    view,
                    credential_key: "unread-private-key".into(),
                },
            )
            .await
            .unwrap();
    }
    let ledger = PurchaseLedger::new(store.clone());
    let l = ledger
        .create(template.account_id.clone(), template.request)
        .await
        .unwrap()
        .lot;
    let l = ledger
        .append(
            l.account_id.clone(),
            l.id.clone(),
            LotEventRequest {
                client_key: "r".into(),
                expected_revision: 1,
                action: LotAction::Receive {
                    inventory_snapshot_id: inv.id.clone(),
                    asset_ids: vec!["1".into()],
                },
            },
        )
        .await
        .unwrap()
        .lot;
    req.lot_id = l.id.clone();
    let service = SaleResearchService::new(store.clone());
    let api = crate::transport::sale_research::router(service.clone());
    let (status, v) = call(
        api.clone(),
        "POST",
        "/api/accounts/fixture-account/sale-research",
        json!(req),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(v["action"], "review_listing");
    assert_eq!(v["execution_allowed"], false);
    assert!(!v.to_string().contains("unread-private-key"));
    let id = v["id"].as_str().unwrap().to_owned();
    assert!(
        store
            .insert_sale_research(serde_json::from_value(v.clone()).unwrap())
            .await
            .is_err()
    );
    ledger
        .append(
            l.account_id.clone(),
            l.id.clone(),
            LotEventRequest {
                client_key: "l".into(),
                expected_revision: l.revision,
                action: LotAction::List {
                    asset_id: "1".into(),
                    listing_ref: "listing".into(),
                    gross: crate::domain::money::Money::cny(5000),
                    listed_at: now(),
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(service.list(l.account_id.clone()).await.unwrap().len(), 1);
    assert_eq!(
        call(
            api,
            "GET",
            &format!("/api/accounts/other/sale-research/{id}"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    drop(service);
    drop(ledger);
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(
        json!(
            store
                .sale_research(l.account_id, id)
                .await
                .unwrap()
                .unwrap()
        ),
        v
    );
    drop(store);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn ledger_dashboard_is_account_scoped_paginated_read_only_and_no_store() {
    let store = Store::open(std::path::Path::new(":memory:")).unwrap();
    let at = now();
    let (template, _) = seed(&store, at).await;
    let app = router(PurchaseLedger::new(store.clone()));
    let base = "/api/accounts/fixture-account/ledger";
    let empty = call(app.clone(), "GET", &format!("{base}/summary"), Value::Null).await;
    assert_eq!(empty.0, StatusCode::OK);
    assert_eq!(empty.1["reported_purchase_cost"]["minor"], 0);
    assert!(empty.1["total_cash_profit"]["value"].is_null());
    store
        .create_lot("fixture-account".into(), template.request, at)
        .await
        .unwrap();
    let sum = call(app.clone(), "GET", &format!("{base}/summary"), Value::Null)
        .await
        .1;
    assert_eq!(sum["reported_purchase_cost"]["minor"], 1000);
    assert_eq!(sum["counts"]["pending_units"], 3);
    let page = call(
        app.clone(),
        "GET",
        &format!("{base}/rows?limit=1"),
        Value::Null,
    )
    .await
    .1;
    assert_eq!(page["total"], 1);
    assert_eq!(page["items"][0]["quantity"], 3);
    assert!(page["items"][0]["asset_id"].is_null());
    assert_eq!(page["items"][0]["purchase_cost"]["minor"], 1000);
    assert!(page["items"][0]["current_market_price"]["value"].is_null());
    assert_eq!(
        call(
            app.clone(),
            "GET",
            &format!("{base}/rows?limit=0"),
            Value::Null
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            app.clone(),
            "GET",
            "/api/accounts/other/ledger/summary",
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            app.clone(),
            "GET",
            "/api/accounts/other/ledger/rows",
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    store
        .add_account(Account {
            id: "other-existing".into(),
            label: "other".into(),
            steam_id: "76561198000000002".into(),
            created_at: at,
        })
        .await
        .unwrap();
    let other = call(
        app.clone(),
        "GET",
        "/api/accounts/other-existing/ledger/summary",
        Value::Null,
    )
    .await
    .1;
    assert_eq!(other["counts"]["lots"], 0);
    assert_eq!(other["reported_purchase_cost"]["minor"], 0);
    let other_rows = call(
        app.clone(),
        "GET",
        "/api/accounts/other-existing/ledger/rows",
        Value::Null,
    )
    .await
    .1;
    assert_eq!(other_rows["total"], 0);
    let lots = store.lots("fixture-account".into()).await.unwrap();
    assert_eq!(lots.len(), 1);
    assert_eq!(lots[0].revision, 1);
    assert!(
        store
            .lot_events("fixture-account".into(), lots[0].id.clone())
            .await
            .unwrap()
            .is_empty()
    );
}
