//! Application and workbench state hierarchies.

use std::path::PathBuf;
use std::sync::Arc;
use ps_command::CommandRegistry;
use ps_domain::{RequestDocument, ResourceId};
use ps_http::HttpResponse;
use ps_settings::AppSettings;
use ps_storage::CacheStorage;
use crate::shell::notifications::NotificationManager;
use crate::shell::shutdown::ShutdownCoordinator;
use crate::shell::window::{WindowManager, WindowState};
use crate::shell::variables::VariableUiController;

/// Master application state root.
#[derive(Debug)]
pub struct AppState {
    pub settings: AppSettings,
    pub window_manager: WindowManager,
    pub command_registry: Arc<CommandRegistry>,
    pub notification_manager: NotificationManager,
    pub shutdown_coordinator: ShutdownCoordinator,
    pub storage: Option<Arc<CacheStorage>>,
    pub vault: Option<Arc<ps_vault::Vault<ps_vault::OsSecretStore>>>,
    pub active_workspace: Option<WorkspaceState>,
}

impl AppState {
    pub fn new(window_state: WindowState) -> Self {
        Self {
            settings: AppSettings::default(),
            window_manager: WindowManager::new(window_state),
            command_registry: Arc::new(CommandRegistry::new()),
            notification_manager: NotificationManager::new(),
            shutdown_coordinator: ShutdownCoordinator::new(),
            storage: None,
            vault: None,
            active_workspace: None,
        }
    }

    /// Configure on workspace load using its stable manifest ID. Credential I/O
    /// remains lazy and must be scheduled on the background runtime.
    pub fn configure_vault(&mut self, workspace_id: ResourceId) -> Result<(), ps_vault::VaultError> {
        self.vault = Some(Arc::new(ps_vault::Vault::new(
            ps_vault::OsSecretStore::new(&workspace_id.to_string())?,
        )));
        Ok(())
    }

    pub fn set_active_workspace(&mut self, path: PathBuf, name: impl Into<String>) {
        self.vault = None;
        self.active_workspace = Some(WorkspaceState::new(path, name));
    }
}

use ps_workspace::{CollectionManager, ResourceTree};
use crate::shell::workbench::WorkbenchState;

/// State of an active open workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub workspace_path: PathBuf,
    pub workspace_name: String,
    pub active_environment_id: Option<ResourceId>,
    pub workbench: WorkbenchState,
    pub collection_manager: Option<CollectionManager>,
    pub resource_tree: Option<ResourceTree>,
    pub variable_ui: VariableUiController,
    pub environments: ps_workspace::EnvironmentManager,
}

impl WorkspaceState {
    pub fn new(path: PathBuf, name: impl Into<String>) -> Self {
        Self {
            environments: ps_workspace::EnvironmentManager::new(path.clone()),
            workspace_path: path,
            workspace_name: name.into(),
            active_environment_id: None,
            workbench: WorkbenchState::new(),
            collection_manager: None,
            resource_tree: None,
            variable_ui: VariableUiController::default(),
        }
    }

    /// Scans the workspace directory, initializes CollectionManager and builds the ResourceTree.
    pub fn scan_resources(&mut self) -> Result<(), ps_workspace::WorkspaceError> {
        let manager = CollectionManager::scan(&self.workspace_path)?;
        let mut environments = self.environments.clone();
        environments.reload()
            .map_err(|error| ps_workspace::WorkspaceError::InvalidEnvironment(error.to_string()))?;
        let active = if self.collection_manager.is_none() && self.environments.list().is_empty() {
            environments.restore_local_state()
                .map_err(|error| ps_workspace::WorkspaceError::InvalidEnvironment(error.to_string()))?
        } else { self.active_environment_id };
        self.active_environment_id = active;
        let tree = ResourceTree::from_manager(&manager);
        self.environments = environments;
        if self.active_environment_id.is_some_and(|id| self.environments.get(id).is_err()) {
            self.active_environment_id = None;
        }
        self.refresh_environment_variables()
            .map_err(|error| ps_workspace::WorkspaceError::InvalidEnvironment(error.to_string()))?;
        self.resource_tree = Some(tree);
        self.collection_manager = Some(manager);
        Ok(())
    }

