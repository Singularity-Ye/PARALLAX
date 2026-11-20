//! Phase-2 per-candidate verification: real Steam order book wall price + Steam price history.
//! Serial, paced, cancellable. Platform errors never become fake numbers; they stay "unverified".
use super::{
    fx::FxResolver,
    history::HistoryResearch,
    orderbook::OrderBookResearch,
    ports::{
        AccountRepository, CredentialStore, HistoryProvider, HistoryRepository,
        InventoryRepository, OrderBookProvider, OrderBookRepository, Repository, SessionVerifier,
    },
};
use crate::domain::{
    accounts::{Platform, SessionStatus},
    history::HistoryRequest,
    money::{Currency, ratio_bps},
    orderbook::OrderBookRequest,
    pricing::{WallGapPolicy, wall_gap_price, wall_gap_price_with_rate},
    selection::{CandidateDeepAnalysis, RuleResult, ScanLog, ScanReport, now},
};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Debug, Default)]
pub struct OrderBookProbe {
    /// Wall/gap listing estimate converted to CNY cents.
    pub smart_price_cny_cents: Option<u64>,
    pub top5_ask_qty: u64,
    pub book_currency: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct HistoryProbe {
    pub snapshot_id: String,
    pub usable_days: usize,
    pub clean_cv_bps: Option<u64>,
    pub r_squared_bps: Option<u32>,
    pub slope_bps_per_day: Option<i64>,
    pub range_position_bps: Option<i64>,
    pub ema7_cents: Option<u64>,
    pub ema30_cents: Option<u64>,
    pub return_p10_bps: Option<i64>,
    pub currency: String,
    pub reused_cache: bool,
}

pub trait DeepProbeProvider: Send + Sync {
    fn probe_orderbook(
        &self,
        market_hash_name: String,
    ) -> Pin<Box<dyn Future<Output = OrderBookProbe> + Send + '_>>;

    fn probe_history(
        &self,
        market_hash_name: String,
        window_days: u32,
    ) -> Pin<Box<dyn Future<Output = Result<HistoryProbe, String>> + Send + '_>>;
}

pub struct LiveDeepProbe<OP, R, C, V, HP> {
    pub order_books: Arc<OrderBookResearch<OP, R>>,
    pub histories: Arc<HistoryResearch<R, C, V, HP>>,
    pub fx: Arc<dyn FxResolver>,
    pub store: R,
}

impl<OP, R, C, V, HP> DeepProbeProvider for LiveDeepProbe<OP, R, C, V, HP>
where
    OP: OrderBookProvider + 'static,
    R: Repository
        + AccountRepository
        + OrderBookRepository
        + HistoryRepository
        + InventoryRepository
        + Clone
        + 'static,
    C: CredentialStore + 'static,
    V: SessionVerifier + 'static,
    HP: HistoryProvider + 'static,
{
    fn probe_orderbook(
        &self,
        market_hash_name: String,
    ) -> Pin<Box<dyn Future<Output = OrderBookProbe> + Send + '_>> {
        Box::pin(async move {
            let req = OrderBookRequest {
                app_id: 730,
                market_hash_name,
                currency: Currency::cny(),
            };
            let snap = match self.order_books.fetch(req).await {
                Ok(s) => s,
                Err(err) => {
                    return OrderBookProbe {
                        error: Some(err.to_string()),
                        ..Default::default()
                    };
                }
            };
            let top5_ask_qty: u64 = snap.book.asks.iter().take(5).map(|l| l.quantity).sum();
            let book_currency = String::from(snap.book.currency.clone());
            let policy = WallGapPolicy::default();
            let analysis = if snap.book.currency == Currency::cny() {
                wall_gap_price(&snap.book, &policy)
            } else {
                match self.fx.rates().await {
                    Ok(fx_snap) => match fx_snap.factor(&snap.book.currency, &Currency::cny()) {
                        Ok((n, d)) => wall_gap_price_with_rate(&snap.book, &policy, n, d),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e.to_string()),
                }
            };
            match analysis {
                Ok(a) => OrderBookProbe {
                    smart_price_cny_cents: a.gross_listing_estimate.map(|m| m.minor),
                    top5_ask_qty,
                    book_currency,
                    error: None,
                },
                Err(e) => OrderBookProbe {
                    top5_ask_qty,
                    book_currency,
                    error: Some(e),
                    ..Default::default()
                },
            }
        })
    }

