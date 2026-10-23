use crate::domain::accounts::{Account, Credentials, Platform, StoredSession, Verification};
use crate::domain::buff_quotes::{BuffQuoteSnapshot, BuffSellPage};
use crate::domain::orderbook::{OrderBook, OrderBookRequest, OrderBookSnapshot};
use crate::domain::{
    planning::PlanPreview,
    selection::{MarketItem, ScanReport, ScanRequest},
};
use std::future::Future;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryReadError {
    Network,
    InvalidPayload,
    Rejected,
    Expired,
    CredentialsChanged,
    ChangedDuringRead,
    RateLimited(u64),
}
impl InventoryReadError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Network => "Steam 库存读取失败或超时；未保存新快照",
            Self::InvalidPayload => "Steam 库存字段、资产身份或游标无效；未保存新快照",
            Self::Rejected => {
                "Steam 拒绝库存读取，可能涉及隐私或访问限制；未当作空库存或判定登录失效"
            }
            Self::Expired => "Steam 返回未登录，请重新获取并验证登录态",
            Self::CredentialsChanged => "Steam 返回新的认证 Cookie，需通过浏览器重新获取和核验",
            Self::ChangedDuringRead => "分页期间库存总数、资产或游标发生冲突；未保存不一致的新快照",
            Self::RateLimited(_) => "Steam 库存读取处于冷却中，请等待后重试",
        }
    }
}
pub trait InventoryProvider: Send + Sync {
    fn page(
        &self,
        steam_id: &str,
        context: &str,
        cursor: Option<&str>,
        secret: &Credentials,
    ) -> impl Future<Output = Result<crate::domain::inventory::InventoryPage, InventoryReadError>> + Send;
}
pub trait InventoryRepository: Send + Sync {
    fn insert_inventory(
        &self,
        snapshot: crate::domain::inventory::InventorySnapshot,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn inventory(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::inventory::InventorySnapshot>>> + Send;
    fn inventories(
        &self,
        account_id: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::inventory::InventorySnapshot>>> + Send;
    fn inventory_retry(&self) -> impl Future<Output = anyhow::Result<u64>> + Send;
    fn set_inventory_retry(&self, at: u64) -> impl Future<Output = anyhow::Result<()>> + Send;
}

#[derive(Clone, Copy, Debug)]
pub enum BuffReadError {
    Network,
    InvalidPayload,
    IdentityMismatch,
    Expired,
    Challenge,
    RateLimited(u64),
    Rejected,
    CredentialsChanged,
}
impl BuffReadError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Network => "BUFF 卖单读取失败或超时；未生成报价",
            Self::InvalidPayload => "BUFF 卖单格式或金额无效；未生成报价",
            Self::IdentityMismatch => "BUFF 商品身份与扫描候选不匹配",
            Self::Expired => "BUFF 登录态已失效，请重新登录",
            Self::Challenge => "BUFF 要求浏览器验证，已阻断后续请求",
            Self::RateLimited(_) => "BUFF 请求处于冷却中，请等待后重试",
            Self::Rejected => "BUFF 拒绝卖单请求；未将失败当作空卖盘",
            Self::CredentialsChanged => {
                "BUFF 返回新的认证 Cookie；当前凭据需通过浏览器重新获取和核验"
            }
        }
    }
}
pub trait BuffQuoteProvider: Send + Sync {
    fn sell_page(
        &self,
        goods_id: u64,
        secret: Credentials,
    ) -> impl Future<Output = Result<BuffSellPage, BuffReadError>> + Send;
}
pub trait BuffQuoteRepository: Send + Sync {
    fn insert_buff_quote(
        &self,
        snapshot: BuffQuoteSnapshot,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn buff_quote(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<BuffQuoteSnapshot>>> + Send;
    fn buff_quotes(
        &self,
        account_id: String,
    ) -> impl Future<Output = anyhow::Result<Vec<BuffQuoteSnapshot>>> + Send;
}

#[derive(Clone, Copy, Debug)]
pub enum MarketReadError {
    Network,
    Rejected,
    InvalidPayload,
    UnsupportedCurrency,
    RateLimited(u64),
}
impl MarketReadError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Network => "Steam 盘口网络请求失败或超时",
            Self::Rejected => "Steam 拒绝公开盘口请求，未将失败当作空盘口",
            Self::InvalidPayload => "Steam 盘口响应格式或金额无效，未生成估值",
            Self::UnsupportedCurrency => "Steam 返回暂不支持的币种，未猜测或换汇",
            Self::RateLimited(_) => "Steam 盘口读取处于冷却中，请等待后重试",
        }
    }
}
pub trait OrderBookProvider: Send + Sync {
    fn order_book(
        &self,
        request: &OrderBookRequest,
    ) -> impl Future<Output = Result<OrderBook, MarketReadError>> + Send;
}
pub trait OrderBookRepository: Send + Sync {
    fn insert_order_book(
        &self,
        snapshot: OrderBookSnapshot,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn order_book(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<OrderBookSnapshot>>> + Send;
    fn order_books(&self) -> impl Future<Output = anyhow::Result<Vec<OrderBookSnapshot>>> + Send;
}

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

/// Owns only newly launched, isolated browser windows; never attaches to a user's
/// existing browser/profile. All returned errors must be safe to show publicly.
pub trait BrowserAuth: Send + Sync {
    type Window: Send + Sync;
    fn launch(
        &self,
        platform: Platform,
    ) -> impl Future<Output = Result<Self::Window, &'static str>> + Send;
    fn capture(
        &self,
        window: &Self::Window,
        platform: Platform,
    ) -> impl Future<Output = Result<Credentials, &'static str>> + Send;
    fn close(&self, window: Self::Window) -> impl Future<Output = Result<(), &'static str>> + Send;
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
