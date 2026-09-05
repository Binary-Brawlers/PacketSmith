//! Native environment resources. Local overrides are session-only until a secure
//! local store is available; neither exports nor workspace files contain secrets.
use crate::{
    parse_resource_from_yaml, serialize_resource_to_yaml, WorkspaceError, ENVIRONMENT_EXT,
};
use chrono::Utc;
use ps_domain::{EnvironmentDocument, ResourceId, VariableEntry, VariableType};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvironmentError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Environment does not exist: {0}")]
    Missing(ResourceId),
    #[error("Invalid environment: {0}")]
    Invalid(String),
}

/// Safe table data: secret contents never reach the display model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentRow {
    pub key: String,
    pub default_value: String,
    pub current_value: Option<String>,
    pub is_secret: bool,
    pub enabled: bool,
    pub value_type: VariableType,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDifference {
    pub key: String,
    pub left: Option<EnvironmentRow>,
    pub right: Option<EnvironmentRow>,
}

#[derive(Clone)]
pub struct EnvironmentManager {
    root: PathBuf,
    documents: HashMap<ResourceId, EnvironmentDocument>,
    paths: HashMap<ResourceId, PathBuf>,
    local: HashMap<(ResourceId, String), String>,
}

impl std::fmt::Debug for EnvironmentManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvironmentManager")
            .field("root", &self.root)
            .field("environment_count", &self.documents.len())
            .finish_non_exhaustive()
    }
}

