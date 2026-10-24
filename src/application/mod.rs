//! Use cases and job ownership; independent of Axum and concrete platform/storage adapters.
#[cfg(test)]
mod account_tests;
pub mod accounts;
pub mod browser_login;
pub mod buff_quotes;
pub mod fx;
pub mod history;
pub mod inventory;
pub mod orderbook;
pub mod ports;
pub mod scanner;
pub mod strategy;

use crate::domain::{
    planning::{PlanPreview, PlanRequest, preview_with_fx_quotes},
    selection::{ScanReport, ScanRequest, now},
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

pub struct Application<S, R> {
    fx: Option<Arc<dyn fx::FxResolver>>,
    source: S,
    store: R,
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}
impl<S: MarketSource + 'static, R: Repository + 'static> Application<S, R> {
    pub fn new(source: S, store: R) -> Arc<Self> {
        Arc::new(Self {
            fx: None,
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
    pub async fn scan(&self, id: String) -> Result<ScanReport, AppError> {
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
        Ok(report)
    }
    pub async fn start(self: &Arc<Self>, request: ScanRequest) -> Result<ScanReport, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let mut jobs = self.jobs.lock().await;
        if !jobs.is_empty() {
            return Err(AppError::Conflict("已有扫描进行中，请等待或停止当前扫描"));
        }
        let report = ScanReport::new(request);
        self.store.save(report.clone()).await?;
        let flag = Arc::new(AtomicBool::new(false));
        jobs.insert(report.id.clone(), flag.clone());
        let pending = report.clone();
        let app = self.clone();
        tokio::spawn(async move {
            let id = pending.id.clone();
            let finished = scanner::scan(&app.source, pending, &flag).await;
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
    pub async fn create_plan(&self, request: PlanRequest) -> Result<PlanPreview, AppError>
    where
        R: ports::AccountRepository + ports::OrderBookRepository + ports::BuffQuoteRepository,
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
        self.store.insert_plan(plan.clone()).await?;
        Ok(plan)
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
