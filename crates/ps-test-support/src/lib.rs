//! Common testing fixtures, in-memory helpers, and mock generators for PacketSmith.

use std::fs;
use std::path::{Path, PathBuf};
use ps_domain::{HttpRequestPayload, ProtocolRequest, RequestDocument};
use ps_storage::{CacheStorage, HistoryRecord};
use ps_workspace::WorkspaceManifest;

/// Creates a temporary workspace directory structure with a valid `packetsmith.yaml`.
pub fn create_test_workspace_in(dir: &Path, name: &str) -> PathBuf {
    let ws_manifest = WorkspaceManifest::new(name);
    fs::create_dir_all(dir.join("collections")).expect("create collections dir");
    fs::create_dir_all(dir.join("environments")).expect("create environments dir");
    fs::create_dir_all(dir.join("specs")).expect("create specs dir");
    ws_manifest.save_to_dir(dir).expect("save manifest");
    dir.to_path_buf()
}

/// Generates a sample HTTP GET request document.
pub fn sample_http_get_request(name: &str, url: &str) -> RequestDocument {
    RequestDocument::new(
        name,
        ProtocolRequest::Http(HttpRequestPayload {
            method: "GET".to_string(),
            url: url.to_string(),
        }),
    )
}

/// Generates a sample execution history record.
pub fn sample_history_record(req_name: &str, url: &str, status: u16) -> HistoryRecord {
    HistoryRecord {
        id: ps_domain::ResourceId::new(),
        request_id: Some(ps_domain::ResourceId::new()),
        request_name: req_name.to_string(),
        protocol: "http".to_string(),
        method: "GET".to_string(),
        url: url.to_string(),
        status_code: Some(status),
        duration_ms: 45,
        response_size_bytes: 512,
        executed_at: chrono::Utc::now(),
    }
}

/// Creates an in-memory cache storage initialized with schema.
pub fn in_memory_storage() -> CacheStorage {
    CacheStorage::open_in_memory().expect("open in-memory storage")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_request_creation() {
        let req = sample_http_get_request("Get Users", "https://api.example.com/users");
        assert_eq!(req.name, "Get Users");
    }
}
