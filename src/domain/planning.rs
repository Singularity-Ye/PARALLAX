//! Deterministic budget preview. A preview is never an executable order.
use super::orderbook::OrderBookSnapshot;
use super::selection::{Candidate, Coverage, ScanReport, SourceKind};
use super::{
    money::WalletPolicy,
    positions::{LiquidityPolicy, PositionSnapshot, remaining_units},
};
use serde::{Deserialize, Serialize};
fn default_position_age() -> u64 {
    300
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRequest {
    pub scan_id: String,
    pub budget_cents: u64,
    pub target_wallet_cents: Option<u64>,
    pub max_units_per_item: u32,
    pub max_positions: usize,
    pub max_quote_age_secs: u64,
    #[serde(default)]
    pub position_snapshot: Option<PositionSnapshot>,
    #[serde(default)]
    pub liquidity_policy: Option<LiquidityPolicy>,
    #[serde(default = "default_position_age")]
    pub max_position_age_secs: u64,
    #[serde(default)]
    pub steam_snapshot_ids: Vec<String>,
}
impl PlanRequest {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = std::collections::HashSet::new();
        if self.steam_snapshot_ids.len() > 20
            || self
                .steam_snapshot_ids
                .iter()
                .any(|id| id.is_empty() || id.len() > 64 || !ids.insert(id))
        {
            return Err("最多引用 20 个唯一 Steam 盘口快照 ID".into());
        }
        if let Some(snapshot) = &self.position_snapshot {
            snapshot.validate()?;
        }
        if let Some(policy) = &self.liquidity_policy {
            policy.validate()?;
            if self.position_snapshot.is_none() {
                return Err("启用日量约束需要账户仓位快照".into());
            }
        }
        if self.scan_id.is_empty() || self.scan_id.len() > 64 {
            return Err("扫描 ID 无效".into());
        }
        if !(1..=100_000_000).contains(&self.budget_cents)
            || self
                .target_wallet_cents
                .is_some_and(|v| v == 0 || v > 1_000_000_000)
            || !(1..=100).contains(&self.max_units_per_item)
            || !(1..=100).contains(&self.max_positions)
            || !(1..=3600).contains(&self.max_quote_age_secs)
            || !(1..=3600).contains(&self.max_position_age_secs)
        {
            return Err("预算、余额目标、持仓数量或报价有效期超出允许范围".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanLine {
    pub candidate: Candidate,
    pub quantity: u32,
    pub cash_total_cents: u64,
    pub estimated_wallet_total_cents: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_additional_units: Option<u32>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SkippedItem {
    pub key: String,
    pub reason: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanPreview {
    pub id: String,
    pub created_at: u64,
    pub status: String,
    pub algorithm: String,
    pub source: SourceKind,
    pub source_coverage: Coverage,
    pub source_stop_reason: String,
    pub request: PlanRequest,
    pub lines: Vec<PlanLine>,
    pub skipped: Vec<SkippedItem>,
    pub cash_total_cents: u64,
    pub remaining_budget_cents: u64,
    pub estimated_wallet_total_cents: u64,
    pub target_met: Option<bool>,
    pub execution_blockers: Vec<String>,
    #[serde(default)]
    pub reserved_cash_cents: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet_policy: Option<WalletPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steam_valuations: Vec<SteamValuationEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SteamValuationEvidence {
    pub snapshot_id: String,
    pub market_hash_name: String,
    pub observed_at: u64,
    pub gross_listing_estimate_cents: u64,
    pub best_bid_cents: Option<u64>,
}

pub fn preview(report: &ScanReport, request: PlanRequest, at: u64) -> Result<PlanPreview, String> {
    preview_with_order_books(report, request, at, &[])
}
pub fn preview_with_order_books(
    report: &ScanReport,
    request: PlanRequest,
    at: u64,
    books: &[OrderBookSnapshot],
) -> Result<PlanPreview, String> {
    request.validate()?;
    if report.id != request.scan_id || report.status != "completed" {
        return Err("只能从已完成的扫描创建计划，请先完成或重新扫描".into());
    }
    let reserved = request
        .position_snapshot
        .as_ref()
        .map_or(0, |s| s.reserved_cash_cents);
    if reserved > request.budget_cents {
        return Err("已预留现金超过输入现金预算".into());
    }
    if let Some(snapshot) = &request.position_snapshot
        && (snapshot.observed_at > at || at - snapshot.observed_at > request.max_position_age_secs)
    {
        return Err("仓位快照已过期或时间异常，请重新填报".into());
    }
    if books.len() != request.steam_snapshot_ids.len()
        || books
            .iter()
            .any(|b| !request.steam_snapshot_ids.contains(&b.id))
    {
        return Err("Steam 盘口快照与请求不一致".into());
    }
    if !books.is_empty() && report.request.source != SourceKind::Steamdt {
        return Err("演示扫描不能混入真实 Steam 盘口".into());
    }
    let mut overrides = std::collections::HashMap::new();
    let mut steam_valuations = Vec::new();
    for book in books {
        let key = &book.request.market_hash_name;
        if !report.candidates.iter().any(|c| &c.item.key == key) {
            return Err("Steam 盘口商品与扫描候选不匹配".into());
        }
        let ask = book.cny_ask(at, request.max_quote_age_secs)?;
        if overrides
            .insert(key.as_str(), (ask, book.observed_at))
            .is_some()
        {
            return Err("同一商品不能引用多个 Steam 盘口快照".into());
        }
        steam_valuations.push(SteamValuationEvidence {
            snapshot_id: book.id.clone(),
            market_hash_name: key.clone(),
            observed_at: book.observed_at,
            gross_listing_estimate_cents: ask,
            best_bid_cents: book.book.best_bid(),
        });
    }
    // Only the Steam side is refreshed; the BUFF ranking timestamp stays old.
    let mut candidates: Vec<_> = report
        .candidates
        .iter()
        .map(|c| {
            let mut item = c.item.clone();
            if let Some((ask, observed_at)) = overrides.get(item.key.as_str()) {
                item.steam_gross_cents = *ask;
                if item.observed_at <= at {
                    item.observed_at = item.observed_at.min(*observed_at);
                }
            }
            super::analysis::evaluate(item, &report.request)
        })
        .collect();
    candidates.sort_by(|a, b| {
        ((a.item.cash_cents as u128) * (b.estimated_wallet_cents as u128))
            .cmp(&((b.item.cash_cents as u128) * (a.estimated_wallet_cents as u128)))
            .then(a.item.key.cmp(&b.item.key))
    });
    let mut plan = PlanPreview {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: at,
        status: "preview_only".into(),
        algorithm: "ratio_greedy_v2".into(),
        source: report.request.source,
        source_coverage: report.coverage.clone(),
        source_stop_reason: report.stop_reason.clone(),
        remaining_budget_cents: request.budget_cents - reserved,
        reserved_cash_cents: reserved,
        wallet_policy: Some(report.request.wallet_policy.clone()),
        position_source: request
            .position_snapshot
            .as_ref()
            .map(|_| "manual_unverified".into()),
        steam_valuations,
        request,
        lines: vec![],
        skipped: vec![],
        cash_total_cents: 0,
        estimated_wallet_total_cents: 0,
        target_met: None,
        execution_blockers: vec![
            "仅为预算预览，未创建或预留订单".into(),
            "尚未复核实时报价、平台可购买数量与结算费用".into(),
            "手续费为模型估算；尚未接入真实历史风险和订单执行".into(),
        ],
    };
    if !plan.steam_valuations.is_empty() {
        plan.execution_blockers.push("已引用 Steam 最低卖价快照作慢卖估值；BUFF 仍为原榜单价，未刷新现金端报价；盘口不保证成交".into());
    }
    plan.execution_blockers
        .push(if plan.request.position_snapshot.is_some() {
            "仓位、日量和可买量来自用户手动填报，未通过平台验证".into()
        } else {
            "未提供账户仓位快照，数量仅受本计划上限约束".into()
        });
    if plan.source == SourceKind::Demo {
        plan.execution_blockers.push("使用固定演示数据".into());
    }
    for candidate in candidates {
        let reason = if !candidate.passed
            || candidate.ratio_bps.is_none()
            || candidate.item.cash_cents == 0
            || candidate.estimated_wallet_cents == 0
        {
            Some("未通过选品预筛或缺少有效报价")
        } else if candidate.item.observed_at > at
            || at - candidate.item.observed_at > plan.request.max_quote_age_secs
        {
            Some("报价已过期或时间异常，请重新扫描")
        } else if plan
            .request
            .target_wallet_cents
            .is_some_and(|t| plan.estimated_wallet_total_cents >= t)
        {
            Some("估算钱包目标已达到")
        } else if plan.lines.len() >= plan.request.max_positions {
            Some("达到本计划商品种类上限")
        } else if candidate.item.cash_cents > plan.remaining_budget_cents {
            Some("剩余预算不足以购买一件")
        } else {
            None
        };
        if let Some(reason) = reason {
            plan.skipped.push(SkippedItem {
                key: candidate.item.key.clone(),
                reason: reason.into(),
            });
            continue;
        }
        let snapshot = plan.request.position_snapshot.as_ref();
        let entry = snapshot.and_then(|s| s.entries.iter().find(|e| e.key == candidate.item.key));
        let limit = if snapshot.is_some() && entry.is_none() {
            Err("仓位快照未覆盖该商品；未假定零持仓".into())
        } else {
            remaining_units(
                candidate.item.cash_cents,
                plan.request.max_units_per_item,
                entry,
                plan.request.liquidity_policy.as_ref(),
            )
        };
        let max_additional = match limit {
            Ok(n) if n > 0 => n,
            other => {
                plan.skipped.push(SkippedItem {
                    key: candidate.item.key.clone(),
                    reason: other.err().unwrap_or_else(|| {
                        "持仓、未决买入、流动性或可买量约束后可增购数量为零".into()
                    }),
                });
                continue;
            }
        };
        let mut quantity = (plan.remaining_budget_cents / candidate.item.cash_cents)
            .min(u64::from(max_additional));
        if let Some(target) = plan.request.target_wallet_cents {
            quantity = quantity.min(
                (target - plan.estimated_wallet_total_cents)
                    .div_ceil(candidate.estimated_wallet_cents),
            );
        }
        let cash = candidate
            .item
            .cash_cents
            .checked_mul(quantity)
            .ok_or("计划现金金额溢出")?;
        let wallet = candidate
            .estimated_wallet_cents
            .checked_mul(quantity)
            .ok_or("计划钱包金额溢出")?;
        plan.cash_total_cents = plan
            .cash_total_cents
            .checked_add(cash)
            .ok_or("计划现金合计溢出")?;
        plan.remaining_budget_cents -= cash;
        plan.estimated_wallet_total_cents = plan
            .estimated_wallet_total_cents
            .checked_add(wallet)
            .ok_or("计划钱包合计溢出")?;
        plan.lines.push(PlanLine {
            candidate: candidate.clone(),
            quantity: quantity as u32,
            cash_total_cents: cash,
            estimated_wallet_total_cents: wallet,
            max_additional_units: Some(max_additional),
        });
    }
    plan.target_met = plan
        .request
        .target_wallet_cents
        .map(|t| plan.estimated_wallet_total_cents >= t);
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steam_override_is_explicit_preserves_buff_time_and_keeps_original_scan() {
        use crate::domain::{money::Currency, orderbook::*};
        let (mut report, mut request) = fixture();
        report.request.source = SourceKind::Steamdt;
        let name = report.candidates[0].item.key.clone();
        let snapshot = OrderBookSnapshot::new(
            OrderBookRequest {
                app_id: 730,
                market_hash_name: name.clone(),
            },
            OrderBook {
                currency: Currency::cny(),
                asks: vec![PriceLevel {
                    price_minor: 2300,
                    quantity: 1,
                }],
                bids: vec![],
                reported_buy_orders: None,
                reported_sell_orders: None,
            },
            101,
        )
        .unwrap();
        request.steam_snapshot_ids = vec![snapshot.id.clone()];
        let plan = preview_with_order_books(
            &report,
            request.clone(),
            101,
            std::slice::from_ref(&snapshot),
        )
        .unwrap();
        let item = &plan
            .lines
            .iter()
            .find(|l| l.candidate.item.key == name)
            .unwrap()
            .candidate;
        assert_eq!(item.estimated_wallet_cents, 2000);
        assert_eq!(item.item.observed_at, 100);
        assert_eq!(report.candidates[0].item.steam_gross_cents, 1150);
        assert_eq!(plan.steam_valuations.len(), 1);
        assert!(
            plan.execution_blockers
                .iter()
                .any(|b| b.contains("BUFF 仍为原榜单价"))
        );
        let mut future_report = report.clone();
        future_report.candidates[0].item.observed_at = 1000;
        let future = preview_with_order_books(
            &future_report,
            request.clone(),
            101,
            std::slice::from_ref(&snapshot),
        )
        .unwrap();
        assert!(
            !future.lines.iter().any(|l| l.candidate.item.key == name),
            "refresh must not hide a future BUFF timestamp"
        );
        // Fresh Steam data must not refresh the older BUFF ranking quote.
        let mut fresh = snapshot;
        fresh.observed_at = 401;
        let stale = preview_with_order_books(&report, request, 401, &[fresh]).unwrap();
        assert!(stale.lines.is_empty());
    }
    #[test]
    fn missing_mismatched_duplicate_or_demo_research_cannot_silently_apply() {
        use crate::domain::{money::Currency, orderbook::*};
        let (mut report, mut request) = fixture();
        let mut snapshot = OrderBookSnapshot::new(
            OrderBookRequest {
                app_id: 730,
                market_hash_name: report.candidates[0].item.key.clone(),
            },
            OrderBook {
                currency: Currency::cny(),
                asks: vec![PriceLevel {
                    price_minor: 2300,
                    quantity: 1,
                }],
                bids: vec![],
                reported_buy_orders: None,
                reported_sell_orders: None,
            },
            100,
        )
        .unwrap();
        request.steam_snapshot_ids = vec![snapshot.id.clone()];
        assert!(preview(&report, request.clone(), 101).is_err());
        assert!(
            preview_with_order_books(
                &report,
                request.clone(),
                101,
                std::slice::from_ref(&snapshot)
            )
            .is_err()
        );
        report.request.source = SourceKind::Steamdt;
        snapshot.request.market_hash_name = "different wear variant".into();
        assert!(
            preview_with_order_books(
                &report,
                request.clone(),
                101,
                std::slice::from_ref(&snapshot)
            )
            .is_err()
        );
        snapshot.request.market_hash_name = report.candidates[0].item.key.clone();
        snapshot.book.currency = "TWD".to_string().try_into().unwrap();
        assert!(
            preview_with_order_books(
                &report,
                request.clone(),
                101,
                std::slice::from_ref(&snapshot)
            )
            .is_err()
        );
        request.steam_snapshot_ids.push(snapshot.id.clone());
        assert!(request.validate().is_err());
    }
    use crate::domain::{
        analysis::evaluate,
        selection::{MarketItem, ScanRequest},
    };
    fn fixture() -> (ScanReport, PlanRequest) {
        let mut report = ScanReport::new(ScanRequest::default());
        report.status = "completed".into();
        report.candidates = [500, 700, 900]
            .into_iter()
            .enumerate()
            .map(|(i, cash)| {
                evaluate(
                    MarketItem {
                        key: i.to_string(),
                        name: i.to_string(),
                        goods_id: Some(i as u64 + 1),
                        cash_cents: cash,
                        steam_gross_cents: 1150,
                        volume: 300,
                        observed_at: 100,
                    },
                    &report.request,
                )
            })
            .collect();
        let request = PlanRequest {
            scan_id: report.id.clone(),
            budget_cents: 1800,
            target_wallet_cents: None,
            max_units_per_item: 2,
            max_positions: 2,
            max_quote_age_secs: 300,
            position_snapshot: None,
            liquidity_policy: None,
            max_position_age_secs: 300,
            steam_snapshot_ids: vec![],
        };
        (report, request)
    }
    #[test]
    fn budget_units_and_positions_are_enforced() {
        let (report, request) = fixture();
        let plan = preview(&report, request, 101).unwrap();
        assert_eq!(plan.cash_total_cents, 1700);
        assert_eq!(plan.remaining_budget_cents, 100);
        assert_eq!(plan.lines.len(), 2);
        assert_eq!(plan.lines[0].quantity, 2);
        assert_eq!(plan.status, "preview_only");
    }
    #[test]
    fn target_stops_allocation_and_stale_quotes_are_explained() {
        let (report, mut request) = fixture();
        request.target_wallet_cents = Some(1000);
        let plan = preview(&report, request.clone(), 101).unwrap();
        assert_eq!(plan.lines.len(), 1);
        assert_eq!(plan.lines[0].quantity, 1);
        assert_eq!(plan.target_met, Some(true));
        let stale = preview(&report, request, 401).unwrap();
        assert!(stale.lines.is_empty());
        assert_eq!(stale.skipped.len(), 3);
        assert_eq!(stale.target_met, Some(false));
    }
    #[test]
    fn incomplete_scans_cannot_produce_a_plan() {
        let (mut report, request) = fixture();
        report.status = "failed".into();
        assert!(preview(&report, request, 101).is_err());
    }
    fn snapshot() -> PositionSnapshot {
        PositionSnapshot {
            account_id: "fixture".into(),
            observed_at: 100,
            reserved_cash_cents: 500,
            entries: vec![super::super::positions::PositionEntry {
                key: "0".into(),
                held_units: 1,
                pending_buy_units: 0,
                available_units: Some(1),
                daily_volume: None,
            }],
        }
    }
    #[test]
    fn snapshot_reservations_exposure_and_uncovered_items_are_preserved() {
        let (report, mut request) = fixture();
        request.position_snapshot = Some(snapshot());
        let plan = preview(&report, request, 101).unwrap();
        assert_eq!(plan.lines.len(), 1);
        assert_eq!(plan.lines[0].quantity, 1);
        assert_eq!(plan.cash_total_cents, 500);
        assert_eq!(
            plan.cash_total_cents + plan.remaining_budget_cents + plan.reserved_cash_cents,
            plan.request.budget_cents
        );
        assert_eq!(plan.position_source.as_deref(), Some("manual_unverified"));
        assert!(plan.skipped.iter().any(|s| s.reason.contains("未覆盖")));
    }
    #[test]
    fn zero_availability_missing_daily_volume_and_stale_snapshot_do_not_allocate() {
        let (report, mut request) = fixture();
        let mut position = snapshot();
        position.entries[0].available_units = Some(0);
        request.position_snapshot = Some(position);
        assert!(
            preview(&report, request.clone(), 101)
                .unwrap()
                .lines
                .is_empty()
        );
        request.position_snapshot.as_mut().unwrap().entries[0].available_units = None;
        request.liquidity_policy = Some(LiquidityPolicy {
            participation_bps: 500,
            low_price_threshold_cents: 500,
            low_price_penalty_bps: 5000,
            low_price_cap: 30,
        });
        let plan = preview(&report, request.clone(), 101).unwrap();
        assert!(plan.lines.is_empty());
        assert!(plan.skipped.iter().any(|s| s.reason.contains("日成交量")));
        assert!(preview(&report, request.clone(), 99).is_err());
        assert!(preview(&report, request, 401).is_err());
    }
    #[test]
    fn cash_reservation_never_turns_unmet_wallet_target_into_spending() {
        let (report, mut request) = fixture();
        request.budget_cents = 600;
        request.target_wallet_cents = Some(1000);
        request.position_snapshot = Some(snapshot());
        let plan = preview(&report, request, 101).unwrap();
        assert_eq!(plan.cash_total_cents, 0);
        assert_eq!(plan.remaining_budget_cents, 100);
        assert_eq!(plan.target_met, Some(false));
    }
    #[test]
    fn old_preview_json_remains_a_readable_snapshot() {
        let (report, request) = fixture();
        let plan = preview(&report, request, 101).unwrap();
        let mut old = serde_json::to_value(&plan).unwrap();
        old.as_object_mut().unwrap().remove("steam_valuations");
        old["request"]
            .as_object_mut()
            .unwrap()
            .remove("steam_snapshot_ids");
        old["algorithm"] = "ratio_greedy_v1".into();
        for name in ["wallet_policy", "position_source", "reserved_cash_cents"] {
            old.as_object_mut().unwrap().remove(name);
        }
        for name in [
            "position_snapshot",
            "liquidity_policy",
            "max_position_age_secs",
        ] {
            old["request"].as_object_mut().unwrap().remove(name);
        }
        for line in old["lines"].as_array_mut().unwrap() {
            line.as_object_mut().unwrap().remove("max_additional_units");
            line["candidate"]
                .as_object_mut()
                .unwrap()
                .remove("valuation");
        }
        let restored: PlanPreview = serde_json::from_value(old).unwrap();
        assert_eq!(restored.algorithm, "ratio_greedy_v1");
        assert_eq!(restored.cash_total_cents, plan.cash_total_cents);
        assert_eq!(
            restored.estimated_wallet_total_cents,
            plan.estimated_wallet_total_cents
        );
        assert!(restored.wallet_policy.is_none());
    }
}
