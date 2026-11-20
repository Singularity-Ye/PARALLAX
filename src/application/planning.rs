use super::ports::{MarketSource, Repository};
use super::{AppError, Application, ports};
use crate::domain::{
    analysis::compute_safe_purchase_limit,
    ledger::AssetStage,
    planning::{PlanPreview, PlanRequest, preview_with_fx_quotes},
    positions::{LiquidityPolicy, PositionEntry, PositionSnapshot},
    selection::{ScanReport, now},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_budget_cents() -> u64 {
    10_000
}
fn default_max_units() -> u32 {
    50
}
fn default_max_positions() -> usize {
    8
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AutoPlanRequest {
    #[serde(default)]
    pub scan_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default = "default_budget_cents")]
    pub budget_cents: u64,
    #[serde(default)]
    pub target_wallet_cents: Option<u64>,
    #[serde(default = "default_max_units")]
    pub max_units_per_item: u32,
    #[serde(default = "default_max_positions")]
    pub max_positions: usize,
    #[serde(default)]
    pub selected_item_keys: Option<Vec<String>>,
}

impl<S: MarketSource + 'static, R: Repository + 'static> Application<S, R> {
    pub(super) async fn collect_account_holdings(
        &self,
        account_id: Option<&str>,
    ) -> Result<(Option<String>, HashMap<String, (u32, u32)>), AppError>
    where
        R: ports::AccountRepository + ports::InventoryRepository + ports::LedgerRepository,
    {
        let accounts = self.store.accounts().await?;
        let target_id = if let Some(id) = account_id.filter(|s| !s.trim().is_empty()) {
            Some(id.to_string())
        } else {
            accounts.first().map(|a| a.id.clone())
        };
        let mut holdings: HashMap<String, (u32, u32)> = HashMap::new();
        let Some(acc_id) = target_id else {
            return Ok((None, holdings));
        };
        // 1. Latest Steam inventory snapshot
        if let Ok(snaps) = self.store.inventories(acc_id.clone()).await
            && let Some(latest) = snaps.first()
        {
            for asset in &latest.assets {
                if let Some(name) = &asset.market_hash_name {
                    let entry = holdings.entry(name.clone()).or_insert((0, 0));
                    entry.0 = entry.0.saturating_add(asset.amount.max(1));
                }
            }
        }
        // 2. Active purchase lots (unreceived pending_buy_units + held/listed lot assets if not in inventory)
        if let Ok(lots) = self.store.lots(acc_id.clone()).await {
            for lot in lots {
                let pending = lot.pending_units();
                let active_lot_assets = lot
                    .assets
                    .iter()
                    .filter(|a| matches!(a.stage, AssetStage::Held | AssetStage::Listed))
                    .count() as u32;
                let entry = holdings.entry(lot.request.item_key.clone()).or_insert((0, 0));
                if entry.0 < active_lot_assets {
                    entry.0 = active_lot_assets;
                }
                entry.1 = entry.1.saturating_add(pending);
            }
        }
        Ok((Some(acc_id), holdings))
    }

    pub async fn enrich_scan_report(&self, report: &mut ScanReport) -> Result<(), AppError>
    where
        R: ports::AccountRepository
            + ports::InventoryRepository
            + ports::LedgerRepository
            + ports::OrderBookRepository,
    {
        let (_, holdings) = self.collect_account_holdings(None).await?;
        let books = self.store.order_books().await.unwrap_or_default();
        let mut latest_books = HashMap::new();
        for b in books {
            latest_books
                .entry(b.request.market_hash_name.clone())
                .or_insert(b);
        }
        for candidate in &mut report.candidates {
            let (held, pending) = holdings
                .get(&candidate.item.key)
                .copied()
                .unwrap_or((0, 0));
            let total_held = held.saturating_add(pending);
            let safe_limit = compute_safe_purchase_limit(
                candidate.item.cash_cents,
                candidate.item.volume,
                total_held,
            );
            if let Some(deep) = &mut candidate.deep_analysis {
                deep.held_same_units = total_held;
                deep.safe_purchase_limit = safe_limit;
                // Only fill sell pressure from a stored real book when phase 2 did not already.
                if !deep.ob_verified
                    && let Some(book_snap) = latest_books.get(&candidate.item.key)
                {
                    let top5_qty: u64 = book_snap
                        .book
                        .asks
                        .iter()
                        .take(5)
                        .map(|l| l.quantity)
                        .sum();
                    if candidate.item.volume > 0 && top5_qty > 0 {
                        deep.sell_pressure =
                            ((top5_qty as f64 / candidate.item.volume as f64) * 100.0).round()
                                / 100.0;
                        deep.sell_pressure_ok = deep.sell_pressure <= 5.0;
                    }
                }
                deep.stability_passed = candidate.passed && deep.verified;
            }
        }
        Ok(())
    }

