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
    let state = Application::new(Provider::new()?, Store::open(&data_path)?);
    println!(
        "异观 · Parallax http://127.0.0.1:{port} | SQLite: {} | live execution: unavailable",
        data_path.display()
    );
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
