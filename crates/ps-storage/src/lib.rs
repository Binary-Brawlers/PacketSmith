//! Local SQLite cache, full-text search indexing, and file watching for PacketSmith.
//!
//! SQLite is used strictly for derived data: execution history, search indexes,
//! cached responses, and transient workbench layout states. All user-authored resources
//! live as human-readable files in the workspace directory.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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

/// Filter criteria for querying execution history.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HistoryFilter {
    pub method: Option<String>,
    pub status_min: Option<u16>,
    pub status_max: Option<u16>,
    pub url_contains: Option<String>,
    pub limit: usize,
}

/// Entry stored in the derived search index for fast retrieval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchIndexEntry {
    pub resource_id: ResourceId,
    pub resource_type: String,
    pub title: String,
    pub path: String,
    pub content: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

/// Record of recently accessed resources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentResource {
    pub resource_id: ResourceId,
    pub resource_type: String,
    pub title: String,
    pub accessed_at: DateTime<Utc>,
}

/// Persistent workbench tab layout record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabStateRecord {
    pub id: String,
    pub workspace_id: String,
    pub resource_id: ResourceId,
    pub tab_order: i32,
    pub is_pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<String>,
}

/// Detailed run timing metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunMetadata {
    pub run_id: ResourceId,
    pub request_id: ResourceId,
    pub dns_lookup_ms: Option<u64>,
    pub tcp_connect_ms: Option<u64>,
    pub tls_handshake_ms: Option<u64>,
    pub ttfb_ms: Option<u64>,
    pub transfer_ms: Option<u64>,
    pub total_ms: u64,
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

            CREATE TABLE IF NOT EXISTS search_index (
                resource_id TEXT PRIMARY KEY,
                resource_type TEXT NOT NULL,
                title TEXT NOT NULL,
                path TEXT NOT NULL,
                content TEXT NOT NULL,
                tags TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_search_title ON search_index(title);
            CREATE INDEX IF NOT EXISTS idx_search_type ON search_index(resource_type);

            CREATE TABLE IF NOT EXISTS recent_resources (
                resource_id TEXT PRIMARY KEY,
                resource_type TEXT NOT NULL,
                title TEXT NOT NULL,
                accessed_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_recent_accessed_at ON recent_resources(accessed_at DESC);

            CREATE TABLE IF NOT EXISTS open_tabs (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                tab_order INTEGER NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                pane_id TEXT
            );

            CREATE TABLE IF NOT EXISTS workbench_layout (
                workspace_id TEXT PRIMARY KEY,
                layout_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS blobs (
                hash TEXT PRIMARY KEY,
                size_bytes INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                data BLOB
            );

            CREATE TABLE IF NOT EXISTS run_metadata (
                run_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                dns_lookup_ms INTEGER,
                tcp_connect_ms INTEGER,
                tls_handshake_ms INTEGER,
                ttfb_ms INTEGER,
                transfer_ms INTEGER,
                total_ms INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // History
    // -----------------------------------------------------------------------

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

    /// Queries history applying filters for method, status range, and URL substring.
    pub fn query_history(&self, filter: &HistoryFilter) -> Result<Vec<HistoryRecord>, StorageError> {
        let mut query = String::from(
            r#"
            SELECT id, request_id, request_name, protocol, method, url,
                   status_code, duration_ms, response_size_bytes, executed_at
            FROM history
            WHERE 1=1
            "#,
        );

        if let Some(ref m) = filter.method {
            query.push_str(&format!(" AND method = '{}'", m.to_uppercase()));
        }
        if let Some(min) = filter.status_min {
            query.push_str(&format!(" AND status_code >= {}", min));
        }
        if let Some(max) = filter.status_max {
            query.push_str(&format!(" AND status_code <= {}", max));
        }
        if let Some(ref u) = filter.url_contains {
            query.push_str(&format!(" AND url LIKE '%{}%'", u));
        }

        query.push_str(" ORDER BY executed_at DESC");
        let limit = if filter.limit == 0 { 50 } else { filter.limit };
        query.push_str(&format!(" LIMIT {}", limit));

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
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

    /// Clears all execution history records from SQLite cache.
    pub fn clear_history(&self) -> Result<usize, StorageError> {
        let deleted = self.conn.execute("DELETE FROM history", [])?;
        Ok(deleted)
    }

    /// Prunes execution history records older than the specified number of days.
    pub fn prune_history(&self, older_than_days: u32) -> Result<usize, StorageError> {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days as i64);
        let deleted = self.conn.execute(
            "DELETE FROM history WHERE executed_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;
        Ok(deleted)
    }

    // -----------------------------------------------------------------------
    // Search Index
    // -----------------------------------------------------------------------

    pub fn upsert_search_index(&self, entry: &SearchIndexEntry) -> Result<(), StorageError> {
        let tags_str = entry.tags.join(",");
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO search_index (
                resource_id, resource_type, title, path, content, tags, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                entry.resource_id.to_string(),
                entry.resource_type,
                entry.title,
                entry.path,
                entry.content,
                tags_str,
                entry.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn search_index(&self, query: &str, limit: usize) -> Result<Vec<SearchIndexEntry>, StorageError> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT resource_id, resource_type, title, path, content, tags, updated_at
            FROM search_index
            WHERE title LIKE ?1 OR content LIKE ?1 OR tags LIKE ?1
            ORDER BY title ASC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![pattern, limit as i64], |row| {
            let id_str: String = row.get(0)?;
            let tags_str: String = row.get(5)?;
            let updated_at_str: String = row.get(6)?;

            let resource_id = id_str.parse().unwrap_or_else(|_| ResourceId::new());
            let tags = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split(',').map(|s| s.to_string()).collect()
            };
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(SearchIndexEntry {
                resource_id,
                resource_type: row.get(1)?,
                title: row.get(2)?,
                path: row.get(3)?,
                content: row.get(4)?,
                tags,
                updated_at,
            })
        })?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Recent Resources
    // -----------------------------------------------------------------------

    pub fn record_recent(&self, id: &ResourceId, r_type: &str, title: &str) -> Result<(), StorageError> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO recent_resources (
                resource_id, resource_type, title, accessed_at
            ) VALUES (?1, ?2, ?3, ?4)
            "#,
            params![id.to_string(), r_type, title, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<RecentResource>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT resource_id, resource_type, title, accessed_at
            FROM recent_resources
            ORDER BY accessed_at DESC
            LIMIT ?1
            "#,
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let id_str: String = row.get(0)?;
            let accessed_at_str: String = row.get(3)?;

            let resource_id = id_str.parse().unwrap_or_else(|_| ResourceId::new());
            let accessed_at = DateTime::parse_from_rfc3339(&accessed_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(RecentResource {
                resource_id,
                resource_type: row.get(1)?,
                title: row.get(2)?,
                accessed_at,
            })
        })?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Blobs
    // -----------------------------------------------------------------------

    pub fn put_blob(&self, hash: &str, data: &[u8]) -> Result<(), StorageError> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO blobs (hash, size_bytes, created_at, data)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![hash, data.len() as i64, Utc::now().to_rfc3339(), data],
        )?;
        Ok(())
    }

    pub fn get_blob(&self, hash: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let mut stmt = self.conn.prepare("SELECT data FROM blobs WHERE hash = ?1")?;
        let mut rows = stmt.query(params![hash])?;
        if let Some(row) = rows.next()? {
            let data: Vec<u8> = row.get(0)?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    // -----------------------------------------------------------------------
    // Open Tabs & Workbench Layout Persistence
    // -----------------------------------------------------------------------

    pub fn save_open_tabs(&self, workspace_id: &str, tabs: &[TabStateRecord]) -> Result<(), StorageError> {
        self.conn.execute("DELETE FROM open_tabs WHERE workspace_id = ?1", params![workspace_id])?;
        let mut stmt = self.conn.prepare(
            r#"
            INSERT INTO open_tabs (id, workspace_id, resource_id, tab_order, is_pinned, pane_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )?;

        for tab in tabs {
            stmt.execute(params![
                tab.id,
                workspace_id,
                tab.resource_id.to_string(),
                tab.tab_order,
                if tab.is_pinned { 1 } else { 0 },
                tab.pane_id,
            ])?;
        }
        Ok(())
    }

    pub fn load_open_tabs(&self, workspace_id: &str) -> Result<Vec<TabStateRecord>, StorageError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, workspace_id, resource_id, tab_order, is_pinned, pane_id
            FROM open_tabs
            WHERE workspace_id = ?1
            ORDER BY tab_order ASC
            "#,
        )?;

        let rows = stmt.query_map(params![workspace_id], |row| {
            let res_id_str: String = row.get(2)?;
            let resource_id = res_id_str.parse().unwrap_or_else(|_| ResourceId::new());
            let is_pinned_int: i32 = row.get(4)?;

            Ok(TabStateRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                resource_id,
                tab_order: row.get(3)?,
                is_pinned: is_pinned_int != 0,
                pane_id: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn clear_open_tabs(&self, workspace_id: &str) -> Result<usize, StorageError> {
        let deleted = self.conn.execute("DELETE FROM open_tabs WHERE workspace_id = ?1", params![workspace_id])?;
        Ok(deleted)
    }

    pub fn save_workbench_layout(&self, workspace_id: &str, layout_json: &str) -> Result<(), StorageError> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO workbench_layout (workspace_id, layout_json, updated_at)
            VALUES (?1, ?2, ?3)
            "#,
            params![workspace_id, layout_json, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn load_workbench_layout(&self, workspace_id: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self.conn.prepare("SELECT layout_json FROM workbench_layout WHERE workspace_id = ?1")?;
        let mut rows = stmt.query(params![workspace_id])?;
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            Ok(Some(json))
        } else {
            Ok(None)
        }
    }

    pub fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }
}

// ---------------------------------------------------------------------------
// Workspace File Watcher & Conflict Detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

/// Workspace file watcher with debouncing and internal cache ignore rules.
#[derive(Debug)]
pub struct WorkspaceWatcher {
    root_path: PathBuf,
    ignore_patterns: Vec<String>,
    last_event_time: Arc<Mutex<Option<Instant>>>,
    debounce_duration: Duration,
}

impl WorkspaceWatcher {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            ignore_patterns: vec![
                ".packetsmith".to_string(),
                ".git".to_string(),
                ".DS_Store".to_string(),
            ],
            last_event_time: Arc::new(Mutex::new(None)),
            debounce_duration: Duration::from_millis(150),
        }
    }

    /// Determines if a changed path should be ignored.
    pub fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.ignore_patterns.iter().any(|pat| path_str.contains(pat))
    }

    /// Checks if sufficient debounce time has elapsed since the last recorded event.
    pub fn should_process_event(&self) -> bool {
        let mut last = match self.last_event_time.lock() {
            Ok(guard) => guard,
            Err(_) => return true,
        };

        let now = Instant::now();
        if let Some(prev) = *last {
            if now.duration_since(prev) < self.debounce_duration {
                return false;
            }
        }
        *last = Some(now);
        true
    }

    /// Checks for conflicts: returns true if an external file modification conflicts
    /// with known uncommitted dirty resource IDs.
    pub fn is_conflict(&self, dirty_resource_paths: &HashSet<PathBuf>, modified_path: &Path) -> bool {
        dirty_resource_paths.contains(modified_path)
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_indexing_and_query() {
        let storage = CacheStorage::open_in_memory().expect("open storage");
        let entry = SearchIndexEntry {
            resource_id: ResourceId::new(),
            resource_type: "request".to_string(),
            title: "Authenticate User".to_string(),
            path: "collections/auth/login.req.yaml".to_string(),
            content: "POST https://api.example.com/v1/login".to_string(),
            tags: vec!["auth".to_string(), "v1".to_string()],
            updated_at: Utc::now(),
        };

        storage.upsert_search_index(&entry).expect("upsert index");
        let results = storage.search_index("Authenticate", 10).expect("search index");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Authenticate User");
    }

    #[test]
    fn test_blob_storage() {
        let storage = CacheStorage::open_in_memory().expect("open storage");
        let payload = b"{\"message\": \"hello world\"}";
        storage.put_blob("hash123", payload).expect("put blob");

        let retrieved = storage.get_blob("hash123").expect("get blob");
        assert_eq!(retrieved, Some(payload.to_vec()));
    }

    #[test]
    fn test_workspace_watcher_ignore_rules() {
        let watcher = WorkspaceWatcher::new("/tmp/test-workspace");
        assert!(watcher.should_ignore(Path::new("/tmp/test-workspace/.packetsmith/cache.db")));
        assert!(watcher.should_ignore(Path::new("/tmp/test-workspace/.git/HEAD")));
        assert!(!watcher.should_ignore(Path::new("/tmp/test-workspace/collections/login.req.yaml")));
    }

    #[test]
    fn test_open_tabs_and_layout_persistence() {
        let storage = CacheStorage::open_in_memory().expect("open storage");
        let ws_id = "ws-123";
        let req1 = ResourceId::new();
        let req2 = ResourceId::new();

        let tabs = vec![
            TabStateRecord {
                id: "tab-1".to_string(),
                workspace_id: ws_id.to_string(),
                resource_id: req1,
                tab_order: 0,
                is_pinned: true,
                pane_id: Some("pane-left".to_string()),
            },
            TabStateRecord {
                id: "tab-2".to_string(),
                workspace_id: ws_id.to_string(),
                resource_id: req2,
                tab_order: 1,
                is_pinned: false,
                pane_id: Some("pane-right".to_string()),
            },
        ];

        storage.save_open_tabs(ws_id, &tabs).expect("save tabs");
        let loaded = storage.load_open_tabs(ws_id).expect("load tabs");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "tab-1");
        assert!(loaded[0].is_pinned);
        assert_eq!(loaded[0].pane_id, Some("pane-left".to_string()));
        assert_eq!(loaded[1].id, "tab-2");
        assert!(!loaded[1].is_pinned);

        let layout = "{\"split\":\"horizontal\",\"ratio\":0.5}";
        storage.save_workbench_layout(ws_id, layout).expect("save layout");
        let loaded_layout = storage.load_workbench_layout(ws_id).expect("load layout");
        assert_eq!(loaded_layout, Some(layout.to_string()));

        let cleared = storage.clear_open_tabs(ws_id).expect("clear tabs");
        assert_eq!(cleared, 2);
        assert_eq!(storage.load_open_tabs(ws_id).expect("load empty").len(), 0);
    }
}