impl EnvironmentManager {
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            root: workspace.into().join("environments"),
            documents: HashMap::new(),
            paths: HashMap::new(),
            local: HashMap::new(),
        }
    }

    pub fn scan(workspace: impl Into<PathBuf>) -> Result<Self, EnvironmentError> {
        let mut manager = Self::new(workspace);
        if !manager.root.exists() {
            return Ok(manager);
        }
        for entry in fs::read_dir(&manager.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || !entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(ENVIRONMENT_EXT)
            {
                continue;
            }
            let doc: EnvironmentDocument =
                parse_resource_from_yaml(&fs::read_to_string(entry.path())?)?;
            validate(&doc)?;
            if manager.documents.contains_key(&doc.id) {
                return Err(EnvironmentError::Invalid("Duplicate environment ID".into()));
            }
            manager.paths.insert(doc.id, entry.path());
            manager.documents.insert(doc.id, doc);
        }
        Ok(manager)
    }

    pub fn list(&self) -> Vec<&EnvironmentDocument> {
        let mut docs: Vec<_> = self.documents.values().collect();
        docs.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.id.to_string().cmp(&b.id.to_string()))
        });
        docs
    }

    pub fn get(&self, id: ResourceId) -> Result<&EnvironmentDocument, EnvironmentError> {
        self.documents.get(&id).ok_or(EnvironmentError::Missing(id))
    }

    pub fn create(&mut self, name: impl Into<String>) -> Result<ResourceId, EnvironmentError> {
        self.insert_new(EnvironmentDocument::new(name))
    }

    fn insert_new(&mut self, mut doc: EnvironmentDocument) -> Result<ResourceId, EnvironmentError> {
        validate(&doc)?;
        if self.documents.contains_key(&doc.id) {
            return Err(EnvironmentError::Invalid("Duplicate environment ID".into()));
        }
        // Default secret values may only be vault references. Raw secrets must
        // be explicitly supplied as session overrides instead.
        protect_defaults(&mut doc);
        fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{}{ENVIRONMENT_EXT}", doc.id));
        let yaml = serialize_resource_to_yaml(&doc)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        if let Err(error) = file
            .write_all(yaml.as_bytes())
            .and_then(|_| file.sync_all())
        {
            let _ = fs::remove_file(&path);
            return Err(error.into());
        }
        let id = doc.id;
        self.paths.insert(id, path);
        self.documents.insert(id, doc);
        Ok(id)
    }

    pub fn save(&mut self, mut doc: EnvironmentDocument) -> Result<(), EnvironmentError> {
        self.get(doc.id)?;
        validate(&doc)?;
        protect_defaults(&mut doc);
        doc.updated_at = Utc::now();
        let path = self
            .paths
            .get(&doc.id)
            .ok_or(EnvironmentError::Missing(doc.id))?;
        let temporary = self.root.join(format!("{}.tmp", ResourceId::new()));
        let result = (|| -> Result<(), EnvironmentError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(serialize_resource_to_yaml(&doc)?.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
        self.local.retain(|(id, key), _| {
            *id != doc.id
                || doc.variables.iter().any(|v| {
                    &v.key == key
                        && self.documents[&doc.id].variables.iter().any(|old| {
                            old.key == v.key
                                && old.is_secret == v.is_secret
                                && old.value_type == v.value_type
                        })
                })
        });
        self.documents.insert(doc.id, doc);
        Ok(())
    }

    pub fn rename(
        &mut self,
        id: ResourceId,
        name: impl Into<String>,
    ) -> Result<(), EnvironmentError> {
        let mut doc = self.get(id)?.clone();
        doc.name = name.into();
        self.save(doc)
    }

    /// Clone defaults under a fresh identity. Session values are never copied.
    pub fn duplicate(
        &mut self,
        id: ResourceId,
        name: impl Into<String>,
    ) -> Result<ResourceId, EnvironmentError> {
        let mut doc = self.get(id)?.clone();
        doc.id = ResourceId::new();
        doc.name = name.into();
        doc.created_at = Utc::now();
        doc.updated_at = doc.created_at;
        self.insert_new(doc)
    }

    pub fn delete(&mut self, id: ResourceId) -> Result<(), EnvironmentError> {
        self.get(id)?;
        fs::remove_file(self.paths.get(&id).ok_or(EnvironmentError::Missing(id))?)?;
        self.paths.remove(&id);
        self.documents.remove(&id);
        self.local.retain(|(env, _), _| *env != id);
        Ok(())
    }

    pub fn set_current(
        &mut self,
        id: ResourceId,
        key: &str,
        value: Option<String>,
    ) -> Result<(), EnvironmentError> {
        let variable = self
            .get(id)?
            .variables
            .iter()
            .find(|v| v.key == key)
            .ok_or_else(|| EnvironmentError::Invalid("Unknown variable".into()))?;
        if let Some(value) = value {
            validate_value(variable.value_type, &value)?;
            self.local.insert((id, key.into()), value);
        } else {
            self.local.remove(&(id, key.into()));
        }
        Ok(())
    }

    /// Execution-only values; callers must retain the secret flag for redaction.
    pub fn effective_variables(
        &self,
        id: ResourceId,
    ) -> Result<Vec<VariableEntry>, EnvironmentError> {
        Ok(self
            .get(id)?
            .variables
            .iter()
            .filter(|v| v.enabled)
            .cloned()
            .map(|mut v| {
                if let Some(value) = self.local.get(&(id, v.key.clone())) {
                    v.value = value.clone();
                }
                v
            })
            .collect())
    }

    pub fn rows(&self, id: ResourceId) -> Result<Vec<EnvironmentRow>, EnvironmentError> {
        Ok(self
            .get(id)?
            .variables
            .iter()
            .map(|v| {
                let display = |s: &str| {
                    if v.is_secret || v.value_type == VariableType::SecretReference {
                        "••••••••".into()
                    } else {
                        s.to_string()
                    }
                };
                EnvironmentRow {
                    key: v.key.clone(),
                    default_value: display(&v.value),
                    current_value: self.local.get(&(id, v.key.clone())).map(|s| display(s)),
                    is_secret: v.is_secret,
                    enabled: v.enabled,
                    value_type: v.value_type,
                    description: v.description.clone(),
                }
            })
            .collect())
    }

    pub fn export(&self, id: ResourceId) -> Result<String, EnvironmentError> {
        let mut doc = self.get(id)?.clone();
        protect_defaults(&mut doc);
        Ok(serialize_resource_to_yaml(&doc)?)
    }

    /// Native YAML import always creates a fresh identity and strips raw secrets.
    pub fn import(&mut self, yaml: &str) -> Result<ResourceId, EnvironmentError> {
        let mut doc: EnvironmentDocument = parse_resource_from_yaml(yaml)?;
        doc.id = ResourceId::new();
        doc.created_at = Utc::now();
        doc.updated_at = doc.created_at;
        self.insert_new(doc)
    }

    pub fn diff(
        &self,
        left: ResourceId,
        right: ResourceId,
    ) -> Result<Vec<EnvironmentDifference>, EnvironmentError> {
        let a = self.rows(left)?;
        let b = self.rows(right)?;
        let mut keys: Vec<_> = a.iter().chain(&b).map(|r| r.key.clone()).collect();
        keys.sort();
        keys.dedup();
        Ok(keys
            .into_iter()
            .filter_map(|key| {
                let left = a.iter().find(|r| r.key == key).cloned();
                let right = b.iter().find(|r| r.key == key).cloned();
                // Secret values are intentionally not compared or exposed.
                (left != right).then_some(EnvironmentDifference { key, left, right })
            })
            .collect())
    }

    pub fn missing_values(&self, id: ResourceId) -> Result<Vec<String>, EnvironmentError> {
        Ok(self
            .effective_variables(id)?
            .into_iter()
            .filter(|v| v.value.trim().is_empty())
            .map(|v| v.key)
            .collect())
    }
}

fn protect_defaults(doc: &mut EnvironmentDocument) {
    for v in &mut doc.variables {
        if v.is_secret && v.value_type != VariableType::SecretReference {
            v.value.clear();
        }
    }
}

fn validate(doc: &EnvironmentDocument) -> Result<(), EnvironmentError> {
    if doc.name.trim().is_empty() {
        return Err(EnvironmentError::Invalid("Name is required".into()));
    }
    let mut keys = HashSet::new();
    for variable in &doc.variables {
        if variable.key.trim().is_empty()
            || !variable
                .key
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
            || !keys.insert(&variable.key)
        {
            return Err(EnvironmentError::Invalid(
                "Empty or duplicate variable key".into(),
            ));
        }
        validate_value(variable.value_type, &variable.value)?;
    }
    Ok(())
}

fn validate_value(kind: VariableType, value: &str) -> Result<(), EnvironmentError> {
    if value.is_empty() || value.contains("{{") && kind != VariableType::SecretReference {
        return Ok(());
    }
    let valid = match kind {
        VariableType::String => true,
        VariableType::Number => serde_json::from_str::<serde_json::Number>(value).is_ok(),
        VariableType::Boolean => matches!(value, "true" | "false"),
        VariableType::Json => serde_json::from_str::<serde_json::Value>(value).is_ok(),
        VariableType::SecretReference => value
            .strip_prefix("{{vault:")
            .and_then(|s| s.strip_suffix("}}"))
            .is_some_and(|name| {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
            }),
    };
    if valid {
        Ok(())
    } else {
        Err(EnvironmentError::Invalid(
            "Value does not match declared type".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("ps-environments-{}", ResourceId::new())))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn secrets_and_current_values_never_leave_the_session() {
        let root = Fixture::new();
        let mut manager = EnvironmentManager::new(&root.0);
        let id = manager.create("Production").unwrap();
        let mut doc = manager.get(id).unwrap().clone();
        doc.variables = vec![
            VariableEntry::secret("token", "default-secret"),
            VariableEntry::new("host", "default-host"),
        ];
        manager.save(doc).unwrap();
        manager
            .set_current(id, "token", Some("local-secret".into()))
            .unwrap();
        manager
            .set_current(id, "host", Some("local-host".into()))
            .unwrap();
        let export = manager.export(id).unwrap();
        for hidden in ["default-secret", "local-secret", "local-host"] {
            assert!(!export.contains(hidden));
        }
        assert!(!format!("{manager:?}").contains("local-secret"));
        assert_eq!(
            manager.rows(id).unwrap()[0].current_value.as_deref(),
            Some("••••••••")
        );
        assert_eq!(
            manager.effective_variables(id).unwrap()[0].value,
            "local-secret"
        );
        let reloaded = EnvironmentManager::scan(&root.0).unwrap();
        assert_eq!(reloaded.missing_values(id).unwrap(), vec!["token"]);
        let persisted = fs::read_to_string(&manager.paths[&id]).unwrap();
        assert!(!persisted.contains("default-secret"));
        assert!(!persisted.contains("local-secret"));
        let copy = manager.duplicate(id, "Copy").unwrap();
        assert_ne!(id, copy);
        assert!(manager.effective_variables(copy).unwrap()[0]
            .value
            .is_empty());
    }

    #[test]
    fn lifecycle_import_and_validation() {
        let root = Fixture::new();
        let mut manager = EnvironmentManager::new(&root.0);
        let id = manager.create("Staging").unwrap();
        let mut doc = manager.get(id).unwrap().clone();
        doc.variables.push(VariableEntry::new("host", "staging"));
        manager.save(doc.clone()).unwrap();
        manager.rename(id, "Production").unwrap();
        assert_eq!(
            EnvironmentManager::scan(&root.0)
                .unwrap()
                .get(id)
                .unwrap()
                .name,
            "Production"
        );
        let imported = manager.import(&manager.export(id).unwrap()).unwrap();
        assert_ne!(id, imported);
        assert!(manager.diff(id, imported).unwrap().is_empty());
        manager
            .set_current(imported, "host", Some("production".into()))
            .unwrap();
        assert_eq!(manager.diff(id, imported).unwrap().len(), 1);
        doc.variables.push(VariableEntry::new("host", "duplicate"));
        assert!(manager.save(doc).is_err());
        manager.delete(id).unwrap();
        assert!(EnvironmentManager::scan(&root.0).unwrap().get(id).is_err());
    }

    #[test]
    fn typed_values_and_vault_reference_validation_do_not_echo_secrets() {
        assert!(validate_value(VariableType::Number, "12.5").is_ok());
        assert!(validate_value(VariableType::Boolean, "false").is_ok());
        assert!(validate_value(VariableType::Json, "{\"a\":1}").is_ok());
        assert!(validate_value(VariableType::SecretReference, "{{vault:token}}").is_ok());
        let error = validate_value(VariableType::SecretReference, "private-token").unwrap_err();
        assert!(!error.to_string().contains("private-token"));
        assert!(validate_value(VariableType::Number, "NaN").is_err());
    }
}