    fn probe_history(
        &self,
        market_hash_name: String,
        window_days: u32,
    ) -> Pin<Box<dyn Future<Output = Result<HistoryProbe, String>> + Send + '_>> {
        Box::pin(async move {
            let accounts = self.store.accounts().await.map_err(|e| e.to_string())?;
            let mut chosen_account = None;
            for acc in &accounts {
                if let Ok(Some(sess)) = self.store.session(acc.id.clone(), Platform::Steam).await
                    && sess.view.status == SessionStatus::Valid
                {
                    chosen_account = Some(acc.id.clone());
                    break;
                }
            }
            let Some(account_id) = chosen_account else {
                return Err("尚无已验证的 Steam 登录态（请在「账号管理」完成 Steam 验证）".into());
            };
            // Reuse a recent (< 24h) saved snapshot for the exact item to save Steam quota.
            let current_ts = now();
            if let Ok(existing) = self.store.histories(account_id.clone()).await
                && let Some(fresh) = existing.into_iter().find(|h| {
                    h.request.item_key == market_hash_name
                        && current_ts.saturating_sub(h.observed_at) <= 86_400
                        && h.analysis.stability.is_some()
                })
            {
                return Ok(extract_history_probe(&fresh, true));
            }
            let req = HistoryRequest {
                scan_id: String::new(),
                inventory_snapshot_id: None,
                manual_item: true,
                item_key: market_hash_name,
                window_days: window_days.clamp(8, 365),
                slippage_bps: 300,
            };
            let snap = self
                .histories
                .fetch(&account_id, req)
                .await
                .map_err(|e| e.to_string())?;
            Ok(extract_history_probe(&snap, false))
        })
    }
}

fn extract_history_probe(
    snap: &crate::domain::history::HistorySnapshot,
    reused_cache: bool,
) -> HistoryProbe {
    let st = snap.analysis.stability.as_ref();
    let currency = snap
        .series
        .currency
        .as_ref()
        .map(|c| String::from(c.clone()))
        .unwrap_or_default();
    HistoryProbe {
        snapshot_id: snap.id.clone(),
        usable_days: st.map_or(snap.analysis.daily_bars.len(), |s| s.usable_days),
        clean_cv_bps: st.and_then(|s| s.clean_sample_cv_bps),
        r_squared_bps: st.and_then(|s| s.trend7.r_squared_bps),
        slope_bps_per_day: st.and_then(|s| s.trend7.relative_slope_bps_per_day),
        range_position_bps: st.and_then(|s| s.reference_range_position_bps),
        ema7_cents: st.and_then(|s| s.ema7.ema.as_ref().map(|m| m.minor)),
        ema30_cents: st.and_then(|s| s.ema30.ema.as_ref().map(|m| m.minor)),
        return_p10_bps: snap.analysis.risk.as_ref().and_then(|r| r.return_p10_bps),
        currency,
        reused_cache,
    }
}

pub fn log(report: &mut ScanReport, level: &str, item: Option<&str>, message: String) {
    if report.logs.len() >= 2000 {
        report.logs.remove(0);
    }
    report.logs.push(ScanLog {
        at: now(),
        level: level.into(),
        item: item.map(Into::into),
        message,
    });
}

fn short(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() > 28 {
        format!("{}…", chars[..28].iter().collect::<String>())
    } else {
        name.into()
    }
}

