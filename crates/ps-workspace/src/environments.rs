//! Native environments with local persistence for non-secret overrides.
//! Secret overrides stay in memory and are excluded from local snapshots.
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

// Do not derive Debug: even non-secret local values are private workspace data.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalEnvironmentState {
    version: u32,
    active: Option<ResourceId>,
    overrides: Vec<LocalOverride>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalOverride {
    environment: ResourceId,
    key: String,
    value: String,
    value_type: VariableType,
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

    fn local_state_path(&self) -> PathBuf {
        self.root.parent().expect("environment root has a parent")
            .join(".packetsmith/environments.local.json")
    }

    /// Persist selection and non-secret overrides only. The snapshot is separate
    /// from portable resources, and is never used by duplicate or export.
    pub fn persist_local_state(&self, active: Option<ResourceId>) -> Result<(), EnvironmentError> {
        if let Some(id) = active { self.get(id)?; }
        let mut overrides = Vec::new();
        for ((id, key), value) in &self.local {
            if let Some(variable) = self.get(*id)?.variables.iter().find(|v| &v.key == key) {
                if !variable.is_secret && variable.value_type != VariableType::SecretReference {
                    overrides.push(LocalOverride {
                        environment: *id, key: key.clone(), value: value.clone(),
                        value_type: variable.value_type,
                    });
                }
            }
        }
        overrides.sort_by(|a, b| a.environment.to_string().cmp(&b.environment.to_string()).then(a.key.cmp(&b.key)));
        let state = LocalEnvironmentState { version: 1, active, overrides };
        let bytes = serde_json::to_vec_pretty(&state)
            .map_err(|_| EnvironmentError::Invalid("Cannot serialize local environment state".into()))?;
        let path = self.local_state_path();
        let directory = path.parent().expect("local state has a parent");
        fs::create_dir_all(directory)?;
        let temporary = directory.join(format!("{}.local.json", ResourceId::new()));
        let result = (|| -> Result<(), EnvironmentError> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &path)?;
            Ok(())
        })();
        if result.is_err() { let _ = fs::remove_file(temporary); }
        result
    }

    /// Restore a validated snapshot atomically. Removed keys, changed types and
    /// newly secret variables cannot resurrect obsolete persisted values.
    pub fn restore_local_state(&mut self) -> Result<Option<ResourceId>, EnvironmentError> {
        let bytes = match fs::read(self.local_state_path()) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let state: LocalEnvironmentState = serde_json::from_slice(&bytes)
            .map_err(|_| EnvironmentError::Invalid("Invalid local environment state".into()))?;
        if state.version != 1 {
            return Err(EnvironmentError::Invalid("Unsupported local environment state version".into()));
        }
        let mut local = HashMap::new();
        for entry in state.overrides {
            let Some(variable) = self.documents.get(&entry.environment)
                .and_then(|doc| doc.variables.iter().find(|v| v.key == entry.key)) else { continue; };
            if variable.is_secret || variable.value_type == VariableType::SecretReference
                || variable.value_type != entry.value_type { continue; }
            validate_value(variable.value_type, &entry.value)?;
            if local.insert((entry.environment, entry.key), entry.value).is_some() {
                return Err(EnvironmentError::Invalid("Duplicate local environment override".into()));
            }
        }
        self.local = local;
        Ok(state.active.filter(|id| self.documents.contains_key(id)))
    }

    /// Rescan portable resources while retaining compatible session overrides,
    /// including secrets. Failure leaves the original manager untouched.
    pub fn reload(&mut self) -> Result<(), EnvironmentError> {
        let mut next = Self::scan(self.root.parent().expect("environment root has a parent"))?;
        for ((id, key), value) in &self.local {
            let old = self.documents.get(id).and_then(|doc| doc.variables.iter().find(|v| &v.key == key));
            let new = next.documents.get(id).and_then(|doc| doc.variables.iter().find(|v| &v.key == key));
            if let (Some(old), Some(new)) = (old, new) {
                if old.is_secret == new.is_secret && old.value_type == new.value_type {
                    next.local.insert((*id, key.clone()), value.clone());
                }
            }
        }
        *self = next;
        Ok(())
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

    /// Compare a target against enabled keys in a reference environment as well
    /// as its own enabled keys. Returns names only, never credential contents.
    pub fn missing_values_against(&self, target: ResourceId, reference: ResourceId) -> Result<Vec<String>, EnvironmentError> {
        let effective = self.effective_variables(target)?;
        let mut keys = self.missing_values(target)?;
        for variable in self.get(reference)?.variables.iter().filter(|v| v.enabled) {
            if !effective.iter().any(|v| v.key == variable.key && !v.value.trim().is_empty()) {
                keys.push(variable.key.clone());
            }
        }
        keys.sort();
        keys.dedup();
        Ok(keys)
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
    fn production_check_includes_absent_disabled_and_empty_reference_keys() {
        let root = Fixture::new();
        let mut manager = EnvironmentManager::new(&root.0);
        let reference = manager.create("Development").unwrap();
        let target = manager.create("Production").unwrap();
        let mut doc = manager.get(reference).unwrap().clone();
        doc.variables = vec![VariableEntry::new("host", "localhost"), VariableEntry::secret("token", ""), VariableEntry::new("absent", "required")];
        manager.save(doc).unwrap();
        let mut doc = manager.get(target).unwrap().clone();
        let mut disabled = VariableEntry::new("host", "production");
        disabled.enabled = false;
        doc.variables = vec![disabled, VariableEntry::secret("token", ""), VariableEntry::new("empty", "")];
        manager.save(doc).unwrap();
        manager.set_current(target, "token", Some("private-production-token".into())).unwrap();
        assert_eq!(manager.missing_values_against(target, reference).unwrap(), vec!["absent", "empty", "host"]);
        let diff = format!("{:?}", manager.diff(reference, target).unwrap());
        assert!(!diff.contains("private-production-token"));
        let clone = manager.duplicate(target, "Clone").unwrap();
        assert!(manager.effective_variables(clone).unwrap().iter().find(|v| v.key == "token").unwrap().value.is_empty());
    }

    #[test]
    fn local_snapshot_restores_selection_and_empty_values_without_secrets() {
        let root = Fixture::new();
        let mut manager = EnvironmentManager::new(&root.0);
        let id = manager.create("Production").unwrap();
        let mut doc = manager.get(id).unwrap().clone();
        doc.variables = vec![VariableEntry::new("host", "default"), VariableEntry::secret("token", "")];
        manager.save(doc).unwrap();
        manager.set_current(id, "host", Some(String::new())).unwrap();
        manager.set_current(id, "token", Some("private-token".into())).unwrap();
        manager.persist_local_state(Some(id)).unwrap();
        let snapshot = fs::read_to_string(manager.local_state_path()).unwrap();
        assert!(!snapshot.contains("private-token"));
        assert!(!snapshot.contains("token"));
        let mut restored = EnvironmentManager::scan(&root.0).unwrap();
        assert_eq!(restored.restore_local_state().unwrap(), Some(id));
        assert_eq!(restored.rows(id).unwrap()[0].current_value.as_deref(), Some(""));
        assert_eq!(restored.rows(id).unwrap()[1].current_value, None);
        manager.set_current(id, "host", None).unwrap();
        manager.persist_local_state(None).unwrap();
        assert_eq!(restored.restore_local_state().unwrap(), None);
        assert_eq!(restored.rows(id).unwrap()[0].current_value, None);
    }

    #[test]
    fn reload_retains_sessions_but_discards_changed_classifications() {
        let root = Fixture::new();
        let mut manager = EnvironmentManager::new(&root.0);
        let id = manager.create("Staging").unwrap();
        let mut doc = manager.get(id).unwrap().clone();
        doc.variables = vec![VariableEntry::new("host", "default"), VariableEntry::secret("token", "")];
        manager.save(doc.clone()).unwrap();
        manager.set_current(id, "host", Some("local".into())).unwrap();
        manager.set_current(id, "token", Some("session-secret".into())).unwrap();
        manager.persist_local_state(Some(id)).unwrap();
        manager.reload().unwrap();
        assert_eq!(manager.effective_variables(id).unwrap()[1].value, "session-secret");
        let mut external = EnvironmentManager::scan(&root.0).unwrap();
        doc.variables[0].is_secret = true;
        external.save(doc).unwrap();
        manager.reload().unwrap();
        assert_eq!(manager.rows(id).unwrap()[0].current_value, None);
        assert_eq!(external.restore_local_state().unwrap(), Some(id));
        assert_eq!(external.rows(id).unwrap()[0].current_value, None);
        external.delete(id).unwrap();
        assert_eq!(external.restore_local_state().unwrap(), None);
    }

    #[test]
    fn invalid_snapshot_is_atomic_and_does_not_echo_contents() {
        let root = Fixture::new();
        let mut manager = EnvironmentManager::new(&root.0);
        let id = manager.create("Staging").unwrap();
        let mut doc = manager.get(id).unwrap().clone();
        doc.variables.push(VariableEntry::new("host", "default"));
        manager.save(doc).unwrap();
        manager.set_current(id, "host", Some("local".into())).unwrap();
        manager.persist_local_state(Some(id)).unwrap();
        fs::write(manager.local_state_path(), "private-invalid-content").unwrap();
        let error = manager.restore_local_state().unwrap_err();
        assert!(!error.to_string().contains("private-invalid-content"));
        assert_eq!(manager.effective_variables(id).unwrap()[0].value, "local");
        fs::write(manager.local_state_path(), r#"{"version":2,"active":null,"overrides":[]}"#).unwrap();
        assert!(manager.restore_local_state().is_err());
        assert_eq!(manager.effective_variables(id).unwrap()[0].value, "local");
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
