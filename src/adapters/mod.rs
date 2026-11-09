//! External systems implement the ports owned by the application.
pub mod browser_auth;
pub mod buff_orders;
pub mod buff_quotes;
pub mod credentials;
mod credentials_guard;
pub mod fx;
pub mod market;
pub mod network;
pub mod session;
pub mod sqlite;
pub mod steam_history;
pub mod steam_inventory;
pub mod steam_orderbook;
mod steam_read;

mod buff_read;

pub mod steam_catalog;
pub mod steam_market;

#[cfg(windows)]
pub mod steam_password;
