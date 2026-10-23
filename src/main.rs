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
    let state = Application::new(Provider::new()?, store);
    println!(
        "Parallax http://127.0.0.1:{port} | SQLite: {} | live execution: unavailable",
        data_path.display()
    );
    axum::serve(
        listener,
        router(state).merge(parallax::transport::accounts::router(auth)),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
