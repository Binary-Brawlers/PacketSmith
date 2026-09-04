//! Workspace manifest definitions, discovery, and file serialization for PacketSmith.
//!
//! PacketSmith uses human-readable, Git-friendly YAML documents for native workspace definitions.
//! The workspace root contains a `packetsmith.yaml` file defining metadata and directory layout.

use std::fs;
use std::path::{Path, PathBuf};
use ps_domain::ResourceId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// File name of the root workspace manifest.
pub const WORKSPACE_MANIFEST_NAME: &str = "packetsmith.yaml";

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip_yaml() {
        let manifest = WorkspaceManifest::new("Production Services");
        let yaml = manifest.to_yaml().expect("YAML serialization failed");
        assert!(yaml.contains("version: 1.0.0"));
        assert!(yaml.contains("Production Services"));

        let loaded = WorkspaceManifest::from_yaml(&yaml).expect("YAML parsing failed");
        assert_eq!(manifest.id, loaded.id);
        assert_eq!(manifest.name, loaded.name);
        assert_eq!(manifest.resource_roots, loaded.resource_roots);
    }
}
