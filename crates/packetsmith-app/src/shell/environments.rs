//! Environment workflows shared by native controls and regression tests.
use super::WorkspaceState;
use ps_domain::{ResourceId, VariableEntry, VariableType};
use ps_workspace::EnvironmentError;

impl WorkspaceState {
    pub fn create_environment(&mut self, name: &str) -> Result<ResourceId, EnvironmentError> {
        let id = self.environments.create(name)?;
        self.select_environment(Some(id))?;
        Ok(id)
    }

    pub fn rename_environment(
        &mut self,
        id: ResourceId,
        name: &str,
    ) -> Result<(), EnvironmentError> {
        self.environments.rename(id, name)?;
        self.refresh_environment_variables()
    }

    /// Clone and duplicate share fresh-ID/default-only semantics.
    pub fn clone_environment(
        &mut self,
        id: ResourceId,
        name: &str,
    ) -> Result<ResourceId, EnvironmentError> {
        let copy = self.environments.duplicate(id, name)?;
        self.select_environment(Some(copy))?;
        Ok(copy)
    }

    pub fn import_environment(&mut self, yaml: &str) -> Result<ResourceId, EnvironmentError> {
        let id = self.environments.import(yaml)?;
        self.select_environment(Some(id))?;
        Ok(id)
    }

    pub fn save_environment_variable(
        &mut self,
        id: ResourceId,
        old_key: Option<&str>,
        variable: VariableEntry,
    ) -> Result<(), EnvironmentError> {
        let mut doc = self.environments.get(id)?.clone();
        if let Some(key) = old_key {
            let existing = doc
                .variables
                .iter_mut()
                .find(|v| v.key == key)
                .ok_or_else(|| {
                    EnvironmentError::Invalid("Variable no longer exists; reload the editor".into())
                })?;
            *existing = variable;
        } else {
            doc.variables.push(variable);
        }
        self.save_environment(doc)
    }

    pub fn delete_environment_variable(
        &mut self,
        id: ResourceId,
        key: &str,
    ) -> Result<(), EnvironmentError> {
        let mut doc = self.environments.get(id)?.clone();
        doc.variables.retain(|v| v.key != key);
        self.save_environment(doc)
    }

    /// Reject overrides from a draft whose classification has not been saved.
    pub fn set_environment_current_checked(
        &mut self,
        id: ResourceId,
        key: &str,
        value: Option<String>,
        expected_type: VariableType,
        expected_secret: bool,
    ) -> Result<(), EnvironmentError> {
        let variable = self
            .environments
            .get(id)?
            .variables
            .iter()
            .find(|v| v.key == key)
            .ok_or_else(|| EnvironmentError::Invalid("Variable no longer exists".into()))?;
        if variable.value_type != expected_type || variable.is_secret != expected_secret {
            return Err(EnvironmentError::Invalid(
                "Save type and secret changes before applying a local value".into(),
            ));
        }
        self.set_environment_current(id, key, value)
    }

    /// Includes No environment in the stable name/ID ordered cycle.
    pub fn next_environment(&mut self) -> Result<(), EnvironmentError> {
        let ids: Vec<_> = self.environments.list().iter().map(|doc| doc.id).collect();
        let next = match self.active_environment_id {
            None => ids.first().copied(),
            Some(id) => ids
                .iter()
                .position(|candidate| *candidate == id)
                .and_then(|index| ids.get(index + 1).copied()),
        };
        self.select_environment(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsaved_secret_classification_cannot_persist_a_credential() {
        let path =
            std::env::temp_dir().join(format!("ps-env-classification-{}", ResourceId::new()));
        let mut ws = WorkspaceState::new(path.clone(), "Test");
        let id = ws.create_environment("Production").unwrap();
        ws.save_environment_variable(id, None, VariableEntry::new("token", ""))
            .unwrap();
        let error = ws
            .set_environment_current_checked(
                id,
                "token",
                Some("private-credential".into()),
                VariableType::String,
                true,
            )
            .unwrap_err();
        assert!(!error.to_string().contains("private-credential"));
        let local =
            std::fs::read_to_string(path.join(".packetsmith/environments.local.json")).unwrap();
        assert!(!local.contains("private-credential"));
        assert!(ws.environments.effective_variables(id).unwrap()[0]
            .value
            .is_empty());
        ws.save_environment_variable(id, Some("token"), VariableEntry::secret("token", ""))
            .unwrap();
        ws.set_environment_current_checked(
            id,
            "token",
            Some("private-credential".into()),
            VariableType::String,
            true,
        )
        .unwrap();
        assert_eq!(
            ws.environments.effective_variables(id).unwrap()[0].value,
            "private-credential"
        );
        assert!(
            !std::fs::read_to_string(path.join(".packetsmith/environments.local.json"))
                .unwrap()
                .contains("private-credential")
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn first_scan_keeps_values_created_in_the_current_session() {
        let path = std::env::temp_dir().join(format!("ps-env-first-scan-{}", ResourceId::new()));
        let mut ws = WorkspaceState::new(path.clone(), "Test");
        let id = ws.create_environment("Production").unwrap();
        ws.save_environment_variable(id, None, VariableEntry::secret("token", ""))
            .unwrap();
        ws.set_environment_current(id, "token", Some("session-secret".into()))
            .unwrap();
        ws.scan_resources().unwrap();
        assert_eq!(ws.active_environment_id, Some(id));
        assert_eq!(
            ws.environments.effective_variables(id).unwrap()[0].value,
            "session-secret"
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn editor_lifecycle_refreshes_resolver_and_preserves_identity() {
        let path = std::env::temp_dir().join(format!("ps-env-editor-{}", ResourceId::new()));
        let mut ws = WorkspaceState::new(path.clone(), "Test");
        let id = ws.create_environment("A").unwrap();
        ws.save_environment_variable(id, None, VariableEntry::new("host", "default"))
            .unwrap();
        ws.set_environment_current(id, "host", Some("local".into()))
            .unwrap();
        ws.rename_environment(id, "Renamed").unwrap();
        assert_eq!(ws.active_environment_id, Some(id));
        assert_eq!(
            ws.variable_ui.resolver().resolve_var("host").as_deref(),
            Some("local")
        );
        assert!(ws
            .save_environment_variable(id, None, VariableEntry::new("host", "duplicate"))
            .is_err());
        let copy = ws.clone_environment(id, "Z").unwrap();
        assert_ne!(id, copy);
        assert_eq!(
            ws.variable_ui.resolver().resolve_var("host").as_deref(),
            Some("default")
        );
        ws.next_environment().unwrap();
        assert_eq!(ws.active_environment_id, None);
        ws.next_environment().unwrap();
        assert_eq!(ws.active_environment_id, Some(id));
        ws.delete_environment_variable(id, "host").unwrap();
        assert!(ws.variable_ui.resolver().resolve_var("host").is_none());
        let imported = ws
            .import_environment(&ws.environments.export(copy).unwrap())
            .unwrap();
        assert_ne!(imported, copy);
        ws.delete_environment(imported).unwrap();
        assert_eq!(ws.active_environment_id, None);
        std::fs::remove_dir_all(path).unwrap();
    }
}