/// Runs stage 2 per-item deep verification with cooldowns, persisting progress after each item.
pub async fn run_deep_verification<R: Repository>(
    store: &R,
    probe: &dyn DeepProbeProvider,
    report: &mut ScanReport,
    cancel: &AtomicBool,
) {
    let limit = report.request.deep_verify_limit;
    if limit == 0 {
        report.phase = "done".into();
        return;
    }
    let targets: Vec<usize> = report
        .candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.passed)
        .map(|(i, _)| i)
        .take(limit)
        .collect();
    let total = targets.len();
    if total == 0 {
        log(
            report,
            "info",
            None,
            "初筛无通过候选，跳过阶段 2 深度复核。".into(),
        );
        report.phase = "done".into();
        return;
    }

    report.status = "running".into();
    report.phase = "deep_verify".into();
    report.deep_total = total;
    report.deep_done = 0;
    let interval_secs = report.request.deep_interval_secs;
    let history_days = report.request.deep_history_days;
    log(
        report,
        "info",
        None,
        format!(
            "进入阶段 2/2 逐件深度复核：共 {total} 件初筛通过候选，冷却 {interval_secs} 秒/件（Steam 十档壁垒 + {history_days} 日历史曲线）"
        ),
    );
    let _ = store.save(report.clone()).await;

    let mut passed_count = 0usize;
    let mut rejected_count = 0usize;

    for (pos, idx) in targets.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            report.stop_reason = "cancelled".into();
            report.status = "cancelled".into();
            log(
                report,
                "warn",
                None,
                format!("用户主动停止深度复核（已完成 {pos}/{total} 件），保留已核验结果。"),
            );
            let _ = store.save(report.clone()).await;
            return;
        }

        // Paced cooldown between candidates (cancellable every 250ms).
        if pos > 0 && interval_secs > 0 {
            let ticks = interval_secs * 4;
            for _ in 0..ticks {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            if cancel.load(Ordering::Relaxed) {
                report.stop_reason = "cancelled".into();
                report.status = "cancelled".into();
                log(
                    report,
                    "warn",
                    None,
                    format!("用户主动停止深度复核（已完成 {pos}/{total} 件），保留已核验结果。"),
                );
                let _ = store.save(report.clone()).await;
                return;
            }
        }

        let item_key = report.candidates[idx].item.key.clone();
        let item_name = report.candidates[idx].item.name.clone();
        let label = format!("[{}/{total}] {}", pos + 1, short(&item_name));
        log(
            report,
            "info",
            Some(&item_key),
            format!("{label} 开始核验：拉取 Steam 实时卖单深度…"),
        );
        let _ = store.save(report.clone()).await;

        // Step 2A: Steam live orderbook + wall/gap price & sell pressure.
        let ob = probe.probe_orderbook(item_key.clone()).await;
        let ob_rejected = apply_orderbook_probe(report, idx, &ob, &label);

        // Step 2B: Only query Steam pricehistory if orderbook didn't already reject this candidate.
        if !ob_rejected {
            log(
                report,
                "info",
                Some(&item_key),
                format!("{label} 盘口过关，拉取 Steam {history_days} 日历史量价曲线…"),
            );
            let _ = store.save(report.clone()).await;
            let hist_res = probe.probe_history(item_key.clone(), history_days).await;
            let keep = apply_history_probe(report, idx, hist_res, &label);
            if keep {
                passed_count += 1;
            } else {
                rejected_count += 1;
            }
        } else {
            rejected_count += 1;
        }

        report.deep_done = pos + 1;
        let _ = store.save(report.clone()).await;
    }

    report.status = "completed".into();
    report.phase = "done".into();
    report.finished_at = Some(now());
    log(
        report,
        "ok",
        None,
        format!(
            "深度复核完毕：共核验 {total} 件，通过/保留 {passed_count} 件，淘汰 {rejected_count} 件。"
        ),
    );
}

