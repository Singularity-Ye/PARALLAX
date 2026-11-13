use super::Store;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::path::Path;

pub const DEFAULT_KEEP_SCANS: usize = 1;
pub const DEFAULT_KEEP_SNAPSHOTS_PER_ACCOUNT: usize = 1;

#[derive(Clone, Debug)]
pub struct IconRecord {
    pub item_key: String,
    pub source_url: String,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub byte_size: u64,
    pub updated_at: u64,
    pub last_used_at: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct IconCacheStats {
    pub indexed_count: u64,
    pub cached_files: u64,
    pub cached_bytes: u64,
    pub max_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LogCacheStats {
    pub scans_total: u64,
    pub scans_cleanable: u64,
    pub snapshots_total: u64,
    pub snapshots_cleanable: u64,
    pub db_bytes: u64,
    pub wal_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct MaintenanceStats {
    pub icons: IconCacheStats,
    pub logs: LogCacheStats,
}

#[derive(Clone, Debug, Serialize)]
pub struct CleanupReport {
    pub cleared_icon_files: u64,
    pub freed_icon_bytes: u64,
    pub deleted_scans: u64,
    pub deleted_snapshots: u64,
    pub reclaimed_db_bytes: u64,
    pub stats: MaintenanceStats,
}

struct SnapshotSpec {
    table: &'static str,
    time_col: &'static str,
    per_account: bool,
    min_keep: usize,
}

const SNAPSHOT_SPECS: &[SnapshotSpec] = &[
    SnapshotSpec {
        table: "inventory_snapshots",
        time_col: "observed_at",
        per_account: true,
        min_keep: DEFAULT_KEEP_SNAPSHOTS_PER_ACCOUNT,
    },
    SnapshotSpec {
        table: "buff_quote_snapshots",
        time_col: "observed_at",
        per_account: true,
        min_keep: DEFAULT_KEEP_SNAPSHOTS_PER_ACCOUNT,
    },
    SnapshotSpec {
        table: "buff_order_observations",
        time_col: "observed_at",
        per_account: true,
        min_keep: DEFAULT_KEEP_SNAPSHOTS_PER_ACCOUNT,
    },
    SnapshotSpec {
        table: "price_history_snapshots",
        time_col: "observed_at",
        per_account: true,
        min_keep: DEFAULT_KEEP_SNAPSHOTS_PER_ACCOUNT,
    },
    SnapshotSpec {
        table: "steam_order_books",
        time_col: "observed_at",
        per_account: false,
        min_keep: 3,
    },
    SnapshotSpec {
        table: "exchange_rate_snapshots",
        time_col: "fetched_at",
        per_account: false,
        min_keep: 1,
    },
    SnapshotSpec {
        table: "plan_previews",
        time_col: "created_at",
        per_account: false,
        min_keep: 3,
    },
];

fn snapshot_where_cleanable(spec: &SnapshotSpec) -> String {
    if spec.per_account {
        format!(
            "id NOT IN (
                SELECT s2.id FROM {t} s2
                WHERE s2.account_id = {t}.account_id
                ORDER BY s2.{c} DESC, s2.rowid DESC
                LIMIT ?1
            )",
            t = spec.table,
            c = spec.time_col
        )
    } else {
        format!(
            "id NOT IN (
                SELECT id FROM {t}
                ORDER BY {c} DESC, rowid DESC
                LIMIT ?1
            )",
            t = spec.table,
            c = spec.time_col
        )
    }
}

fn count_table(db: &Connection, sql: &str, limit: i64) -> anyhow::Result<u64> {
    let n: i64 = db.query_row(sql, [limit], |r| r.get(0))?;
    Ok(n.max(0) as u64)
}

fn dir_cache_usage(icons_dir: &Path) -> (u64, u64) {
    let Ok(entries) = std::fs::read_dir(icons_dir) else {
        return (0, 0);
    };
    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_file() && !path.extension().is_some_and(|e| e == "tmp") {
            files += 1;
            bytes = bytes.saturating_add(meta.len());
        }
    }
    (files, bytes)
}

