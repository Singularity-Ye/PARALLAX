use super::{
    AppError,
    ports::{OrderBookProvider, OrderBookRepository},
};
use crate::domain::{orderbook::*, selection::now};
use std::sync::Arc;

pub struct OrderBookResearch<P, R> {
    provider: P,
    repo: R,
}

impl<P: OrderBookProvider, R: OrderBookRepository> OrderBookResearch<P, R> {
    pub fn new(provider: P, repo: R) -> Arc<Self> {
        Arc::new(Self { provider, repo })
    }
    pub async fn fetch(&self, request: OrderBookRequest) -> Result<OrderBookSnapshot, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let book = self
            .provider
            .order_book(&request)
            .await
            .map_err(AppError::Market)?;
        let snapshot = OrderBookSnapshot::new(request, book, now()).map_err(AppError::Invalid)?;
        self.repo.insert_order_book(snapshot.clone()).await?;
        Ok(snapshot)
    }
    pub async fn get(&self, id: String) -> Result<OrderBookSnapshot, AppError> {
        self.repo
            .order_book(id)
            .await?
            .ok_or(AppError::Missing("盘口快照不存在"))
    }
    pub async fn list(&self) -> Result<Vec<OrderBookSnapshot>, AppError> {
        Ok(self.repo.order_books().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::{market::Provider, sqlite::Store},
        application::{Application, ports::*},
        domain::{
            analysis::evaluate,
            money::Currency,
            planning::PlanRequest,
            selection::{MarketItem, ScanReport, ScanRequest, SourceKind},
        },
    };
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use std::{
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use tower::ServiceExt;
    struct Source {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }
    impl OrderBookProvider for Source {
        async fn order_book(&self, _: &OrderBookRequest) -> Result<OrderBook, MarketReadError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(MarketReadError::RateLimited(now() + 300));
            }
            Ok(OrderBook {
                currency: Currency::cny(),
                asks: vec![PriceLevel {
                    price_minor: 2300,
                    quantity: 1,
                }],
                bids: vec![],
                reported_buy_orders: None,
                reported_sell_orders: None,
            })
        }
    }
    #[tokio::test]
    async fn research_http_persists_evidence_and_plan_consumes_it_without_changing_scan() {
        let repo = Store::open(Path::new(":memory:")).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let app = OrderBookResearch::new(
            Source {
                calls: calls.clone(),
                fail: false,
            },
            repo.clone(),
        );
        let router = crate::transport::orderbook::router(app.clone());
        let body = r#"{"app_id":730,"market_hash_name":"fixture wear variant"}"#;
        let post = || {
            Request::builder()
                .method("POST")
                .uri("/api/steam/order-books")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap()
        };
        let response = router.clone().oneshot(post()).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let saved: OrderBookSnapshot =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert_eq!(
            app.get(saved.id.clone()).await.unwrap().book.best_ask(),
            Some(2300)
        );
        assert!(
            repo.insert_order_book(saved.clone()).await.is_err(),
            "snapshots are immutable"
        );
        let application = Application::new(Provider::new().unwrap(), repo.clone());
        let mut report = ScanReport::new(ScanRequest {
            source: SourceKind::Steamdt,
            ..Default::default()
        });
        report.status = "completed".into();
        report.finished_at = Some(now());
        report.candidates = vec![evaluate(
            MarketItem {
                key: "fixture wear variant".into(),
                name: "fixture".into(),
                goods_id: Some(123),
                cash_cents: 500,
                steam_gross_cents: 1150,
                volume: 300,
                observed_at: now(),
            },
            &report.request,
        )];
        repo.save(report.clone()).await.unwrap();
        let mut request:PlanRequest=serde_json::from_value(serde_json::json!({"scan_id":report.id,"budget_cents":2000,"target_wallet_cents":null,"max_units_per_item":2,"max_positions":2,"max_quote_age_secs":300,"steam_snapshot_ids":[saved.id]})).unwrap();
        let plan = application.create_plan(request.clone()).await.unwrap();
        assert_eq!(plan.lines[0].candidate.estimated_wallet_cents, 2000);
        assert_eq!(
            application
                .plan(plan.id)
                .await
                .unwrap()
                .steam_valuations
                .len(),
            1
        );
        assert_eq!(
            repo.get(report.id).await.unwrap().unwrap().candidates[0]
                .item
                .steam_gross_cents,
            1150
        );
        request.steam_snapshot_ids = vec!["missing".into()];
        assert!(matches!(
            application.create_plan(request).await,
            Err(AppError::Missing(_))
        ));
        let mut cross = post();
        cross
            .headers_mut()
            .insert("host", "127.0.0.1:38480".parse().unwrap());
        cross
            .headers_mut()
            .insert("origin", "https://untrusted.test".parse().unwrap());
        assert_eq!(
            router.oneshot(cross).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn bad_request_and_provider_failure_do_not_save_fake_success() {
        let calls = Arc::new(AtomicUsize::new(0));
        let app = OrderBookResearch::new(
            Source {
                calls: calls.clone(),
                fail: true,
            },
            Store::open(Path::new(":memory:")).unwrap(),
        );
        assert!(
            app.fetch(OrderBookRequest {
                app_id: 570,
                market_hash_name: "fixture".into()
            })
            .await
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let router = crate::transport::orderbook::router(app.clone());
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/steam/order-books")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"app_id":730,"market_hash_name":"fixture"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
        assert!(body["retry_at"].as_u64().unwrap() > now());
        assert!(app.list().await.unwrap().is_empty());
    }
}