/// Applies OrderBookProbe to candidate `idx`. Returns `true` if the candidate is rejected.
fn apply_orderbook_probe(
    report: &mut ScanReport,
    idx: usize,
    ob: &OrderBookProbe,
    label: &str,
) -> bool {
    let request = report.request.clone();
    let key = report.candidates[idx].item.key.clone();
    let cash = report.candidates[idx].item.cash_cents;
    let volume = report.candidates[idx].item.volume;
    let mut deep = report.candidates[idx]
        .deep_analysis
        .clone()
        .unwrap_or_default();

    let mut rejected: Option<String> = None;
    let mut rules = Vec::new();
    let mut lines = Vec::new();

    match (&ob.error, ob.smart_price_cny_cents) {
        (Some(err), _) => {
            lines.push((
                "warn",
                format!("{label} Steam 卖单读取失败：{err}；保留初筛参考，未核验壁垒"),
            ));
        }
        (None, None) => {
            lines.push((
                "warn",
                format!("{label} Steam 卖盘为空或无法计算壁垒价；保留初筛参考"),
            ));
        }
        (None, Some(smart)) => {
            deep.smart_price_cents = smart;
            deep.ob_verified = true;
            if volume > 0 {
                let daily_slot = (volume as f64 / 10.0).max(1.0);
                deep.sell_pressure = ((ob.top5_ask_qty as f64 / daily_slot) * 100.0).round() / 100.0;
                deep.sell_pressure_ok = deep.sell_pressure <= 5.0;
            }
            let valuation = request.wallet_policy.estimate(smart).ok().flatten();
            let net = valuation.as_ref().map_or(0, |v| v.net.minor);
            let real = ratio_bps(cash, net);
            deep.real_ratio_bps = real;
            // Sync candidate display figures with real verified Steam wall price & net wallet.
            report.candidates[idx].item.steam_gross_cents = smart;
            report.candidates[idx].estimated_wallet_cents = net;
            report.candidates[idx].ratio_bps = real;
            report.candidates[idx].valuation = valuation;

            let ratio_text = real.map_or("—".into(), |r| format!("{:.2}%", r as f64 / 100.0));
            let ratio_ok = real.is_some_and(|r| r <= request.max_ratio_bps);
            lines.push((
                if ratio_ok && deep.sell_pressure_ok {
                    "info"
                } else {
                    "warn"
                },
                format!(
                    "{label} Steam 壁垒价 ¥{:.2}（净得 ¥{:.2} · 原币 {}）· 真实折算 {} {} 上限 {:.2}% · 卖压 {:.2}（前5档 {} 件）",
                    smart as f64 / 100.0,
                    net as f64 / 100.0,
                    ob.book_currency,
                    ratio_text,
                    if ratio_ok { "≤" } else { ">" },
                    request.max_ratio_bps as f64 / 100.0,
                    deep.sell_pressure,
                    ob.top5_ask_qty
                ),
            ));
            rules.push(RuleResult {
                rule: "Steam 壁垒复核".into(),
                passed: ratio_ok,
                reason: format!(
                    "BUFF ¥{:.2} / 壁垒净得 ¥{:.2} → {ratio_text}（上限 {:.2}%）",
                    cash as f64 / 100.0,
                    net as f64 / 100.0,
                    request.max_ratio_bps as f64 / 100.0
                ),
            });
            if !ratio_ok {
                rejected = Some(format!(
                    "真实折算 {ratio_text} > 上限 {:.2}%",
                    request.max_ratio_bps as f64 / 100.0
                ));
            } else if !deep.sell_pressure_ok {
                rules.push(RuleResult {
                    rule: "Steam 卖压深度".into(),
                    passed: false,
                    reason: format!("前5档 {} 件 / 日量基准 → 卖压 {:.2} > 5.00", ob.top5_ask_qty, deep.sell_pressure),
                });
                rejected = Some(format!("卖压 {:.2} 过高（前5档 {} 件）", deep.sell_pressure, ob.top5_ask_qty));
            }
        }
    }

    if let Some(reason) = &rejected {
        lines.push(("warn", format!("{label} → 淘汰：{reason}")));
        deep.stability_passed = false;
        deep.summary = format!("× 盘口淘汰：{reason}");
        report.candidates[idx].passed = false;
        report.coverage.accepted = report.coverage.accepted.saturating_sub(1);
        report.coverage.rejected += 1;
    }

    report.candidates[idx].rules.extend(rules);
    report.candidates[idx].deep_analysis = Some(deep);
    for (level, msg) in lines {
        log(report, level, Some(&key), msg);
    }
    rejected.is_some()
}