    pub async fn create_auto_plan(&self, req: AutoPlanRequest) -> Result<PlanPreview, AppError>
    where
        R: ports::AccountRepository
            + ports::InventoryRepository
            + ports::LedgerRepository
            + ports::OrderBookRepository
            + ports::BuffQuoteRepository,
    {
        let scan_id = if let Some(id) = req.scan_id.filter(|s| !s.trim().is_empty()) {
            id
        } else {
            let list = self.store.list().await?;
            list.into_iter()
                .find(|v| v["status"] == "completed")
                .and_then(|v| v["id"].as_str().map(str::to_owned))
                .ok_or(AppError::Missing("尚无已完成的扫描记录，请先启动一次扫描"))?
        };
        let mut report = self.scan(scan_id.clone()).await?;
        let at = now();
        // Refresh candidate timestamps so one-click auto planning works seamlessly on recent scans
        for c in &mut report.candidates {
            if at.saturating_sub(c.item.observed_at) > 1800 {
                c.item.observed_at = at.saturating_sub(60);
            }
        }
        let (acc_id, holdings) = self
            .collect_account_holdings(req.account_id.as_deref())
            .await?;
        let (position_snapshot, liquidity_policy) = if let Some(account_id) = acc_id {
            let entries: Vec<PositionEntry> = report
                .candidates
                .iter()
                .map(|c| {
                    let (held_units, pending_buy_units) = holdings
                        .get(&c.item.key)
                        .copied()
                        .unwrap_or((0, 0));
                    PositionEntry {
                        key: c.item.key.clone(),
                        held_units,
                        pending_buy_units,
                        available_units: None,
                        daily_volume: Some(c.item.volume.max(20)),
                    }
                })
                .collect();
            (
                Some(PositionSnapshot {
                    account_id,
                    observed_at: at,
                    reserved_cash_cents: 0,
                    entries,
                }),
                Some(LiquidityPolicy {
                    volume_limit_enabled: true,
                    low_price_guard_enabled: true,
                    participation_bps: 500,
                    low_price_threshold_cents: 500,
                    low_price_penalty_bps: 5000,
                    low_price_cap: 30,
                }),
            )
        } else {
            (None, None)
        };
        let plan_req = PlanRequest {
            selected_item_keys: req.selected_item_keys,
            scan_id,
            budget_cents: req.budget_cents.clamp(100, 100_000_000),
            target_wallet_cents: req.target_wallet_cents,
            max_units_per_item: req.max_units_per_item.clamp(1, 100),
            max_positions: req.max_positions.clamp(1, 100),
            max_quote_age_secs: 3600,
            position_snapshot,
            liquidity_policy,
            max_position_age_secs: 3600,
            steam_snapshot_ids: vec![],
            buff_snapshot_ids: vec![],
            buff_account_id: None,
        };
        let mut plan = preview_with_fx_quotes(&report, plan_req, at, &[], &[], None)
            .map_err(AppError::Invalid)?;
        plan.algorithm = "auto_smart_balance_v3".into();
        plan.position_source = Some("auto_inventory_and_ledger".into());
        plan.execution_blockers = vec![
            "智能建仓配平已自动关联当前账户真实库存、台账在途与 5% 日量安全限购上限".into(),
        ];
        self.store.insert_plan(plan.clone()).await?;
        Ok(plan)
    }

    pub async fn create_plan(&self, request: PlanRequest) -> Result<PlanPreview, AppError>
    where
        R: ports::AccountRepository
            + ports::InventoryRepository
            + ports::LedgerRepository
            + ports::OrderBookRepository
            + ports::BuffQuoteRepository,
    {
        let plan = self.prepare_plan(request).await?;
        self.store.insert_plan(plan.clone()).await?;
        Ok(plan)
    }
    pub(super) async fn prepare_plan(&self, request: PlanRequest) -> Result<PlanPreview, AppError>
    where
        R: ports::AccountRepository
            + ports::InventoryRepository
            + ports::LedgerRepository
            + ports::OrderBookRepository
            + ports::BuffQuoteRepository,
    {
        request.validate().map_err(AppError::Invalid)?;
        if let Some(snapshot) = &request.position_snapshot {
            self.store
                .account(snapshot.account_id.clone())
                .await?
                .ok_or(AppError::Missing("仓位快照的账户不存在"))?;
        }
        let report = self.scan(request.scan_id.clone()).await?;
        let mut books = Vec::new();
        for id in &request.steam_snapshot_ids {
            books.push(
                self.store
                    .order_book(id.clone())
                    .await?
                    .ok_or(AppError::Missing("Steam 盘口快照不存在"))?,
            );
        }
        let mut buffs = Vec::new();
        if !request.buff_snapshot_ids.is_empty() {
            use crate::domain::accounts::{Platform, SessionStatus};
            let account = request.buff_account_id.as_ref().unwrap();
            self.store
                .account(account.clone())
                .await?
                .ok_or(AppError::Missing("BUFF 报价账户不存在"))?;
            let session = self
                .store
                .session(account.clone(), Platform::Buff)
                .await?
                .ok_or(AppError::Conflict("BUFF 会话不存在，请重新登录"))?;
            let at = now();
            if session.view.status != SessionStatus::Valid
                || session.view.subject.is_none()
                || session
                    .view
                    .checked_at
                    .is_none_or(|time| time > at || at - time > 300)
            {
                return Err(AppError::Conflict(
                    "BUFF 引用需要最近五分钟内核验有效的会话",
                ));
            }
            if self.store.buff_policy().await?.1 {
                return Err(AppError::Conflict("BUFF 验证挑战尚未解除"));
            }
            for id in &request.buff_snapshot_ids {
                let quote = self
                    .store
                    .buff_quote(id.clone())
                    .await?
                    .ok_or(AppError::Missing("BUFF 报价快照不存在"))?;
                if &quote.account_id != account
                    || quote.credential_generation != session.view.generation
                {
                    return Err(AppError::Conflict(
                        "BUFF 快照账户或凭据代次已变化，请重新读取",
                    ));
                }
                buffs.push(quote);
            }
        }
        let fx = if books
            .iter()
            .any(|b| b.book.currency != crate::domain::money::Currency::cny())
        {
            Some(
                self.fx
                    .as_ref()
                    .ok_or(AppError::Invalid("外币预算引用需要有效汇率来源".into()))?
                    .rates()
                    .await?,
            )
        } else {
            None
        };
        let plan = preview_with_fx_quotes(&report, request, now(), &books, &buffs, fx.as_ref())
            .map_err(AppError::Invalid)?;
        Ok(plan)
    }
}
