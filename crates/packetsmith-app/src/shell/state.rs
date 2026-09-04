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

/// Master application state root.
#[derive(Debug)]
pub struct AppState {
    pub settings: AppSettings,
    pub window_manager: WindowManager,
    pub command_registry: Arc<CommandRegistry>,
    pub notification_manager: NotificationManager,
    pub shutdown_coordinator: ShutdownCoordinator,
    pub storage: Option<Arc<CacheStorage>>,
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
            active_workspace: None,
        }
    }

    pub fn set_active_workspace(&mut self, path: PathBuf, name: impl Into<String>) {
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
}

impl WorkspaceState {
    pub fn new(path: PathBuf, name: impl Into<String>) -> Self {
        Self {
            workspace_path: path,
            workspace_name: name.into(),
            active_environment_id: None,
            workbench: WorkbenchState::new(),
            collection_manager: None,
            resource_tree: None,
        }
    }

    /// Scans the workspace directory, initializes CollectionManager and builds the ResourceTree.
    pub fn scan_resources(&mut self) -> Result<(), ps_workspace::WorkspaceError> {
        let manager = CollectionManager::scan(&self.workspace_path)?;
        let tree = ResourceTree::from_manager(&manager);
        self.resource_tree = Some(tree);
        self.collection_manager = Some(manager);
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
