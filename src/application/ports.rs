use crate::domain::{
    planning::PlanPreview,
    selection::{MarketItem, ScanReport, ScanRequest},
};
use std::future::Future;

pub trait MarketSource: Send + Sync {
    fn page(
        &self,
        request: &ScanRequest,
        page: usize,
    ) -> impl Future<Output = Result<Vec<MarketItem>, String>> + Send;
}

pub trait Repository: Send + Sync {
    fn save(&self, report: ScanReport) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn get(&self, id: String) -> impl Future<Output = anyhow::Result<Option<ScanReport>>> + Send;
    fn list(&self) -> impl Future<Output = anyhow::Result<Vec<serde_json::Value>>> + Send;
    fn insert_plan(&self, plan: PlanPreview) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn get_plan(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<PlanPreview>>> + Send;
    fn list_plans(&self) -> impl Future<Output = anyhow::Result<Vec<PlanPreview>>> + Send;
}
