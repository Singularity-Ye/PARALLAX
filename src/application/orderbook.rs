use super::{
    AppError,
    ports::{OrderBookProvider, OrderBookRepository},
};
use crate::domain::{
    fx::FxSnapshot,
    money::{Currency, FeeQuote, FeeSchedule, Money},
    orderbook::*,
    pricing::{ListingPriceAnalysis, WallGapPolicy, wall_gap_price, wall_gap_price_with_rate},
    selection::now,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListingPreviewRequest {
    #[serde(default = "Currency::cny")]
    pub display_currency: Currency,
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
            if !crate::domain::fx::supported(&schedule.currency) {
                return Err("费用模型币种尚未适配".into());
            }
            // Keep API amounts within JS exact-integer range, including zero-price results.
            if schedule.increment_minor > 1_000_000_000_000
                || schedule.platform_min_minor > 1_000_000_000_000
                || schedule.publisher_min_minor > 1_000_000_000_000
            {
                return Err("费用参数超出支持范围".into());
            }
        }
        if !crate::domain::fx::supported(&self.display_currency) {
            return Err("显示币种尚未适配".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListingPricePreview {
    pub fx: Option<FxSnapshot>,
    pub display_currency: Currency,
    pub display_gross_listing_estimate: Option<Money>,
    pub display_net_estimate: Option<Money>,
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
    fx: Option<Arc<dyn super::fx::FxResolver>>,
}

impl<P: OrderBookProvider, R: OrderBookRepository> OrderBookResearch<P, R> {
    pub fn new(provider: P, repo: R) -> Arc<Self> {
        Arc::new(Self {
            provider,
            repo,
            fx: None,
        })
    }
    pub fn with_fx(provider: P, repo: R, fx: Arc<dyn super::fx::FxResolver>) -> Arc<Self> {
        Arc::new(Self {
            provider,
            repo,
            fx: Some(fx),
        })
    }
    async fn resolve_fx(
        &self,
        from: &Currency,
        to: &Currency,
    ) -> Result<Option<FxSnapshot>, AppError> {
        if !crate::domain::fx::supported(from) || !crate::domain::fx::supported(to) {
            return Err(AppError::Invalid("币种最小单位尚未适配".into()));
        }
        if from == to {
            return Ok(None);
        }
        let resolver = self
            .fx
            .as_ref()
            .ok_or(AppError::Invalid("外币换算需要有效汇率来源".into()))?;
        Ok(Some(resolver.rates().await?))
    }
    pub async fn valuation(
        &self,
        id: String,
        currency: Currency,
    ) -> Result<BookValuation, AppError> {
        let snapshot = self.get(id).await?;
        snapshot.book.validate().map_err(AppError::Invalid)?;
        let fx = self.resolve_fx(&snapshot.book.currency, &currency).await?;
        let convert = |amount: Option<u64>| {
            amount
                .map(|minor| {
                    let m = Money {
                        minor,
                        currency: snapshot.book.currency.clone(),
                    };
                    match &fx {
                        Some(f) => f.convert(&m, &currency).map_err(AppError::Invalid),
                        None => Ok(m),
                    }
                })
                .transpose()
        };
        Ok(BookValuation {
            snapshot_id: snapshot.id,
            observed_at: snapshot.observed_at,
            native_currency: snapshot.book.currency.clone(),
            display_currency: currency.clone(),
            best_ask: convert(snapshot.book.best_ask())?,
            best_bid: convert(snapshot.book.best_bid())?,
            fx,
        })
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
        if request
            .fee_schedule
            .as_ref()
            .is_some_and(|f| f.currency != snapshot.book.currency)
        {
            return Err(AppError::Invalid(
                "费用模型必须使用盘口原币；不能把人民币最低费套到外币".into(),
            ));
        }
        let fx = if snapshot.book.currency != Currency::cny() {
            self.resolve_fx(&snapshot.book.currency, &Currency::cny())
                .await?
        } else {
            self.resolve_fx(&snapshot.book.currency, &request.display_currency)
                .await?
        };
        let analysis = if snapshot.book.currency == Currency::cny() {
            wall_gap_price(&snapshot.book, &request.policy)
        } else {
            let (n, d) = fx
                .as_ref()
                .ok_or(AppError::Invalid("缺少汇率".into()))?
                .factor(&snapshot.book.currency, &Currency::cny())
                .map_err(AppError::Invalid)?;
            wall_gap_price_with_rate(&snapshot.book, &request.policy, n, d)
        }
        .map_err(AppError::Invalid)?;
        let fee_estimate = match (
            &request.fee_schedule,
            &analysis.native_gross_listing_estimate,
        ) {
            (Some(schedule), Some(gross)) => {
                schedule.net_from_gross(gross).map_err(AppError::Invalid)?
            }
            _ => None,
        };
        let display = |money: &Money| {
            if money.currency == request.display_currency {
                Ok(money.clone())
            } else {
                fx.as_ref()
                    .ok_or(AppError::Invalid("缺少显示币种汇率".into()))?
                    .convert(money, &request.display_currency)
                    .map_err(AppError::Invalid)
            }
        };
        let display_gross_listing_estimate = analysis
            .native_gross_listing_estimate
            .as_ref()
            .map(display)
            .transpose()?;
        let display_net_estimate = fee_estimate.as_ref().map(|f| display(&f.net)).transpose()?;
        let age_secs = at - snapshot.observed_at;
        let fresh = age_secs <= request.max_quote_age_secs;
        // Old immutable snapshots retain their creation-time restriction; report current policy.
        let mut warnings: Vec<String> = snapshot
            .warnings
            .into_iter()
            .map(|warning| {
                if warning == "平台实际返回非 CNY；保留原币，不换汇，不能用于人民币预算"
                {
                    "平台实际返回非 CNY；保留原币，当前显示与预算通过独立有效汇率快照换算".into()
                } else {
                    warning
                }
            })
            .collect();
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
        if fx.is_some() {
            warnings.push("换算使用带日期的参考汇率，不等于银行或Steam实际换汇结算价；原币挂牌参考向上取整到已适配最小单位，展示估值向下取整".into());
        }
        Ok(ListingPricePreview {
            fx,
            display_currency: request.display_currency.clone(),
            display_gross_listing_estimate,
            display_net_estimate,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookValuation {
    pub snapshot_id: String,
    pub observed_at: u64,
    pub native_currency: Currency,
    pub display_currency: Currency,
    pub best_ask: Option<Money>,
    pub best_bid: Option<Money>,
    pub fx: Option<FxSnapshot>,
}

#[cfg(test)]
#[path = "uah_tests.rs"]
mod uah_tests;

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
                currency: crate::domain::money::Currency::cny(),
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
            display_currency: Currency::cny(),
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
    async fn fx_research_and_budget_preserve_native_quotes_and_saved_rate_evidence() {
        struct Rates(crate::domain::fx::FxSnapshot);
        impl crate::application::fx::FxResolver for Rates {
            fn rates(
                &self,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<
                            Output = Result<crate::domain::fx::FxSnapshot, AppError>,
                        > + Send
                        + '_,
                >,
            > {
                Box::pin(async { Ok(self.0.clone()) })
            }
        }
        let at = now();
        let repo = Store::open(Path::new(":memory:")).unwrap();
        let fx = Arc::new(Rates(crate::domain::fx::tests::fixture(at)));
        let snapshot = OrderBookSnapshot::new(
            OrderBookRequest {
                currency: crate::domain::money::Currency::cny(),
                app_id: 730,
                market_hash_name: "fx fixture".into(),
            },
            OrderBook {
                currency: "TWD".to_string().try_into().unwrap(),
                asks: vec![PriceLevel {
                    price_minor: 11500,
                    quantity: 30,
                }],
                bids: vec![PriceLevel {
                    price_minor: 10000,
                    quantity: 1,
                }],
                reported_buy_orders: None,
                reported_sell_orders: None,
            },
            at,
        )
        .unwrap();
        repo.insert_order_book(snapshot.clone()).await.unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let app = OrderBookResearch::with_fx(
            Source {
                calls: calls.clone(),
                fail: true,
            },
            repo.clone(),
            fx.clone(),
        );
        let valuation = app
            .valuation(snapshot.id.clone(), Currency::cny())
            .await
            .unwrap();
        assert_eq!(valuation.best_ask.unwrap().minor, 2300);
        assert_eq!(valuation.fx.unwrap().id, fx.0.id);
        let request = ListingPreviewRequest {
            display_currency: Currency::cny(),
            policy: WallGapPolicy::default(),
            max_quote_age_secs: 300,
            fee_schedule: Some(FeeSchedule {
                currency: "TWD".to_string().try_into().unwrap(),
                platform_bps: 500,
                publisher_bps: 1000,
                platform_min_minor: 7,
                publisher_min_minor: 7,
                increment_minor: 1,
            }),
        };
        let preview = app
            .listing_preview(snapshot.id.clone(), request.clone())
            .await
            .unwrap();
        assert_eq!(
            preview
                .analysis
                .native_gross_listing_estimate
                .unwrap()
                .minor,
            11495
        );
        assert_eq!(preview.display_gross_listing_estimate.unwrap().minor, 2299);
        assert_eq!(
            String::from(preview.fee_estimate.unwrap().net.currency),
            "TWD"
        );
        assert_eq!(preview.fx.unwrap().id, fx.0.id);
        let mut wrong_fee = request;
        wrong_fee.fee_schedule.as_mut().unwrap().currency = Currency::cny();
        assert!(matches!(
            app.listing_preview(snapshot.id.clone(), wrong_fee).await,
            Err(AppError::Invalid(_))
        ));
        let router = crate::transport::orderbook::router(app);
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/steam/order-books/{}/valuation", snapshot.id))
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let application = Application::with_fx(Provider::new().unwrap(), repo.clone(), fx.clone());
        let mut report = ScanReport::new(ScanRequest {
            source: SourceKind::Steamdt,
            ..Default::default()
        });
        report.status = "completed".into();
        report.finished_at = Some(at);
        report.candidates = vec![evaluate(
            MarketItem {
                key: "fx fixture".into(),
                name: "fx fixture".into(),
                goods_id: Some(1),
                cash_cents: 500,
                steam_gross_cents: 1150,
                volume: 300,
                observed_at: at,
            },
            &report.request,
        )];
        repo.save(report.clone()).await.unwrap();
        let request:PlanRequest=serde_json::from_value(serde_json::json!({"scan_id":report.id,"budget_cents":2000,"target_wallet_cents":null,"max_units_per_item":2,"max_positions":2,"max_quote_age_secs":300,"steam_snapshot_ids":[snapshot.id]})).unwrap();
        let plan = application.create_plan(request).await.unwrap();
        assert_eq!(plan.lines[0].candidate.estimated_wallet_cents, 2000);
        assert_eq!(
            plan.steam_valuations[0].native_ask.as_ref().unwrap().minor,
            11500
        );
        assert_eq!(plan.steam_valuations[0].best_bid_cents, Some(2000));
        let saved = application.plan(plan.id).await.unwrap();
        assert_eq!(saved.fx.unwrap().id, fx.0.id);
        assert_eq!(
            repo.order_book(snapshot.id)
                .await
                .unwrap()
                .unwrap()
                .book
                .best_ask(),
            Some(11500)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
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
            display_currency: Currency::cny(),
            policy: WallGapPolicy::default(),
            max_quote_age_secs: 300,
            fee_schedule: None,
        };
        for (currency, at) in [("TWD", now()), ("CNY", now() + 600)] {
            let snapshot = OrderBookSnapshot::new(
                OrderBookRequest {
                    currency: crate::domain::money::Currency::cny(),
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
            currency: "JPY".to_string().try_into().unwrap(),
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
                currency: crate::domain::money::Currency::cny(),
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
