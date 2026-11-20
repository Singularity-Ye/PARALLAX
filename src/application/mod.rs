//! Use cases and job ownership; independent of Axum and concrete platform/storage adapters.
#[cfg(test)]
mod account_tests;
pub mod accounts;
pub mod browser_login;
pub mod buff_orders;
pub mod buff_quotes;
pub mod catalog;
pub mod checkout;
pub mod fx;
pub mod history;
pub mod holding_research;
pub mod inventory;
pub mod ledger;
pub mod notify;
pub mod orderbook;
pub mod planning;
pub mod ports;
pub mod preflight;
pub mod replanning;
pub mod sale_research;
pub mod scanner;
pub mod steam_guard;
pub mod strategy;
pub mod watchlist;
pub mod workers;
pub mod assets;
pub mod deals;
pub mod deep_verify;
pub mod gift;

use crate::domain::{
    planning::PlanPreview,
    selection::{ScanReport, ScanRequest},
};
use ports::{MarketSource, Repository};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum AppError {
    Fx(String),
    Invalid(String),
    Missing(&'static str),
    Conflict(&'static str),
    Storage(anyhow::Error),
    Market(ports::MarketReadError),
    Buff(ports::BuffReadError),
    SteamRead(ports::SteamReadError),
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::Storage(value)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fx(s) | Self::Invalid(s) => write!(f, "{s}"),
            Self::Missing(s) | Self::Conflict(s) => write!(f, "{s}"),
            Self::Storage(e) => write!(f, "{e}"),
            Self::Market(e) => write!(f, "{e:?}"),
            Self::Buff(e) => write!(f, "{e:?}"),
            Self::SteamRead(e) => write!(f, "{}", e.message()),
        }
    }
}

impl std::error::Error for AppError {}

pub struct Application<S, R> {
    fx: Option<Arc<dyn fx::FxResolver>>,
    deep_probe: Option<Arc<dyn deep_verify::DeepProbeProvider>>,
    source: S,
    store: R,
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl<S: MarketSource + 'static, R: Repository + 'static> Application<S, R> {
    pub fn new(source: S, store: R) -> Arc<Self> {
        Arc::new(Self {
            fx: None,
            deep_probe: None,
            source,
            store,
            jobs: Mutex::new(HashMap::new()),
        })
    }
    pub fn with_fx(source: S, store: R, fx: Arc<dyn fx::FxResolver>) -> Arc<Self> {
        Arc::new(Self {
            source,
            store,
            fx: Some(fx),
            deep_probe: None,
            jobs: Mutex::new(HashMap::new()),
        })
    }
    pub fn with_fx_and_probe(
        source: S,
        store: R,
        fx: Arc<dyn fx::FxResolver>,
        deep_probe: Arc<dyn deep_verify::DeepProbeProvider>,
    ) -> Arc<Self> {
        Arc::new(Self {
            source,
            store,
            fx: Some(fx),
            deep_probe: Some(deep_probe),
            jobs: Mutex::new(HashMap::new()),
        })
    }
    pub async fn scans(&self) -> Result<Vec<serde_json::Value>, AppError> {
        let jobs = self.jobs.lock().await;
        let mut reports = self.store.list().await?;
        for report in &mut reports {
            if report["status"] == "running"
                && !jobs.contains_key(report["id"].as_str().unwrap_or_default())
            {
                report["status"] = "failed".into();
                report["stop_reason"] = "persistence_error".into();
            }
        }
        Ok(reports)
    }
    pub async fn scan(&self, id: String) -> Result<ScanReport, AppError>
    where
        R: ports::AccountRepository
            + ports::InventoryRepository
            + ports::LedgerRepository
            + ports::OrderBookRepository,
    {
        let jobs = self.jobs.lock().await;
        let mut report = self
            .store
            .get(id.clone())
            .await?
            .ok_or(AppError::Missing("扫描记录不存在"))?;
        if report.status == "running" && !jobs.contains_key(&id) {
            report.status = "failed".into();
            report.stop_reason = "persistence_error".into();
            report.error = Some("任务已退出但结果未成功保存；请检查服务日志".into());
        }
        drop(jobs);
        let _ = self.enrich_scan_report(&mut report).await;
        Ok(report)
    }
    pub async fn start(self: &Arc<Self>, request: ScanRequest) -> Result<ScanReport, AppError>
    where
        R: ports::AccountRepository
            + ports::InventoryRepository
            + ports::LedgerRepository
            + ports::OrderBookRepository,
    {
        request.validate().map_err(AppError::Invalid)?;
        let mut jobs = self.jobs.lock().await;
        if !jobs.is_empty() {
            return Err(AppError::Conflict("已有扫描进行中，请等待或停止当前扫描"));
        }
        let mut report = ScanReport::new(request);
        let src_label = if report.request.source == crate::domain::selection::SourceKind::Demo {
            "演示样本 (Demo)"
        } else {
            "SteamDT 实时榜单"
        };
        let start_msg = format!(
            "启动选品扫描 · 来源={src_label} · 价格 ¥{:.2}–¥{:.2} · 最低日量 {} · 目标折算 ≤ {:.2}%",
            report.request.min_price_cents as f64 / 100.0,
            report.request.max_price_cents as f64 / 100.0,
            report.request.min_volume,
            report.request.max_ratio_bps as f64 / 100.0
        );
        deep_verify::log(&mut report, "info", None, start_msg);
        self.store.save(report.clone()).await?;
        let flag = Arc::new(AtomicBool::new(false));
        jobs.insert(report.id.clone(), flag.clone());
        let pending = report.clone();
        let app = self.clone();
        tokio::spawn(async move {
            let id = pending.id.clone();
            let mut finished = scanner::scan(&app.source, pending, &flag).await;
            let stage1_msg = format!(
                "阶段 1/2 初筛完成：读取 {} 页 / {} 条，去重分析 {} 件，初筛通过 {} 件、淘汰 {} 件。",
                finished.coverage.pages_received,
                finished.coverage.rows_received,
                finished.coverage.examined,
                finished.coverage.accepted,
                finished.coverage.rejected
            );
            deep_verify::log(&mut finished, "info", None, stage1_msg);
            let _ = app.enrich_scan_report(&mut finished).await;
            if finished.status == "completed"
                && finished.request.source == crate::domain::selection::SourceKind::Steamdt
                && finished.request.deep_verify_limit > 0
                && let Some(probe) = &app.deep_probe
            {
                deep_verify::run_deep_verification(&app.store, probe.as_ref(), &mut finished, &flag)
                    .await;
                let _ = app.enrich_scan_report(&mut finished).await;
            } else {
                finished.phase = "done".into();
            }
            if let Err(err) = app.store.save(finished).await {
                eprintln!("Failed to persist scan {id}: {err}");
            }
            app.jobs.lock().await.remove(&id);
        });
        Ok(report)
    }
    pub async fn cancel(&self, id: &str) -> Result<(), AppError> {
        let jobs = self.jobs.lock().await;
        let flag = jobs
            .get(id)
            .ok_or(AppError::Conflict("该扫描已结束或不属于当前进程"))?;
        flag.store(true, Ordering::Relaxed);
        Ok(())
    }
    pub async fn plans(&self) -> Result<Vec<PlanPreview>, AppError> {
        Ok(self.store.list_plans().await?)
    }
    pub async fn plan(&self, id: String) -> Result<PlanPreview, AppError> {
        self.store
            .get_plan(id)
            .await?
            .ok_or(AppError::Missing("计划记录不存在"))
    }
}
