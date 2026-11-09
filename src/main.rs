use parallax::{
    adapters::{market::Provider, sqlite::Store},
    application::Application,
    transport::http::router,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("PARALLAX_PORT")
        .unwrap_or_else(|_| "38480".into())
        .parse()?;
    // Bind first: if already running, do not touch another process's scan records.
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let data_path =
        PathBuf::from(std::env::var("PARALLAX_DB").unwrap_or_else(|_| "data/parallax.db".into()));
    let store = Store::open(&data_path)?;
    parallax::adapters::network::initialize(&data_path)?;
    let auth = parallax::application::accounts::Accounts::new(
        store.clone(),
        parallax::adapters::credentials::NativeVault,
        parallax::adapters::session::OnlineVerifier::new()?,
    );
    let fx = parallax::application::fx::ExchangeRates::new(
        parallax::adapters::fx::Frankfurter::new()?,
        store.clone(),
    );
    let order_books = parallax::application::orderbook::OrderBookResearch::with_fx(
        parallax::adapters::steam_orderbook::SteamOrderBook::new()?,
        store.clone(),
        fx.clone(),
    );
    let buff_quotes = parallax::application::buff_quotes::BuffResearch::new(
        store.clone(),
        auth.clone(),
        parallax::adapters::buff_quotes::BuffSellOrders::new()?,
    );
    let buff_orders = parallax::application::buff_orders::BuffOrderResearch::new(
        store.clone(),
        auth.clone(),
        parallax::adapters::buff_orders::BuffPurchaseHistory::new()?,
    );
    let inventory = parallax::application::inventory::InventoryResearch::new(
        store.clone(),
        auth.clone(),
        parallax::adapters::steam_inventory::SteamInventory::new()?,
    );
    let histories = parallax::application::history::HistoryResearch::new(
        store.clone(),
        auth.clone(),
        parallax::adapters::steam_history::SteamHistory::new()?,
    );
    let strategies = parallax::application::strategy::StrategyResearch::new(store.clone());
    let preflights = parallax::application::preflight::Preflights::new(store.clone());
    let ledger = parallax::application::ledger::PurchaseLedger::new(store.clone());
    let checkout = parallax::application::checkout::CheckoutPreparation::new(store.clone());
    let sale_research =
        parallax::application::sale_research::SaleResearchService::new(store.clone());
    let holding_research =
        parallax::application::holding_research::HoldingResearchService::new(store.clone());
    let watches = parallax::application::watchlist::WatchService::new(store.clone());
    let guard = parallax::application::steam_guard::SteamGuard::new(
        store.clone(),
        parallax::adapters::credentials::NativeVault,
    );
    let catalog = parallax::application::catalog::Catalog::new(
        parallax::adapters::steam_catalog::SteamCatalog::new()?,
    );
    let live_fx: std::sync::Arc<dyn parallax::application::fx::FxResolver> = fx.clone();
    let deals_service = std::sync::Arc::new(parallax::application::deals::SteamDealsService::new(live_fx.clone()));
    let gift_service = std::sync::Arc::new(parallax::application::gift::SteamGiftService::new(
        std::sync::Arc::new(store.clone()),
        std::sync::Arc::new(parallax::adapters::credentials::NativeVault),
        live_fx.clone(),
    ));
    let data_dir = data_path.parent().unwrap_or(&data_path);
    let notify = parallax::application::notify::NotifyService::new(data_dir);
    let workers = parallax::application::workers::BackgroundWorkers::new(
        auth.clone(),
        guard.clone(),
        ledger.clone(),
        Some(live_fx.clone()),
        notify.clone(),
    );
    let state = Application::with_fx(Provider::new()?, store, fx);
    let browser_logins = parallax::application::browser_login::BrowserLogins::new(
        auth.clone(),
        parallax::adapters::browser_auth::LocalBrowser,
    );
    println!(
        "Parallax http://127.0.0.1:{port} | SQLite: {} | live execution: unavailable",
        data_path.display()
    );
    let routes = router(state);
    #[cfg(windows)]
    let routes = routes.merge(parallax::transport::accounts::steam_password_router(
        auth.clone(),
        guard.clone(),
    ));
    axum::serve(
        listener,
        routes
            .merge(parallax::transport::catalog::router(catalog))
            .merge(parallax::transport::steam_guard::live_market_router(
                auth.clone(),
                guard.clone(),
                Some(live_fx),
            ))
            .merge(parallax::transport::accounts::router(auth))
            .merge(parallax::transport::steam_guard::router(guard))
            .merge(parallax::transport::workers::router(workers, notify))
            .merge(parallax::transport::deals_and_gift::router(deals_service, gift_service))
            .merge(parallax::transport::network::router())
            .merge(parallax::transport::buff_quotes::router(buff_quotes))
            .merge(parallax::transport::buff_orders::router(buff_orders))
            .merge(parallax::transport::inventory::router(inventory))
            .merge(parallax::transport::history::router(histories))
            .merge(parallax::transport::strategy::router(strategies))
            .merge(parallax::transport::holding_research::router(
                holding_research,
            ))
            .merge(parallax::transport::watchlist::router(watches))
            .merge(parallax::transport::preflight::router(preflights))
            .merge(parallax::transport::ledger::router(ledger))
            .merge(parallax::transport::checkout::router(checkout))
            .merge(parallax::transport::sale_research::router(sale_research))
            .merge(parallax::transport::orderbook::router(order_books))
            .merge(parallax::transport::browser_login::router(browser_logins)),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
