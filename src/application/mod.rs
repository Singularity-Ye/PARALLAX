//! Use cases and job ownership; independent of Axum and concrete platform/storage adapters.
pub mod ports;
pub mod scanner;

use crate::domain::{
    planning::{PlanPreview, PlanRequest, preview},
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
    Invalid(String),
    Missing(&'static str),
    Conflict(&'static str),
    Storage(anyhow::Error),
}
impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::Storage(value)
    }
}

pub struct Application<S, R> {
    source: S,
    store: R,
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}
impl<S: MarketSource + 'static, R: Repository + 'static> Application<S, R> {
    pub fn new(source: S, store: R) -> Arc<Self> {
        Arc::new(Self {
            source,
            store,
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
    pub async fn create_plan(&self, request: PlanRequest) -> Result<PlanPreview, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let report = self.scan(request.scan_id.clone()).await?;
        let plan = preview(&report, request, now()).map_err(AppError::Invalid)?;
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
