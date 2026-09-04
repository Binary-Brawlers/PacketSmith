//! Workspace manifest definitions, discovery, and file serialization for PacketSmith.
//!
//! PacketSmith uses human-readable, Git-friendly YAML documents for native workspace definitions.
//! Resources are persisted in subdirectories using deterministic slug naming conventions:
//! - `<slug>.req.yaml` for Request documents
//! - `<slug>.col.yaml` for Collection documents
//! - `<slug>.folder.yaml` for Folder documents
//! - `<slug>.env.yaml` for Environment documents

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use ps_domain::{FolderDocument, ResourceId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod collection_manager;
pub mod tree;

pub use collection_manager::{CollectionManager, ResourceParent, EXAMPLE_EXT, TRASH_DIR_REL};
pub use tree::{
    BreadcrumbItem, CollectionNode, FolderNode, QuickOpenResult, RequestNode, ResourceTree,
};

/// Root manifest filename.
pub const WORKSPACE_MANIFEST_NAME: &str = "packetsmith.yaml";

/// Standard file extensions for resources.
pub const REQUEST_EXT: &str = ".req.yaml";
pub const COLLECTION_EXT: &str = ".col.yaml";
pub const FOLDER_EXT: &str = ".folder.yaml";
pub const ENVIRONMENT_EXT: &str = ".env.yaml";

/// Schema version for the workspace format.
pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("Manifest file not found at {0}")]
    NotFound(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML serialization or deserialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid schema version: expected '{0}', got '{1}'")]
    UnsupportedSchemaVersion(String, String),
    #[error("Circular folder hierarchy detected involving folder ID: {0}")]
    CircularHierarchy(ResourceId),
    #[error("Resource collision: filename '{0}' already exists")]
    Collision(String),
}

/// Converts a human-readable name into a filesystem-safe slug (lowercase, alphanumeric + hyphens).
pub fn slugify(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();

    let trimmed = slug.trim_matches('-');
    let mut clean = String::new();
    let mut last_was_dash = false;
    for c in trimmed.chars() {
        if c == '-' {
            if !last_was_dash {
                clean.push(c);
                last_was_dash = true;
            }
        } else {
            clean.push(c);
            last_was_dash = false;
        }
    }

    if clean.is_empty() {
        "resource".to_string()
    } else {
        clean
    }
}

/// Root manifest representing a PacketSmith workspace (`packetsmith.yaml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub version: String,
    pub id: ResourceId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_resource_roots")]
    pub resource_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
}

fn default_resource_roots() -> Vec<String> {
    vec![
        "collections".to_string(),
        "environments".to_string(),
        "specs".to_string(),
    ]
}

impl WorkspaceManifest {
    /// Creates a new workspace manifest with default configuration.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            version: CURRENT_SCHEMA_VERSION.to_string(),
            id: ResourceId::new(),
            name: name.into(),
            description: None,
            resource_roots: default_resource_roots(),
            active_environment: None,
        }
    }

    /// Loads a workspace manifest from a given directory path.
    pub fn load_from_dir(dir: &Path) -> Result<Self, WorkspaceError> {
        let manifest_path = dir.join(WORKSPACE_MANIFEST_NAME);
        if !manifest_path.exists() {
            return Err(WorkspaceError::NotFound(manifest_path));
        }
        let content = fs::read_to_string(&manifest_path)?;
        let manifest: Self = serde_yaml::from_str(&content)?;
        Ok(manifest)
    }

    /// Saves the workspace manifest to the specified directory.
    pub fn save_to_dir(&self, dir: &Path) -> Result<PathBuf, WorkspaceError> {
        let manifest_path = dir.join(WORKSPACE_MANIFEST_NAME);
        let content = serde_yaml::to_string(self)?;
        fs::write(&manifest_path, content)?;
        Ok(manifest_path)
    }

    /// Serializes the manifest deterministically to a YAML string.
    pub fn to_yaml(&self) -> Result<String, WorkspaceError> {
        let serialized = serde_yaml::to_string(self)?;
        Ok(serialized)
    }

    /// Parses a manifest from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, WorkspaceError> {
        let manifest: Self = serde_yaml::from_str(yaml)?;
        Ok(manifest)
    }
}

/// Deterministically serializes any serializable resource to a clean YAML string.
pub fn serialize_resource_to_yaml<T: Serialize>(resource: &T) -> Result<String, WorkspaceError> {
    let yaml = serde_yaml::to_string(resource)?;
    Ok(yaml)
}

/// Parses any resource from a YAML string.
pub fn parse_resource_from_yaml<T: DeserializeOwned>(yaml: &str) -> Result<T, WorkspaceError> {
    let resource = serde_yaml::from_str(yaml)?;
    Ok(resource)
}

/// Validates folder relationships ensuring there are no circular parent hierarchies.
pub fn validate_folder_hierarchy(folders: &[FolderDocument]) -> Result<(), WorkspaceError> {
    let parent_map: HashMap<ResourceId, Option<ResourceId>> = folders
        .iter()
        .map(|f| (f.id, f.parent_id))
        .collect();

    for &id in parent_map.keys() {
        let mut visited = HashSet::new();
        let mut curr = Some(id);

        while let Some(current_id) = curr {
            if !visited.insert(current_id) {
                return Err(WorkspaceError::CircularHierarchy(current_id));
            }
            curr = parent_map.get(&current_id).copied().flatten();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_domain::{HttpRequestPayload, ProtocolRequest, RequestDocument};

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Get User Profile"), "get-user-profile");
        assert_eq!(slugify("V1 / Auth / Login!"), "v1-auth-login");
        assert_eq!(slugify("---"), "resource");
    }

    #[test]
    fn test_request_yaml_serialization() {
        let req = RequestDocument::new(
            "List Items",
            ProtocolRequest::Http(HttpRequestPayload {
                method: "GET".to_string(),
                url: "https://api.example.com/items".to_string(),
            }),
        );
        let yaml = serialize_resource_to_yaml(&req).expect("serialize request");
        assert!(yaml.contains("List Items"));
        assert!(yaml.contains("https://api.example.com/items"));

        let loaded: RequestDocument = parse_resource_from_yaml(&yaml).expect("parse request");
        assert_eq!(req.id, loaded.id);
        assert_eq!(req.name, loaded.name);
    }

    #[test]
    fn test_folder_hierarchy_cycle_detection() {
        let col_id = ResourceId::new();
        let mut f1 = FolderDocument::new(col_id, "Folder 1");
        let mut f2 = FolderDocument::new(col_id, "Folder 2");

        // Create cycle: f1 -> f2 -> f1
        f1.parent_id = Some(f2.id);
        f2.parent_id = Some(f1.id);

        let res = validate_folder_hierarchy(&[f1, f2]);
        assert!(res.is_err());
        match res.unwrap_err() {
            WorkspaceError::CircularHierarchy(_) => {}
            other => panic!("Expected CircularHierarchy error, got {:?}", other),
        }
    }
}
