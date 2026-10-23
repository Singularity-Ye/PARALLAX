use crate::domain::accounts::{Account, Credentials, Platform, StoredSession, Verification};
use crate::domain::{
    planning::PlanPreview,
    selection::{MarketItem, ScanReport, ScanRequest},
};
use std::future::Future;

pub trait AccountRepository: Send + Sync {
    fn add_account(&self, account: Account) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn accounts(&self) -> impl Future<Output = anyhow::Result<Vec<Account>>> + Send;
    fn account(&self, id: String) -> impl Future<Output = anyhow::Result<Option<Account>>> + Send;
    fn session(
        &self,
        id: String,
        platform: Platform,
    ) -> impl Future<Output = anyhow::Result<Option<StoredSession>>> + Send;
    fn save_session(
        &self,
        id: String,
        session: StoredSession,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn sessions(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Vec<StoredSession>>> + Send;
    fn buff_policy(&self) -> impl Future<Output = anyhow::Result<(u64, bool)>> + Send;
    fn set_buff_policy(
        &self,
        at: u64,
        blocked: bool,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}
pub trait CredentialStore: Send + Sync {
    fn put(
        &self,
        key: String,
        secret: Credentials,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn get(&self, key: String) -> impl Future<Output = anyhow::Result<Option<Credentials>>> + Send;
    fn remove(&self, key: String) -> impl Future<Output = anyhow::Result<()>> + Send;
}
pub trait SessionVerifier: Send + Sync {
    fn verify(
        &self,
        account: &Account,
        platform: Platform,
        secret: Credentials,
    ) -> impl Future<Output = Verification> + Send;
}

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
