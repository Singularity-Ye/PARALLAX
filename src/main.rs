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
    let state = Application::with_fx(Provider::new()?, store, fx);
    let browser_logins = parallax::application::browser_login::BrowserLogins::new(
        auth.clone(),
        parallax::adapters::browser_auth::LocalBrowser,
    );
    println!(
        "Parallax http://127.0.0.1:{port} | SQLite: {} | live execution: unavailable",
        data_path.display()
    );
    axum::serve(
        listener,
        router(state)
            .merge(parallax::transport::accounts::router(auth))
            .merge(parallax::transport::buff_quotes::router(buff_quotes))
            .merge(parallax::transport::buff_orders::router(buff_orders))
            .merge(parallax::transport::inventory::router(inventory))
            .merge(parallax::transport::history::router(histories))
            .merge(parallax::transport::strategy::router(strategies))
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
