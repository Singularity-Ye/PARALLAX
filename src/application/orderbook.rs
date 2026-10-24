use super::{
    AppError,
    ports::{OrderBookProvider, OrderBookRepository},
};
use crate::domain::{
    money::{FeeQuote, FeeSchedule},
    orderbook::*,
    pricing::{ListingPriceAnalysis, WallGapPolicy, wall_gap_price},
    selection::now,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListingPreviewRequest {
    pub policy: WallGapPolicy,
    pub max_quote_age_secs: u64,
    #[serde(default)]
    pub fee_schedule: Option<FeeSchedule>,
}
impl ListingPreviewRequest {
    fn validate(&self) -> Result<(), String> {
        self.policy.validate()?;
        if !(1..=86400).contains(&self.max_quote_age_secs) {
            return Err("研究报价时效须为 1–86400 秒".into());
        }
        if let Some(schedule) = &self.fee_schedule {
            schedule.validate()?;
            if schedule.currency != self.policy.currency {
                return Err("费用模型与定价币种不一致".into());
            }
            // Keep API amounts within JS exact-integer range, including zero-price results.
            if schedule.increment_minor > 1_000_000_000_000
                || schedule.platform_min_minor > 1_000_000_000_000
                || schedule.publisher_min_minor > 1_000_000_000_000
            {
                return Err("费用参数超出支持范围".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListingPricePreview {
    pub preview_only: bool,
    pub snapshot_id: String,
    pub market: OrderBookRequest,
    pub source: String,
    pub identity_binding: String,
    pub observed_at: u64,
    pub computed_at: u64,
    pub age_secs: u64,
    pub freshness: String,
    pub request: ListingPreviewRequest,
    pub analysis: ListingPriceAnalysis,
    pub fee_estimate: Option<FeeQuote>,
    pub warnings: Vec<String>,
}

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
    /// Reuses immutable local depth only. Never calls the provider or writes an order/plan.
    pub async fn listing_preview(
        &self,
        id: String,
        request: ListingPreviewRequest,
    ) -> Result<ListingPricePreview, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let snapshot = self.get(id).await?;
        snapshot.request.validate().map_err(AppError::Invalid)?;
        let at = now();
        if snapshot.source != "steam_public_query_action" || snapshot.observed_at > at {
            return Err(AppError::Invalid(
                "盘口来源或时间异常，不能计算挂牌参考价".into(),
            ));
        }
        let analysis =
            wall_gap_price(&snapshot.book, &request.policy).map_err(AppError::Invalid)?;
        let fee_estimate = match (&request.fee_schedule, &analysis.gross_listing_estimate) {
            (Some(schedule), Some(gross)) => {
                schedule.net_from_gross(gross).map_err(AppError::Invalid)?
            }
            _ => None,
        };
        let age_secs = at - snapshot.observed_at;
        let fresh = age_secs <= request.max_quote_age_secs;
        let mut warnings = snapshot.warnings;
        warnings.push(
            "继承的壁垒/断层参数未用真实成交校准；参考价不是成交价、上架指令或收益保证".into(),
        );
        warnings
            .push("数量墙只依据返回的可见卖盘；剔除低量最低档与断层阻挡量是两个独立规则".into());
        if !fresh {
            warnings.push("快照已过所选时效，仅作历史研究；重新计算不会刷新平台报价时间".into());
        }
        if analysis.wall.as_ref().is_some_and(|w| !w.reached_threshold) {
            warnings
                .push("返回深度未累计到所选墙量，按可见末档截止，不能声称已找到完整价格墙".into());
        }
        if request.fee_schedule.is_some() {
            warnings.push(
                "净钱包估算使用显式费用模型，未经平台结算验证；null 表示无价格或没有可负担的正净额"
                    .into(),
            );
        } else {
            warnings.push("未提供费用模型，净钱包价值保持未知".into());
        }
        Ok(ListingPricePreview {
            preview_only: true,
            snapshot_id: snapshot.id,
            market: snapshot.request,
            source: snapshot.source,
            identity_binding: snapshot.identity_binding,
            observed_at: snapshot.observed_at,
            computed_at: at,
            age_secs,
            freshness: if fresh {
                "within_selected_age"
            } else {
                "historical_only"
            }
            .into(),
            request,
            analysis,
            fee_estimate,
            warnings,
        })
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
    async fn listing_preview_http_uses_saved_depth_and_never_refreshes_or_saves_it() {
        let repo = Store::open(Path::new(":memory:")).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let app = OrderBookResearch::new(
            Source {
                calls: calls.clone(),
                fail: true,
            },
            repo.clone(),
        );
        let snapshot = OrderBookSnapshot::new(
            OrderBookRequest {
                app_id: 730,
                market_hash_name: "saved fixture".into(),
            },
            OrderBook {
                currency: Currency::cny(),
                asks: vec![PriceLevel {
                    price_minor: 2300,
                    quantity: 30,
                }],
                bids: vec![],
                reported_buy_orders: None,
                reported_sell_orders: None,
            },
            now() - 301,
        )
        .unwrap();
        repo.insert_order_book(snapshot.clone()).await.unwrap();
        let request = ListingPreviewRequest {
            policy: WallGapPolicy::default(),
            max_quote_age_secs: 300,
            fee_schedule: Some(FeeSchedule {
                currency: Currency::cny(),
                platform_bps: 500,
                publisher_bps: 1000,
                platform_min_minor: 7,
                publisher_min_minor: 7,
                increment_minor: 1,
            }),
        };
        let router = crate::transport::orderbook::router(app.clone());
        let post = |id: &str, value: &ListingPreviewRequest| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/steam/order-books/{id}/listing-preview"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(value).unwrap()))
                .unwrap()
        };
        let response = router
            .clone()
            .oneshot(post(&snapshot.id, &request))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let result: ListingPricePreview =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert!(result.preview_only);
        assert_eq!(result.freshness, "historical_only");
        assert_eq!(result.observed_at, snapshot.observed_at);
        assert_eq!(result.analysis.gross_listing_estimate.unwrap().minor, 2299);
        assert!(result.fee_estimate.unwrap().net.minor < 2299);
        let mut cross = post(&snapshot.id, &request);
        cross
            .headers_mut()
            .insert("host", "127.0.0.1:38480".parse().unwrap());
        cross
            .headers_mut()
            .insert("origin", "https://untrusted.test".parse().unwrap());
        assert_eq!(
            router.clone().oneshot(cross).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            router
                .oneshot(post("missing", &request))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(app.list().await.unwrap().len(), 1);
        let saved = app.get(snapshot.id).await.unwrap();
        assert_eq!(saved.observed_at, snapshot.observed_at);
        assert_eq!(saved.book.asks[0].price_minor, 2300);
    }
    #[tokio::test]
    async fn listing_preview_rejects_foreign_future_and_fee_currency_mismatch() {
        let repo = Store::open(Path::new(":memory:")).unwrap();
        let app = OrderBookResearch::new(
            Source {
                calls: Arc::new(AtomicUsize::new(0)),
                fail: true,
            },
            repo.clone(),
        );
        let req = ListingPreviewRequest {
            policy: WallGapPolicy::default(),
            max_quote_age_secs: 300,
            fee_schedule: None,
        };
        for (currency, at) in [("TWD", now()), ("CNY", now() + 600)] {
            let snapshot = OrderBookSnapshot::new(
                OrderBookRequest {
                    app_id: 730,
                    market_hash_name: "fixture".into(),
                },
                OrderBook {
                    currency: currency.to_string().try_into().unwrap(),
                    asks: vec![PriceLevel {
                        price_minor: 100,
                        quantity: 1,
                    }],
                    bids: vec![],
                    reported_buy_orders: None,
                    reported_sell_orders: None,
                },
                at,
            )
            .unwrap();
            repo.insert_order_book(snapshot.clone()).await.unwrap();
            assert!(matches!(
                app.listing_preview(snapshot.id, req.clone()).await,
                Err(AppError::Invalid(_))
            ));
        }
        let mut bad = req.clone();
        bad.max_quote_age_secs = 0;
        assert!(bad.validate().is_err());
        bad = req;
        bad.fee_schedule = Some(FeeSchedule {
            currency: "USD".to_string().try_into().unwrap(),
            platform_bps: 0,
            publisher_bps: 0,
            platform_min_minor: 0,
            publisher_min_minor: 0,
            increment_minor: 1,
        });
        assert!(bad.validate().is_err());
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
