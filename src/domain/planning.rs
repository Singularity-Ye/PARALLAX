//! Deterministic budget preview. A preview is never an executable order.
use super::selection::{Candidate, Coverage, ScanReport, SourceKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanRequest {
    pub scan_id: String,
    pub budget_cents: u64,
    pub target_wallet_cents: Option<u64>,
    pub max_units_per_item: u32,
    pub max_positions: usize,
    pub max_quote_age_secs: u64,
}
impl PlanRequest {
    pub fn validate(&self) -> Result<(), String> {
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
}

pub fn preview(report: &ScanReport, request: PlanRequest, at: u64) -> Result<PlanPreview, String> {
    request.validate()?;
    if report.id != request.scan_id || report.status != "completed" {
        return Err("只能从已完成的扫描创建计划，请先完成或重新扫描".into());
    }
    let mut candidates: Vec<_> = report.candidates.iter().collect();
    candidates.sort_by(|a, b| {
        a.ratio_bps
            .cmp(&b.ratio_bps)
            .then(a.item.key.cmp(&b.item.key))
    });
    let mut plan = PlanPreview {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: at,
        status: "preview_only".into(),
        algorithm: "ratio_greedy_v1".into(),
        source: report.request.source,
        source_coverage: report.coverage.clone(),
        source_stop_reason: report.stop_reason.clone(),
        remaining_budget_cents: request.budget_cents,
        request,
        lines: vec![],
        skipped: vec![],
        cash_total_cents: 0,
        estimated_wallet_total_cents: 0,
        target_met: None,
        execution_blockers: vec![
            "仅为预算预览，未创建或预留订单".into(),
            "尚未复核实时报价、可购买数量与手续费".into(),
            "尚未接入历史风险、账户持仓和订单执行".into(),
        ],
    };
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
        let mut quantity = (plan.remaining_budget_cents / candidate.item.cash_cents)
            .min(u64::from(plan.request.max_units_per_item));
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
        plan.cash_total_cents += cash;
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
}