    /// Select an environment atomically; None restores the lower-priority scopes.
    pub fn select_environment(&mut self, id: Option<ResourceId>) -> Result<(), ps_workspace::EnvironmentError> {
        if let Some(id) = id { self.environments.get(id)?; }
        self.environments.persist_local_state(id)?;
        self.active_environment_id = id;
        self.refresh_environment_variables()
    }

    pub fn save_environment(&mut self, doc: ps_domain::EnvironmentDocument) -> Result<(), ps_workspace::EnvironmentError> {
        self.environments.save(doc)?;
        self.refresh_environment_variables()?;
        self.environments.persist_local_state(self.active_environment_id)
    }

    pub fn set_environment_current(&mut self, id: ResourceId, key: &str, value: Option<String>) -> Result<(), ps_workspace::EnvironmentError> {
        let mut environments = self.environments.clone();
        environments.set_current(id, key, value)?;
        environments.persist_local_state(self.active_environment_id)?;
        self.environments = environments;
        self.refresh_environment_variables()
    }

    pub fn delete_environment(&mut self, id: ResourceId) -> Result<(), ps_workspace::EnvironmentError> {
        self.environments.delete(id)?;
        if self.active_environment_id == Some(id) { self.active_environment_id = None; }
        self.refresh_environment_variables()?;
        self.environments.persist_local_state(self.active_environment_id)
    }

    /// Refresh after editing defaults or local overrides, and before execution.
    pub fn refresh_environment_variables(&mut self) -> Result<(), ps_workspace::EnvironmentError> {
        let entries = if let Some(id) = self.active_environment_id {
            let doc = self.environments.get(id)?;
            let source = ps_variable::VariableSource::new(ps_domain::VariableScope::Environment, doc.name.clone()).with_resource_id(id);
            self.environments.effective_variables(id)?.into_iter().map(|v| {
                let secret = v.is_secret || v.value_type == ps_domain::VariableType::SecretReference;
                let definition = ps_variable::VariableDefinition::new(v.key, v.value, source.clone());
                if secret { definition.secret() } else { definition }
            }).collect::<Vec<_>>()
        } else { Vec::new() };
        let resolver = self.variable_ui.resolver_mut();
        resolver.clear_scope(ps_domain::VariableScope::Environment);
        for definition in entries { resolver.insert(ps_domain::VariableScope::Environment, definition); }
        Ok(())
    }

    pub fn active_tab(&self) -> Option<&RequestTabState> {
        self.workbench.active_pane().and_then(|p| p.active_tab())
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut RequestTabState> {
        let active_id = self.workbench.active_pane_id.clone();
        self.workbench.get_pane_mut(&active_id).and_then(|p| p.active_tab_mut())
    }

    pub fn open_tab(&mut self, request: RequestDocument) {
        self.workbench.open_request(request);
    }

    pub fn close_tab(&mut self, index: usize) -> Option<RequestTabState> {
        let active_id = self.workbench.active_pane_id.clone();
        self.workbench.force_close_tab(&active_id, index)
    }

    /// Persists open workbench tabs to SQLite cache.
    pub fn persist_tabs(&self, storage: &CacheStorage, workspace_id: &str) -> Result<(), ps_storage::StorageError> {
        let records = self.workbench.to_tab_state_records(workspace_id);
        storage.save_open_tabs(workspace_id, &records)?;
        Ok(())
    }

    /// Restores open workbench tabs from SQLite cache.
    pub fn restore_tabs(&mut self, storage: &CacheStorage, workspace_id: &str) -> Result<(), ps_storage::StorageError> {
        if let Some(ref manager) = self.collection_manager {
            let records = storage.load_open_tabs(workspace_id)?;
            self.workbench.restore_from_records(&records, manager);
        }
        Ok(())
    }
}

/// State of an individual request tab in the workbench.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestTabState {
    pub tab_id: ResourceId,
    pub request: RequestDocument,
    pub is_dirty: bool,
    pub is_pinned: bool,
    pub is_executing: bool,
    pub latest_response: Option<HttpResponse>,
}