/// Applies HistoryProbe to candidate `idx`. Returns `true` if the candidate remains accepted.
fn apply_history_probe(
    report: &mut ScanReport,
    idx: usize,
    hist_res: Result<HistoryProbe, String>,
    label: &str,
) -> bool {
    let max_cv = report.request.deep_max_cv_bps;
    let min_slope = report.request.deep_min_slope_bps;
    let key = report.candidates[idx].item.key.clone();
    let mut deep = report.candidates[idx]
        .deep_analysis
        .clone()
        .unwrap_or_default();

    let mut rejected: Option<String> = None;
    let mut rules = Vec::new();
    let mut lines = Vec::new();

    match hist_res {
        Ok(h) => {
            deep.history_verified = true;
            deep.history_snapshot_id = Some(h.snapshot_id.clone());
            deep.history_days = h.usable_days as u32;
            if let Some(cv) = h.clean_cv_bps {
                deep.cv_30d = cv as f64 / 10_000.0;
            }
            deep.r_squared = h.r_squared_bps.map_or(0.0, |r| r as f64 / 10_000.0);
            deep.slope_bps_per_day = h.slope_bps_per_day;
            if let Some(pos) = h.range_position_bps {
                deep.price_percentile = (pos.clamp(0, 10_000) / 100) as u8;
            }
            if h.currency == "CNY" {
                if let Some(e7) = h.ema7_cents {
                    deep.ema7_cents = e7;
                }
                if let Some(e30) = h.ema30_cents {
                    deep.ema30_cents = e30;
                }
            }
            deep.trend_status = match h.slope_bps_per_day {
                Some(s) if s >= 30 => "RISING",
                Some(s) if s <= -40 => "FALLING",
                Some(_) => "STABLE",
                None => "UNKNOWN",
            }
            .into();

            let cv_ok = h.clean_cv_bps.is_some_and(|cv| cv <= max_cv);
            let slope_ok = h.slope_bps_per_day.is_none_or(|s| s >= min_slope);
            let cv_str = h
                .clean_cv_bps
                .map_or("未知".into(), |v| format!("{:.2}%", v as f64 / 100.0));
            let slope_str = h
                .slope_bps_per_day
                .map_or("未知".into(), |v| format!("{:+.2}%/日", v as f64 / 100.0));
            let r2_str = h
                .r_squared_bps
                .map_or("未知".into(), |v| format!("{:.3}", v as f64 / 10_000.0));
            let p10_str = h
                .return_p10_bps
                .map_or("—".into(), |v| format!("{:+.2}%", v as f64 / 100.0));
            let cache_tag = if h.reused_cache { "（复用24h内缓存）" } else { "" };

            lines.push((
                if cv_ok && slope_ok { "info" } else { "warn" },
                format!(
                    "{label} 历史 {} 有效日{cache_tag} · CV {cv_str}（上限 {:.2}%）· 7日斜率 {slope_str} · R² {r2_str} · 分位 P{} · P10回撤 {p10_str}",
                    h.usable_days,
                    max_cv as f64 / 100.0,
                    deep.price_percentile
                ),
            ));
            rules.push(RuleResult {
                rule: "历史波动 CV".into(),
                passed: cv_ok,
                reason: format!("清洗 CV {cv_str} / 上限 {:.2}%", max_cv as f64 / 100.0),
            });
            rules.push(RuleResult {
                rule: "历史趋势斜率".into(),
                passed: slope_ok,
                reason: format!("7日相对斜率 {slope_str} / 下限 {:+.2}%/日", min_slope as f64 / 100.0),
            });
            if !cv_ok {
                rejected = Some(format!("历史波动 CV {cv_str} 超标或样本不足"));
            } else if !slope_ok {
                rejected = Some(format!("近 7 日下跌趋势 {slope_str}"));
            }
        }
        Err(err) => {
            lines.push((
                "warn",
                format!("{label} 历史曲线暂未读取：{err}（盘口结果已保留）"),
            ));
        }
    }

    deep.verified = deep.ob_verified && deep.history_verified;
    let keep = rejected.is_none();
    if let Some(reason) = &rejected {
        lines.push(("warn", format!("{label} → 淘汰：{reason}")));
        deep.stability_passed = false;
        deep.summary = format!("× 曲线淘汰：{reason}");
        report.candidates[idx].passed = false;
        report.coverage.accepted = report.coverage.accepted.saturating_sub(1);
        report.coverage.rejected += 1;
    } else if deep.verified {
        lines.push(("ok", format!("{label} → ✓ 盘口与历史曲线双通过")));
        deep.stability_passed = true;
        deep.summary = format!(
            "✓ 双核验通过 · CV {:.3} · R² {:.2} · P{}",
            deep.cv_30d, deep.r_squared, deep.price_percentile
        );
    } else if deep.ob_verified {
        lines.push(("ok", format!("{label} → ✓ Steam 盘口壁垒已通过（历史待登录复核）")));
        deep.stability_passed = false;
        deep.summary = format!("✓ 盘口壁垒已核验 · 卖压 {:.2}", deep.sell_pressure);
    } else {
        lines.push(("info", format!("{label} → 保留初筛结果（Steam 暂未核验）")));
        deep.stability_passed = false;
        deep.summary = "保留初筛（Steam 暂未核验）".into();
    }

    report.candidates[idx].rules.extend(rules);
    report.candidates[idx].deep_analysis = Some(deep);
    for (level, msg) in lines {
        log(report, level, Some(&key), msg);
    }
    keep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::sqlite::Store;
    use crate::domain::{
        analysis::evaluate,
        selection::{MarketItem, ScanRequest, SourceKind},
    };
    use std::sync::atomic::AtomicUsize;

    struct MockProbe {
        history_calls: AtomicUsize,
    }
    impl DeepProbeProvider for MockProbe {
        fn probe_orderbook(
            &self,
            name: String,
        ) -> Pin<Box<dyn Future<Output = OrderBookProbe> + Send + '_>> {
            Box::pin(async move {
                match name.as_str() {
                    "item-pass" => OrderBookProbe {
                        smart_price_cny_cents: Some(2000), // net ~1739, cash 1000 => 57.5% <= 75%
                        top5_ask_qty: 20,
                        book_currency: "CNY".into(),
                        error: None,
                    },
                    "item-bad-wall" => OrderBookProbe {
                        smart_price_cny_cents: Some(1200), // net ~1043, cash 1000 => 95.88% > 75%
                        top5_ask_qty: 10,
                        book_currency: "TWD".into(),
                        error: None,
                    },
                    _ => OrderBookProbe {
                        smart_price_cny_cents: Some(2000),
                        top5_ask_qty: 15,
                        book_currency: "CNY".into(),
                        error: None,
                    },
                }
            })
        }

        fn probe_history(
            &self,
            name: String,
            _: u32,
        ) -> Pin<Box<dyn Future<Output = Result<HistoryProbe, String>> + Send + '_>> {
            self.history_calls.fetch_add(1, Ordering::Relaxed);
            Box::pin(async move {
                if name == "item-pass" {
                    Ok(HistoryProbe {
                        snapshot_id: "snap-1".into(),
                        usable_days: 30,
                        clean_cv_bps: Some(320), // 3.2% <= 10%
                        r_squared_bps: Some(6400),
                        slope_bps_per_day: Some(15),
                        range_position_bps: Some(4500),
                        ema7_cents: Some(1980),
                        ema30_cents: Some(1950),
                        return_p10_bps: Some(-280),
                        currency: "CNY".into(),
                        reused_cache: false,
                        ..Default::default()
                    })
                } else {
                    Ok(HistoryProbe {
                        snapshot_id: "snap-2".into(),
                        usable_days: 30,
                        clean_cv_bps: Some(2200), // 22% > 10% -> reject
                        r_squared_bps: Some(2000),
                        slope_bps_per_day: Some(-250),
                        range_position_bps: Some(9000),
                        currency: "CNY".into(),
                        reused_cache: false,
                        ..Default::default()
                    })
                }
            })
        }
    }

    #[tokio::test]
    async fn deep_verification_filters_wall_ratio_short_circuits_history_and_checks_cv() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("test.db")).unwrap();
        let req = ScanRequest {
            source: SourceKind::Steamdt,
            max_ratio_bps: 7500,
            deep_verify_limit: 5,
            deep_interval_secs: 0,
            deep_max_cv_bps: 1000,
            deep_min_slope_bps: -100,
            deep_history_days: 30,
            ..ScanRequest::default()
        };
        let mut report = ScanReport::new(req.clone());
        for key in ["item-pass", "item-bad-wall", "item-bad-cv"] {
            let c = evaluate(
                MarketItem {
                    key: key.into(),
                    name: key.into(),
                    goods_id: Some(100),
                    icon_url: None,
                    cash_cents: 1000,
                    steam_gross_cents: 2000,
                    volume: 300,
                    observed_at: now(),
                },
                &req,
            );
            assert!(c.passed);
            report.coverage.examined += 1;
            report.coverage.accepted += 1;
            report.candidates.push(c);
        }
        store.save(report.clone()).await.unwrap();

        let probe = MockProbe {
            history_calls: AtomicUsize::new(0),
        };
        run_deep_verification(&store, &probe, &mut report, &AtomicBool::new(false)).await;

        assert_eq!(report.status, "completed");
        assert_eq!(report.phase, "done");
        assert_eq!(report.deep_total, 3);
        assert_eq!(report.deep_done, 3);
        // item-bad-wall failed orderbook ratio -> history was NOT queried for it (only 2 history calls!)
        assert_eq!(probe.history_calls.load(Ordering::Relaxed), 2);
        assert!(report.candidates[0].passed);
        assert!(report.candidates[0].deep_analysis.as_ref().unwrap().verified);
        assert!(!report.candidates[1].passed);
        assert!(!report.candidates[2].passed);
        assert_eq!(report.coverage.accepted, 1);
        assert_eq!(report.coverage.rejected, 2);
        assert!(report.logs.len() >= 6);
    }
}
