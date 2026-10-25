use super::*;
use crate::{
    adapters::{market::Provider, sqlite::Store},
    application::{Application, fx::FxResolver, ports::*, strategy::StrategyResearch},
    domain::{
        analysis::evaluate,
        fx::{SCALE, tests::fixture},
        planning::PlanRequest,
        selection::*,
        strategy::*,
    },
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;
struct Rates(FxSnapshot);
impl FxResolver for Rates {
    fn rates(&self) -> Pin<Box<dyn Future<Output = Result<FxSnapshot, AppError>> + Send + '_>> {
        Box::pin(async { Ok(self.0.clone()) })
    }
}
struct NoPlatform(Arc<AtomicUsize>);
impl OrderBookProvider for NoPlatform {
    async fn order_book(&self, _: &OrderBookRequest) -> Result<OrderBook, MarketReadError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(MarketReadError::Network)
    }
}
#[tokio::test]
async fn uah_depth_fees_display_budget_and_strategy_retain_original_evidence() {
    let at = now();
    let repo = Store::open(Path::new(":memory:")).unwrap();
    let mut rate = fixture(at);
    rate.rates.get_mut("UAH").unwrap().units_per_cny_1e8 = 6 * SCALE;
    let fx = Arc::new(Rates(rate.clone()));
    let uah: Currency = "UAH".to_string().try_into().unwrap();
    let snapshot = OrderBookSnapshot::new(
        OrderBookRequest {
            currency: uah.clone(),
            app_id: 730,
            market_hash_name: "UAH fixture".into(),
        },
        OrderBook {
            currency: uah.clone(),
            asks: vec![PriceLevel {
                price_minor: 6900,
                quantity: 30,
            }],
            bids: vec![PriceLevel {
                price_minor: 6600,
                quantity: 10,
            }],
            reported_buy_orders: None,
            reported_sell_orders: None,
        },
        at,
    )
    .unwrap();
    repo.insert_order_book(snapshot.clone()).await.unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let books = OrderBookResearch::with_fx(NoPlatform(calls.clone()), repo.clone(), fx.clone());
    let quote = books
        .valuation(snapshot.id.clone(), Currency::cny())
        .await
        .unwrap();
    assert_eq!(quote.best_ask.unwrap(), Money::cny(1150));
    let request = ListingPreviewRequest {
        display_currency: uah.clone(),
        policy: WallGapPolicy::default(),
        max_quote_age_secs: 300,
        fee_schedule: Some(FeeSchedule {
            currency: uah.clone(),
            platform_bps: 500,
            publisher_bps: 1000,
            platform_min_minor: 7,
            publisher_min_minor: 7,
            increment_minor: 1,
        }),
    };
    let priced = books
        .listing_preview(snapshot.id.clone(), request.clone())
        .await
        .unwrap();
    assert_eq!(
        priced.analysis.native_gross_listing_estimate.unwrap().minor,
        6894
    );
    assert_eq!(priced.display_gross_listing_estimate.unwrap().currency, uah);
    assert_eq!(priced.display_net_estimate.unwrap().minor, 5996);
    assert_eq!(priced.fee_estimate.unwrap().net.currency, uah);
    let mut bad = request;
    bad.fee_schedule.as_mut().unwrap().currency = Currency::cny();
    assert!(matches!(
        books.listing_preview(snapshot.id.clone(), bad).await,
        Err(AppError::Invalid(_))
    ));
    // The ordinary display endpoint can select UAH for any saved native book.
    let app_router = crate::transport::orderbook::router(books);
    let response = app_router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/steam/order-books/{}/valuation", snapshot.id))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"display_currency":"UAH"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["best_ask"]["minor"], 6900);
    assert_eq!(value["best_ask"]["currency"], "UAH");
    let mut report = ScanReport::new(ScanRequest {
        source: SourceKind::Steamdt,
        ..Default::default()
    });
    report.status = "completed".into();
    report.finished_at = Some(at);
    report.candidates.push(evaluate(
        MarketItem {
            key: "UAH fixture".into(),
            name: "UAH fixture".into(),
            goods_id: Some(1),
            cash_cents: 500,
            steam_gross_cents: 1000,
            volume: 300,
            observed_at: at,
        },
        &report.request,
    ));
    repo.save(report.clone()).await.unwrap();
    let application = Application::with_fx(Provider::new().unwrap(), repo.clone(), fx);
    let req:PlanRequest=serde_json::from_value(serde_json::json!({"scan_id":report.id,"budget_cents":2000,"target_wallet_cents":null,"max_units_per_item":2,"max_positions":2,"max_quote_age_secs":300,"steam_snapshot_ids":[snapshot.id]})).unwrap();
    let plan = application.create_plan(req).await.unwrap();
    assert_eq!(plan.lines[0].candidate.estimated_wallet_cents, 1000);
    let native = plan.steam_valuations[0].native_ask.as_ref().unwrap();
    assert_eq!(native.minor, 6900);
    assert_eq!(native.currency, uah);
    assert_eq!(plan.fx.as_ref().unwrap().id, rate.id);
    let research = StrategyResearch::new(repo.clone());
    let strategy = research
        .create(StrategyDraft {
            objective: Default::default(),
            schema_version: 1,
            name: "UAH fixture research".into(),
            description: "fixture only".into(),
            rules: vec![StrategyRule::StrictRatio {
                max_ratio_bps: 9000,
            }],
        })
        .await
        .unwrap();
    let evaluation = research
        .evaluate(StrategyEvaluationRequest {
            strategy_id: strategy.strategy_id,
            revision: 1,
            plan_id: plan.id.clone(),
            account_id: None,
            history_snapshot_ids: vec![],
            order_book_snapshot_ids: vec![],
        })
        .await
        .unwrap();
    assert_eq!(evaluation.status, ResearchStatus::Pass);
    assert!(!evaluation.execution_allowed);
    let saved = research.get(evaluation.id).await.unwrap();
    assert_eq!(
        saved.plan.fx.unwrap().rates["UAH"].units_per_cny_1e8,
        6 * SCALE
    );
    assert_eq!(
        repo.get_plan(plan.id)
            .await
            .unwrap()
            .unwrap()
            .steam_valuations[0]
            .native_ask
            .as_ref()
            .unwrap()
            .currency,
        uah
    );
    assert_eq!(
        repo.order_book(snapshot.id)
            .await
            .unwrap()
            .unwrap()
            .book
            .best_ask(),
        Some(6900)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