impl RequestTabState {
    pub fn new(request: RequestDocument) -> Self {
        Self {
            tab_id: ResourceId::new(),
            request,
            is_dirty: false,
            is_pinned: false,
            is_executing: false,
            latest_response: None,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_domain::{HttpRequestPayload, ProtocolRequest};

    #[test]
    fn environment_selection_and_local_values_survive_workspace_restart() {
        let path = std::env::temp_dir().join(format!("ps-env-restart-{}", ResourceId::new()));
        let mut ws = WorkspaceState::new(path.clone(), "Workspace");
        let id = ws.environments.create("Staging").unwrap();
        let mut doc = ws.environments.get(id).unwrap().clone();
        doc.variables.push(ps_domain::VariableEntry::new("host", "default"));
        ws.save_environment(doc).unwrap();
        ws.select_environment(Some(id)).unwrap();
        ws.set_environment_current(id, "host", Some("local".into())).unwrap();
        let mut restored = WorkspaceState::new(path.clone(), "Workspace");
        restored.scan_resources().unwrap();
        assert_eq!(restored.active_environment_id, Some(id));
        assert_eq!(restored.variable_ui.resolver().resolve_var("host").as_deref(), Some("local"));
        restored.scan_resources().unwrap();
        assert_eq!(restored.variable_ui.resolver().resolve_var("host").as_deref(), Some("local"));
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn environment_switch_clears_stale_values_and_preserves_globals() {
        let path = std::env::temp_dir().join(format!("ps-env-state-{}", ResourceId::new()));
        let mut ws = WorkspaceState::new(path.clone(), "Workspace");
        let a = ws.environments.create("A").unwrap();
        let b = ws.environments.create("B").unwrap();
        let mut doc = ws.environments.get(a).unwrap().clone();
        doc.variables.push(ps_domain::VariableEntry::new("host", "environment"));
        ws.save_environment(doc).unwrap();
        ws.variable_ui.resolver_mut().insert(ps_domain::VariableScope::Global,
            ps_variable::VariableDefinition::new("host", "global", ps_variable::VariableSource::new(ps_domain::VariableScope::Global, "Workspace")));
        ws.select_environment(Some(a)).unwrap();
        assert_eq!(ws.variable_ui.resolver().resolve_var("host").as_deref(), Some("environment"));
        ws.set_environment_current(a, "host", Some("local".into())).unwrap();
        assert_eq!(ws.variable_ui.resolver().resolve_var("host").as_deref(), Some("local"));
        assert!(ws.select_environment(Some(ResourceId::new())).is_err());
        assert_eq!(ws.active_environment_id, Some(a));
        ws.select_environment(Some(b)).unwrap();
        assert_eq!(ws.variable_ui.resolver().resolve_var("host").as_deref(), Some("global"));
        ws.delete_environment(b).unwrap();
        assert_eq!(ws.active_environment_id, None);
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn test_workspace_tab_lifecycle() {
        let mut ws = WorkspaceState::new(PathBuf::from("/tmp/test"), "Test WS");
        assert!(ws.active_tab().is_none());

        let req = RequestDocument::new(
            "Users API",
            ProtocolRequest::Http(HttpRequestPayload {
                method: "GET".to_string(),
                url: "https://api.example.com/users".to_string(),
            }),
        );

        ws.open_tab(req);
        assert!(ws.active_tab().is_some());
        assert_eq!(ws.active_tab().unwrap().request.name, "Users API");

        let closed = ws.close_tab(0);
        assert!(closed.is_some());
        assert!(ws.active_tab().is_none());
    }
}
