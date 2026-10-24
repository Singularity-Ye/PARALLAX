use crate::domain::selection::{ScanReport, now};
use crate::{application::ports::Repository, domain::planning::PlanPreview};
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
mod accounts;
mod buff_quotes;
mod fx;
mod history;
mod inventory;
mod ledger;
mod orderbook;
mod preflight;
mod sale_research;
mod steam_policy;
mod strategy;

#[derive(Clone)]
pub struct Store(Arc<Mutex<Connection>>);
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS scans (id TEXT PRIMARY KEY, created_at INTEGER NOT NULL, report TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS plan_previews (id TEXT PRIMARY KEY, created_at INTEGER NOT NULL, report TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS accounts (id TEXT PRIMARY KEY, steam_id TEXT NOT NULL, metadata TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS account_sessions (account_id TEXT NOT NULL, platform TEXT NOT NULL, metadata TEXT NOT NULL, credential_key TEXT NOT NULL, PRIMARY KEY(account_id,platform));
            CREATE TABLE IF NOT EXISTS auth_policy (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS steam_order_books (id TEXT PRIMARY KEY, observed_at INTEGER NOT NULL, snapshot TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS buff_quote_snapshots (id TEXT PRIMARY KEY, account_id TEXT NOT NULL, observed_at INTEGER NOT NULL, snapshot TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS inventory_snapshots (id TEXT PRIMARY KEY, account_id TEXT NOT NULL, observed_at INTEGER NOT NULL, snapshot TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS price_history_snapshots (id TEXT PRIMARY KEY, account_id TEXT NOT NULL, observed_at INTEGER NOT NULL, snapshot TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS exchange_rate_snapshots (id TEXT PRIMARY KEY, fetched_at INTEGER NOT NULL, snapshot TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS strategy_versions (strategy_id TEXT NOT NULL, revision INTEGER NOT NULL, created_at INTEGER NOT NULL, snapshot TEXT NOT NULL, PRIMARY KEY(strategy_id, revision));
            CREATE TABLE IF NOT EXISTS strategy_evaluations (id TEXT PRIMARY KEY, created_at INTEGER NOT NULL, snapshot TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS preflight_inspections (id TEXT PRIMARY KEY, created_at INTEGER NOT NULL, snapshot TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS purchase_lots (id TEXT PRIMARY KEY, account_id TEXT NOT NULL, client_key TEXT NOT NULL, purchase_ref TEXT NOT NULL, snapshot TEXT NOT NULL, UNIQUE(account_id,client_key), UNIQUE(account_id,purchase_ref));
            CREATE TABLE IF NOT EXISTS purchase_lot_events (id TEXT PRIMARY KEY, account_id TEXT NOT NULL, lot_id TEXT NOT NULL, client_key TEXT NOT NULL, revision INTEGER NOT NULL, snapshot TEXT NOT NULL, UNIQUE(account_id,client_key), UNIQUE(lot_id,revision));
            CREATE TABLE IF NOT EXISTS purchase_asset_claims (account_id TEXT NOT NULL, asset_id TEXT NOT NULL, lot_id TEXT NOT NULL, PRIMARY KEY(account_id,asset_id));
            CREATE TABLE IF NOT EXISTS purchase_external_claims (account_id TEXT NOT NULL, kind TEXT NOT NULL, reference TEXT NOT NULL, lot_id TEXT NOT NULL, PRIMARY KEY(account_id,kind,reference));
            CREATE TABLE IF NOT EXISTS sale_research (id TEXT PRIMARY KEY, account_id TEXT NOT NULL, created_at INTEGER NOT NULL, snapshot TEXT NOT NULL);")?;
        // A restarted process cannot continue an old in-memory scan. Preserve evidence honestly.
        let reports = {
            let mut stmt = db.prepare("SELECT report FROM scans")?;
            stmt.query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        for raw in reports {
            let mut report: ScanReport = serde_json::from_str(&raw)?;
            if report.status == "running" {
                report.status = "interrupted".into();
                report.stop_reason = "process_restart".into();
                report.finished_at = Some(now());
                db.execute(
                    "UPDATE scans SET report=?1 WHERE id=?2",
                    params![serde_json::to_string(&report)?, report.id],
                )?;
            }
        }
        Ok(Self(Arc::new(Mutex::new(db))))
    }
}
impl Repository for Store {
    async fn save(&self, report: ScanReport) -> Result<()> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            db.execute(
                "INSERT INTO scans(id,created_at,report) VALUES(?1,?2,?3)
                ON CONFLICT(id) DO UPDATE SET report=excluded.report",
                params![
                    report.id,
                    i64::try_from(report.created_at)?,
                    serde_json::to_string(&report)?
                ],
            )?;
            Ok(())
        })
        .await?
    }
    async fn get(&self, id: String) -> Result<Option<ScanReport>> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<ScanReport>> {
            let db = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            let raw: Option<String> = db
                .query_row("SELECT report FROM scans WHERE id=?1", [id], |r| r.get(0))
                .optional()?;
            raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
                .transpose()
        })
        .await?
    }
    async fn list(&self) -> Result<Vec<serde_json::Value>> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<serde_json::Value>> {
            let db = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            let mut stmt = db.prepare(
                "SELECT report FROM scans ORDER BY created_at DESC, rowid DESC LIMIT 30",
            )?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.map(|r| {
                let mut value: serde_json::Value = serde_json::from_str(&r?)?;
                value
                    .as_object_mut()
                    .ok_or_else(|| anyhow::anyhow!("invalid stored report"))?
                    .remove("candidates");
                Ok(value)
            })
            .collect()
        })
        .await?
    }
    async fn insert_plan(&self, plan: PlanPreview) -> Result<()> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            // Immutable snapshot: an existing plan ID must never be overwritten.
            db.execute(
                "INSERT INTO plan_previews(id,created_at,report) VALUES(?1,?2,?3)",
                params![
                    plan.id,
                    i64::try_from(plan.created_at)?,
                    serde_json::to_string(&plan)?
                ],
            )?;
            Ok(())
        })
        .await?
    }
    async fn get_plan(&self, id: String) -> Result<Option<PlanPreview>> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || -> Result<Option<PlanPreview>> {
            let db = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            let raw: Option<String> = db
                .query_row("SELECT report FROM plan_previews WHERE id=?1", [id], |r| {
                    r.get(0)
                })
                .optional()?;
            raw.map(|s| serde_json::from_str(&s).map_err(Into::into))
                .transpose()
        })
        .await?
    }
    async fn list_plans(&self) -> Result<Vec<PlanPreview>> {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<PlanPreview>> {
            let db = db
                .lock()
                .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
            let mut stmt = db.prepare(
                "SELECT report FROM plan_previews ORDER BY created_at DESC, rowid DESC LIMIT 30",
            )?;
            stmt.query_map([], |r| r.get::<_, String>(0))?
                .map(|r| Ok(serde_json::from_str(&r?)?))
                .collect()
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::selection::ScanRequest;
    #[tokio::test]
    async fn restart_marks_running_scan_interrupted() {
        let path = std::env::temp_dir().join(format!("parallax-test-{}.db", uuid::Uuid::new_v4()));
        let report = ScanReport::new(ScanRequest::default());
        let store = Store::open(&path).unwrap();
        store.save(report.clone()).await.unwrap();
        drop(store);
        let reopened = Store::open(&path).unwrap();
        let saved = reopened.get(report.id).await.unwrap().unwrap();
        assert_eq!(saved.status, "interrupted");
        assert_eq!(saved.stop_reason, "process_restart");
        drop(reopened);
        std::fs::remove_file(path).unwrap();
    }
}