fn db_and_wal_bytes(db_path: &Path) -> (u64, u64) {
    if db_path.as_os_str() == ":memory:" {
        return (0, 0);
    }
    let db_bytes = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);
    let mut wal_os = db_path.as_os_str().to_os_string();
    wal_os.push("-wal");
    let wal_bytes = std::fs::metadata(Path::new(&wal_os))
        .map(|m| m.len())
        .unwrap_or(0);
    (db_bytes, wal_bytes)
}

fn collect_stats_sync(
    db: &Connection,
    icons_dir: &Path,
    db_path: &Path,
    max_icon_bytes: u64,
    keep_scans: usize,
) -> anyhow::Result<MaintenanceStats> {
    let indexed_count = count_table(db, "SELECT COUNT(*) FROM item_icons", 0)
        .or_else(|_| db.query_row("SELECT COUNT(*) FROM item_icons", [], |r| r.get::<_, i64>(0)).map(|v| v.max(0) as u64))?;
    let (cached_files, cached_bytes) = dir_cache_usage(icons_dir);

    let keep_scans_i64 = i64::try_from(keep_scans).unwrap_or(1);
    let scans_total: u64 = db
        .query_row("SELECT COUNT(*) FROM scans", [], |r| r.get::<_, i64>(0))?
        .max(0) as u64;
    let scans_cleanable = count_table(
        db,
        "SELECT COUNT(*) FROM scans
         WHERE COALESCE(json_extract(report, '$.status'), '') != 'running'
           AND id NOT IN (
             SELECT id FROM scans
             WHERE COALESCE(json_extract(report, '$.status'), '') != 'running'
             ORDER BY created_at DESC, rowid DESC
             LIMIT ?1
           )",
        keep_scans_i64,
    )?;

    let mut snapshots_total = 0u64;
    let mut snapshots_cleanable = 0u64;
    for spec in SNAPSHOT_SPECS {
        let total_sql = format!("SELECT COUNT(*) FROM {}", spec.table);
        snapshots_total += db
            .query_row(&total_sql, [], |r| r.get::<_, i64>(0))?
            .max(0) as u64;
        let clean_sql = format!(
            "SELECT COUNT(*) FROM {} WHERE {}",
            spec.table,
            snapshot_where_cleanable(spec)
        );
        let keep = i64::try_from(spec.min_keep).unwrap_or(1);
        snapshots_cleanable += count_table(db, &clean_sql, keep)?;
    }

    let (db_bytes, wal_bytes) = db_and_wal_bytes(db_path);
    Ok(MaintenanceStats {
        icons: IconCacheStats {
            indexed_count,
            cached_files,
            cached_bytes,
            max_bytes: max_icon_bytes,
        },
        logs: LogCacheStats {
            scans_total,
            scans_cleanable,
            snapshots_total,
            snapshots_cleanable,
            db_bytes,
            wal_bytes,
        },
    })
}

pub(crate) fn upsert_icons_conn(
    conn: &Connection,
    entries: &[(&str, &str)],
    at: u64,
) -> anyhow::Result<usize> {
    if entries.is_empty() {
        return Ok(0);
    }
    let mut stmt = conn.prepare(
        "INSERT INTO item_icons(item_key, source_url, updated_at, last_used_at)
         VALUES(?1, ?2, ?3, ?3)
         ON CONFLICT(item_key) DO UPDATE SET
           source_url = excluded.source_url,
           updated_at = excluded.updated_at",
    )?;
    let at_i64 = i64::try_from(at)?;
    let mut count = 0usize;
    for (raw_key, raw_url) in entries {
        let key = raw_key.trim();
        let url = raw_url.trim();
        if key.is_empty() || url.is_empty() || key.len() > 500 || url.len() > 1000 {
            continue;
        }
        stmt.execute(params![key, url, at_i64])?;
        count += 1;
    }
    Ok(count)
}

