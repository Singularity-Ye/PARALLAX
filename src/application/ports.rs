use crate::domain::accounts::{Account, Credentials, Platform, StoredSession, Verification};
use crate::domain::buff_quotes::{BuffQuoteSnapshot, BuffSellPage};
use crate::domain::orderbook::{OrderBook, OrderBookRequest, OrderBookSnapshot};
use crate::domain::{
    planning::PlanPreview,
    selection::{MarketItem, ScanReport, ScanRequest},
};
use std::future::Future;

pub trait FxProvider: Send + Sync {
    fn fetch_rates(
        &self,
        at: u64,
    ) -> impl Future<Output = Result<crate::domain::fx::FxSnapshot, String>> + Send;
}
pub trait FxRepository: Send + Sync {
    fn insert_fx(
        &self,
        snapshot: crate::domain::fx::FxSnapshot,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn latest_fx(
        &self,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::fx::FxSnapshot>>> + Send;
}

#[derive(Clone, Copy)]
pub enum SteamReadScope {
    Inventory,
    PriceHistory,
}
impl SteamReadScope {
    pub fn key(self) -> &'static str {
        match self {
            Self::Inventory => "steam_inventory",
            Self::PriceHistory => "steam_price_history",
        }
    }
}
pub trait SteamReadPolicy: Send + Sync {
    fn steam_read_retry(
        &self,
        scope: SteamReadScope,
    ) -> impl Future<Output = anyhow::Result<u64>> + Send;
    fn set_steam_read_retry(
        &self,
        scope: SteamReadScope,
        at: u64,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}
pub trait HistoryProvider: Send + Sync {
    fn history(
        &self,
        request: &crate::domain::history::HistoryRequest,
        secret: &Credentials,
        at: u64,
    ) -> impl Future<Output = Result<crate::domain::history::HistorySeries, SteamReadError>> + Send;
}
pub trait HistoryRepository: SteamReadPolicy {
    fn insert_history(
        &self,
        snapshot: crate::domain::history::HistorySnapshot,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn history(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::history::HistorySnapshot>>> + Send;
    fn histories(
        &self,
        account: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::history::HistorySnapshot>>> + Send;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SteamReadError {
    Network,
    InvalidPayload,
    Rejected,
    Expired,
    CredentialsChanged,
    ChangedDuringRead,
    RateLimited(u64),
}
impl SteamReadError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Network => "Steam 只读数据获取失败或超时；未保存新快照",
            Self::InvalidPayload => "Steam 只读响应字段、身份或格式无效；未保存新快照",
            Self::Rejected => {
                "Steam 拒绝只读请求，可能涉及隐私或访问限制；未当作空数据或判定登录失效"
            }
            Self::Expired => "Steam 返回未登录，请重新获取并验证登录态",
            Self::CredentialsChanged => "Steam 返回新的认证 Cookie，需通过浏览器重新获取和核验",
            Self::ChangedDuringRead => "分页期间库存总数、资产或游标发生冲突；未保存不一致的新快照",
            Self::RateLimited(_) => "Steam 只读数据获取处于冷却中，请等待后重试",
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
    ) -> impl Future<Output = Result<crate::domain::inventory::InventoryPage, SteamReadError>> + Send;
}
pub trait InventoryRepository: SteamReadPolicy {
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
            Self::Network => "BUFF 只读请求失败或超时；未保存新观察",
            Self::InvalidPayload => "BUFF 只读响应格式、范围或金额无效；未保存新观察",
            Self::IdentityMismatch => "BUFF 响应身份与当前请求不匹配",
            Self::Expired => "BUFF 登录态已失效，请重新登录",
            Self::Challenge => "BUFF 要求浏览器验证，已阻断后续请求",
            Self::RateLimited(_) => "BUFF 请求处于冷却中，请等待后重试",
            Self::Rejected => "BUFF 拒绝只读请求；未将失败当作空结果",
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

pub enum BuffReadPermit {
    Allowed,
    Blocked,
    CoolingUntil(u64),
}
pub trait AccountRepository: Send + Sync {
    // Atomically reserve pacing; a read cannot clear a challenge set by another connection.
    fn try_buff_read(&self, at: u64)
    -> impl Future<Output = anyhow::Result<BuffReadPermit>> + Send;

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

pub trait StrategyRepository: Send + Sync {
    /// Atomic compare-and-append: false means the expected revision is no longer current.
    fn append_strategy(
        &self,
        version: crate::domain::strategy::StrategyVersion,
        expected_revision: u32,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;
    fn strategy_version(
        &self,
        id: String,
        revision: u32,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::strategy::StrategyVersion>>> + Send;
    fn strategy_heads(
        &self,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::strategy::StrategyVersion>>> + Send;
    fn strategy_versions(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::strategy::StrategyVersion>>> + Send;
    fn insert_evaluation(
        &self,
        evaluation: crate::domain::strategy::StrategyEvaluation,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn evaluation(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::strategy::StrategyEvaluation>>> + Send;
    fn evaluations(
        &self,
    ) -> impl Future<
        Output = anyhow::Result<Vec<crate::domain::strategy::StrategyEvaluationSummary>>,
    > + Send;
}

pub trait PreflightRepository: Send + Sync {
    /// Read account/session metadata and BUFF policy in a single read transaction.
    fn preflight_context(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::preflight::AccountContext>>> + Send;
    fn insert_preflight(
        &self,
        inspection: crate::domain::preflight::Preflight,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn preflight(
        &self,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::preflight::Preflight>>> + Send;
    fn preflights(
        &self,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::preflight::PreflightSummary>>> + Send;
}

pub trait LedgerRepository: Send + Sync {
    fn create_lot(
        &self,
        account: String,
        request: crate::domain::ledger::LotRequest,
        at: u64,
    ) -> impl Future<Output = anyhow::Result<crate::domain::ledger::LedgerOutcome>> + Send;
    fn append_lot_event(
        &self,
        account: String,
        id: String,
        request: crate::domain::ledger::LotEventRequest,
        inventory: Option<crate::domain::inventory::InventorySnapshot>,
        at: u64,
    ) -> impl Future<Output = anyhow::Result<crate::domain::ledger::LedgerOutcome>> + Send;
    fn lot(
        &self,
        account: String,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::ledger::PurchaseLot>>> + Send;
    fn lots(
        &self,
        account: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::ledger::PurchaseLot>>> + Send;
    fn lot_events(
        &self,
        account: String,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::ledger::LotEvent>>> + Send;
}
pub trait SaleResearchRepository: Send + Sync {
    fn insert_sale_research(
        &self,
        value: crate::domain::sale_research::SaleResearch,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn sale_research(
        &self,
        account: String,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::sale_research::SaleResearch>>> + Send;
    fn sale_researches(
        &self,
        account: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::sale_research::SaleResearchSummary>>>
    + Send;
}

pub trait CheckoutRepository: Send + Sync {
    fn observe_funding(
        &self,
        account: String,
        request: crate::domain::funding::FundingRequest,
        at: u64,
    ) -> impl Future<
        Output = anyhow::Result<
            crate::domain::checkout::CheckoutOutcome<crate::domain::funding::FundingSnapshot>,
        >,
    > + Send;
    fn funding_view(
        &self,
        account: String,
        at: u64,
    ) -> impl Future<
        Output = anyhow::Result<
            crate::domain::checkout::CheckoutOutcome<crate::domain::funding::FundingView>,
        >,
    > + Send;
    fn prepare_intent(
        &self,
        account: String,
        request: crate::domain::checkout::PrepareRequest,
        at: u64,
    ) -> impl Future<
        Output = anyhow::Result<
            crate::domain::checkout::CheckoutOutcome<crate::domain::checkout::PurchaseIntent>,
        >,
    > + Send;
    fn cancel_intent(
        &self,
        account: String,
        id: String,
        request: crate::domain::checkout::CancelRequest,
        at: u64,
    ) -> impl Future<
        Output = anyhow::Result<
            crate::domain::checkout::CheckoutOutcome<crate::domain::checkout::PurchaseIntent>,
        >,
    > + Send;
    fn purchase_intent(
        &self,
        account: String,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::checkout::PurchaseIntent>>> + Send;
    fn purchase_intents(
        &self,
        account: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::checkout::IntentSummary>>> + Send;
    fn intent_events(
        &self,
        account: String,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::checkout::IntentEvent>>> + Send;
}

pub trait BuffOrderProvider: Send + Sync {
    fn history_page(
        &self,
        page: u32,
        size: u32,
        principal: &str,
        secret: &Credentials,
    ) -> impl Future<Output = Result<crate::domain::buff_orders::OrderHistoryPage, BuffReadError>> + Send;
}
pub trait BuffOrderRepository: Send + Sync {
    fn insert_buff_orders(
        &self,
        snapshot: crate::domain::buff_orders::OrderHistorySnapshot,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;
    fn buff_orders(
        &self,
        id: String,
    ) -> impl Future<
        Output = anyhow::Result<Option<crate::domain::buff_orders::OrderHistorySnapshot>>,
    > + Send;
    fn buff_order_observations(
        &self,
        account: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::buff_orders::OrderHistorySnapshot>>> + Send;
}

pub trait HoldingResearchRepository: Send + Sync {
    fn insert_holding_research(
        &self,
        report: crate::domain::holding_research::HoldingResearchReport,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn holding_research(
        &self,
        id: String,
    ) -> impl Future<
        Output = anyhow::Result<Option<crate::domain::holding_research::HoldingResearchReport>>,
    > + Send;
    fn holding_researches(
        &self,
        account: String,
    ) -> impl Future<
        Output = anyhow::Result<Vec<crate::domain::holding_research::HoldingResearchSummary>>,
    > + Send;
}

pub trait WatchRepository: Send + Sync {
    fn save_watch(
        &self,
        item: crate::domain::watchlist::WatchItem,
    ) -> impl Future<Output = anyhow::Result<crate::domain::watchlist::WatchItem>> + Send;
    fn watches(
        &self,
        account: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::watchlist::WatchItem>>> + Send;
    fn watch(
        &self,
        account: String,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Option<crate::domain::watchlist::WatchItem>>> + Send;
    fn save_observation(
        &self,
        item: crate::domain::watchlist::WatchObservation,
    ) -> impl Future<Output = anyhow::Result<crate::domain::watchlist::WatchObservation>> + Send;
    fn observations(
        &self,
        account: String,
        id: String,
    ) -> impl Future<Output = anyhow::Result<Vec<crate::domain::watchlist::WatchObservation>>> + Send;
}
