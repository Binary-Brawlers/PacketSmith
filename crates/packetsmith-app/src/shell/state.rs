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

/// State of an active open workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceState {
    pub workspace_path: PathBuf,
    pub workspace_name: String,
    pub active_environment_id: Option<ResourceId>,
    pub tabs: Vec<RequestTabState>,
    pub active_tab_index: usize,
}

impl WorkspaceState {
    pub fn new(path: PathBuf, name: impl Into<String>) -> Self {
        Self {
            workspace_path: path,
            workspace_name: name.into(),
            active_environment_id: None,
            tabs: Vec::new(),
            active_tab_index: 0,
        }
    }

    pub fn active_tab(&self) -> Option<&RequestTabState> {
        self.tabs.get(self.active_tab_index)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut RequestTabState> {
        self.tabs.get_mut(self.active_tab_index)
    }

    pub fn open_tab(&mut self, request: RequestDocument) {
        let tab = RequestTabState::new(request);
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
    }

    pub fn close_tab(&mut self, index: usize) -> Option<RequestTabState> {
        if index < self.tabs.len() {
            let removed = self.tabs.remove(index);
            if self.active_tab_index >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab_index = self.tabs.len() - 1;
            }
            Some(removed)
        } else {
            None
        }
    }
}

/// State of an individual request tab in the workbench.
#[derive(Debug, Clone)]
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
        assert_eq!(ws.tabs.len(), 0);

        let req = RequestDocument::new(
            "Users API",
            ProtocolRequest::Http(HttpRequestPayload {
                method: "GET".to_string(),
                url: "https://api.example.com/users".to_string(),
            }),
        );

        ws.open_tab(req);
        assert_eq!(ws.tabs.len(), 1);
        assert_eq!(ws.active_tab_index, 0);

        let closed = ws.close_tab(0);
        assert!(closed.is_some());
        assert_eq!(ws.tabs.len(), 0);
    }
}