impl Store {
    /// Records discovered external icon URLs for CS2 items without touching cached files.
    pub async fn record_item_icons(
        &self,
        entries: Vec<(String, String)>,
        at: u64,
    ) -> anyhow::Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }
        self.account_db(move |db| {
            let tx = db.unchecked_transaction()?;
            let pairs: Vec<(&str, &str)> = entries
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let count = upsert_icons_conn(&tx, &pairs, at)?;
            tx.commit()?;
            Ok(count)
        })
        .await
    }

    pub async fn lookup_item_icon(&self, key: String) -> anyhow::Result<Option<IconRecord>> {
        self.account_db(move |db| {
            db.query_row(
                "SELECT item_key, source_url, file_name, content_type, byte_size, updated_at, last_used_at
                 FROM item_icons WHERE item_key = ?1",
                [key],
                |r| {
                    Ok(IconRecord {
                        item_key: r.get(0)?,
                        source_url: r.get(1)?,
                        file_name: r.get(2)?,
                        content_type: r.get(3)?,
                        byte_size: r.get::<_, i64>(4)?.max(0) as u64,
                        updated_at: r.get::<_, i64>(5)?.max(0) as u64,
                        last_used_at: r.get::<_, i64>(6)?.max(0) as u64,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
        })
        .await
    }

    pub async fn touch_item_icon(&self, key: String, at: u64) -> anyhow::Result<()> {
        self.account_db(move |db| {
            db.execute(
                "UPDATE item_icons SET last_used_at = ?2 WHERE item_key = ?1",
                params![key, i64::try_from(at)?],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn mark_item_icon_cached(
        &self,
        key: String,
        source_url: String,
        file_name: String,
        content_type: String,
        byte_size: u64,
        at: u64,
    ) -> anyhow::Result<()> {
        self.account_db(move |db| {
            let at_i64 = i64::try_from(at)?;
            let size_i64 = i64::try_from(byte_size)?;
            db.execute(
                "INSERT INTO item_icons(item_key, source_url, file_name, content_type, byte_size, updated_at, last_used_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?6)
                 ON CONFLICT(item_key) DO UPDATE SET
                   source_url = excluded.source_url,
                   file_name = excluded.file_name,
                   content_type = excluded.content_type,
                   byte_size = excluded.byte_size,
                   updated_at = excluded.updated_at,
                   last_used_at = excluded.last_used_at",
                params![key, source_url, file_name, content_type, size_i64, at_i64],
            )?;
            Ok(())
        })
        .await
    }

    /// Evicts least-recently-used cached icon files once total disk usage exceeds `max_bytes`.
    pub async fn evict_icons_over_budget(
        &self,
        icons_dir: std::path::PathBuf,
        max_bytes: u64,
    ) -> anyhow::Result<(u64, u64)> {
        self.account_db(move |db| {
            let (_, mut total_bytes) = dir_cache_usage(&icons_dir);
            if total_bytes <= max_bytes {
                return Ok((0, 0));
            }
            let victims: Vec<(String, String, u64)> = {
                let mut stmt = db.prepare(
                    "SELECT item_key, file_name, byte_size
                     FROM item_icons
                     WHERE file_name IS NOT NULL
                     ORDER BY last_used_at ASC, updated_at ASC, rowid ASC",
                )?;
                stmt.query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?.max(0) as u64,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
            };
            let mut evicted_files = 0u64;
            let mut freed_bytes = 0u64;
            let tx = db.unchecked_transaction()?;
            {
                let mut clear_stmt = tx.prepare(
                    "UPDATE item_icons SET file_name = NULL, content_type = NULL, byte_size = 0 WHERE item_key = ?1",
                )?;
                for (key, fname, recorded_size) in victims {
                    if total_bytes <= max_bytes {
                        break;
                    }
                    let safe_name = Path::new(&fname).file_name().and_then(|s| s.to_str());
                    if let Some(name) = safe_name {
                        let path = icons_dir.join(name);
                        let actual_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(recorded_size);
                        if path.is_file() && std::fs::remove_file(&path).is_ok() {
                            evicted_files += 1;
                            freed_bytes += actual_size;
                            total_bytes = total_bytes.saturating_sub(actual_size);
                        }
                    }
                    clear_stmt.execute([key])?;
                }
            }
            tx.commit()?;
            Ok((evicted_files, freed_bytes))
        })
        .await
    }

    pub async fn maintenance_stats(
        &self,
        icons_dir: std::path::PathBuf,
        db_path: std::path::PathBuf,
        max_icon_bytes: u64,
        keep_scans: usize,
    ) -> anyhow::Result<MaintenanceStats> {
        self.account_db(move |db| {
            collect_stats_sync(db, &icons_dir, &db_path, max_icon_bytes, keep_scans)
        })
        .await
    }

    pub async fn clear_icon_cache(
        &self,
        icons_dir: std::path::PathBuf,
        db_path: std::path::PathBuf,
        max_icon_bytes: u64,
        keep_scans: usize,
    ) -> anyhow::Result<CleanupReport> {
        self.account_db(move |db| {
            let mut cleared_files = 0u64;
            let mut freed_bytes = 0u64;
            if let Ok(entries) = std::fs::read_dir(&icons_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(meta) = entry.metadata()
                        && meta.is_file()
                        && std::fs::remove_file(&path).is_ok()
                    {
                        cleared_files += 1;
                        freed_bytes = freed_bytes.saturating_add(meta.len());
                    }
                }
            }
            db.execute(
                "UPDATE item_icons SET file_name = NULL, content_type = NULL, byte_size = 0",
                [],
            )?;
            let stats = collect_stats_sync(db, &icons_dir, &db_path, max_icon_bytes, keep_scans)?;
            Ok(CleanupReport {
                cleared_icon_files: cleared_files,
                freed_icon_bytes: freed_bytes,
                deleted_scans: 0,
                deleted_snapshots: 0,
                reclaimed_db_bytes: 0,
                stats,
            })
        })
        .await
    }

    /// Prunes old finished scan logs and redundant market snapshots while preserving active accounts,
    /// ledger lots, audit evaluations, watchlists, strategies, and latest snapshots per scope.
    pub async fn cleanup_runtime_logs(
        &self,
        icons_dir: std::path::PathBuf,
        db_path: std::path::PathBuf,
        max_icon_bytes: u64,
        keep_scans: usize,
    ) -> anyhow::Result<CleanupReport> {
        self.account_db(move |db| {
            let (db_before, wal_before) = db_and_wal_bytes(&db_path);
            let keep_scans_i64 = i64::try_from(keep_scans).unwrap_or(1);

            let tx = rusqlite::Transaction::new_unchecked(
                db,
                rusqlite::TransactionBehavior::Immediate,
            )?;
            let deleted_scans = tx.execute(
                "DELETE FROM scans
                 WHERE COALESCE(json_extract(report, '$.status'), '') != 'running'
                   AND id NOT IN (
                     SELECT id FROM scans
                     WHERE COALESCE(json_extract(report, '$.status'), '') != 'running'
                     ORDER BY created_at DESC, rowid DESC
                     LIMIT ?1
                   )",
                [keep_scans_i64],
            )? as u64;

            let mut deleted_snapshots = 0u64;
            for spec in SNAPSHOT_SPECS {
                let sql = format!(
                    "DELETE FROM {} WHERE {}",
                    spec.table,
                    snapshot_where_cleanable(spec)
                );
                let keep = i64::try_from(spec.min_keep).unwrap_or(1);
                deleted_snapshots += tx.execute(&sql, [keep])? as u64;
            }
            tx.commit()?;

            let is_memory = db_path.as_os_str() == ":memory:";
            if !is_memory && (deleted_scans + deleted_snapshots > 0 || wal_before > 64 * 1024) {
                db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")?;
            }

            let (db_after, wal_after) = db_and_wal_bytes(&db_path);
            let reclaimed_db_bytes =
                (db_before + wal_before).saturating_sub(db_after + wal_after);
            let stats = collect_stats_sync(db, &icons_dir, &db_path, max_icon_bytes, keep_scans)?;
            Ok(CleanupReport {
                cleared_icon_files: 0,
                freed_icon_bytes: 0,
                deleted_scans,
                deleted_snapshots,
                reclaimed_db_bytes,
                stats,
            })
        })
        .await
    }
}
