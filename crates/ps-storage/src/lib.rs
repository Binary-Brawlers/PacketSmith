//! Local SQLite cache and ephemeral data persistence for PacketSmith.
//!
//! SQLite is used strictly for derived data: execution history, search indexes,
//! cached responses, and transient workbench layout states. All user-authored resources
//! live as human-readable files in the workspace directory.

use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use ps_domain::ResourceId;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

pub const CACHE_DB_REL_PATH: &str = ".packetsmith/cache.db";

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Execution history entry stored in SQLite cache.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub id: ResourceId,
    pub request_id: Option<ResourceId>,
    pub request_name: String,
    pub protocol: String,
    pub method: String,
    pub url: String,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub response_size_bytes: usize,
    pub executed_at: DateTime<Utc>,
}

/// Storage service wrapping SQLite cache database.
#[derive(Debug)]
pub struct CacheStorage {
    conn: Connection,
    db_path: Option<PathBuf>,
}

impl CacheStorage {
    /// Opens or creates the SQLite cache database in the workspace directory.
    pub fn open_workspace(workspace_dir: &Path) -> Result<Self, StorageError> {
        let db_path = workspace_dir.join(CACHE_DB_REL_PATH);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&db_path)?;
        let mut storage = Self {
            conn,
            db_path: Some(db_path),
        };
        storage.configure_pragmas()?;
        storage.run_migrations()?;
        Ok(storage)
    }

    /// Opens an in-memory SQLite database, ideal for unit tests.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let mut storage = Self {
            conn,
            db_path: None,
        };
        storage.configure_pragmas()?;
        storage.run_migrations()?;
        Ok(storage)
    }

    fn configure_pragmas(&self) -> Result<(), StorageError> {
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        self.conn.pragma_update(None, "synchronous", "NORMAL")?;
        self.conn.pragma_update(None, "busy_timeout", 5000)?;
        Ok(())
    }

    fn run_migrations(&mut self) -> Result<(), StorageError> {
        info!("Running SQLite cache migrations");
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS history (
                id TEXT PRIMARY KEY,
                request_id TEXT,
                request_name TEXT NOT NULL,
                protocol TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                status_code INTEGER,
                duration_ms INTEGER NOT NULL,
                response_size_bytes INTEGER NOT NULL,
                executed_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_history_executed_at ON history(executed_at DESC);
            CREATE INDEX IF NOT EXISTS idx_history_request_id ON history(request_id);

            CREATE TABLE IF NOT EXISTS open_tabs (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                tab_order INTEGER NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;
        Ok(())
    }

    /// Inserts an execution record into history.
    pub fn insert_history(&self, record: &HistoryRecord) -> Result<(), StorageError> {
        self.conn.execute(
            r#"
            INSERT INTO history (
                id, request_id, request_name, protocol, method, url,
                status_code, duration_ms, response_size_bytes, executed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                record.id.to_string(),
                record.request_id.map(|id| id.to_string()),
                record.request_name,
                record.protocol,
                record.method,
                record.url,
                record.status_code,
                record.duration_ms,
                record.response_size_bytes as i64,
                record.executed_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Retrieves recent execution history entries up to the specified limit.
    pub fn list_history(&self, limit: usize) -> Result<Vec<HistoryRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, request_id, request_name, protocol, method, url,
                   status_code, duration_ms, response_size_bytes, executed_at
            FROM history
            ORDER BY executed_at DESC
            LIMIT ?1
            "#,
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let id_str: String = row.get(0)?;
            let req_id_str: Option<String> = row.get(1)?;
            let executed_at_str: String = row.get(9)?;

            let id = id_str.parse().unwrap_or_else(|_| ResourceId::new());
            let request_id = req_id_str.and_then(|s| s.parse().ok());
            let executed_at = DateTime::parse_from_rfc3339(&executed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let size_i64: i64 = row.get(8)?;

            Ok(HistoryRecord {
                id,
                request_id,
                request_name: row.get(2)?,
                protocol: row.get(3)?,
                method: row.get(4)?,
                url: row.get(5)?,
                status_code: row.get(6)?,
                duration_ms: row.get(7)?,
                response_size_bytes: size_i64 as usize,
                executed_at,
            })
        })?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }

    pub fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_insert_and_retrieve() {
        let storage = CacheStorage::open_in_memory().expect("Failed to open in-memory db");
        let record = HistoryRecord {
            id: ResourceId::new(),
            request_id: Some(ResourceId::new()),
            request_name: "Test Request".to_string(),
            protocol: "http".to_string(),
            method: "POST".to_string(),
            url: "https://httpbin.org/post".to_string(),
            status_code: Some(200),
            duration_ms: 125,
            response_size_bytes: 1024,
            executed_at: Utc::now(),
        };

        storage.insert_history(&record).expect("Insert failed");
        let list = storage.list_history(10).expect("Query failed");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].request_name, "Test Request");
        assert_eq!(list[0].status_code, Some(200));
    }
}
