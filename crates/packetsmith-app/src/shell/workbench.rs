//! Multi-pane workbench layout, tab lifecycle, closed-tab restoration, and layout persistence.
//!
//! Provides the workbench state managing split panes (horizontal & vertical), tab pinning,
//! tab duplication, reordering, dirty-close protection, and LIFO closed-tab restoration (Cmd+Shift+T).

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use ps_domain::{RequestDocument, ResourceId};
use ps_storage::TabStateRecord;
use ps_ui_components::SplitOrientation;
use ps_workspace::CollectionManager;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::shell::state::RequestTabState;

pub const DEFAULT_PANE_ID: &str = "pane-main";
pub const MAX_CLOSED_TABS_HISTORY: usize = 50;

#[derive(Error, Debug)]
pub enum CloseTabError {
    #[error("Tab has unsaved changes: {0}")]
    UnsavedChanges(ResourceId),
    #[error("Pane not found: {0}")]
    PaneNotFound(String),
    #[error("Tab index out of bounds: {0}")]
    IndexOutOfBounds(usize),
}

/// Binary split tree modeling arbitrary nested horizontal and vertical workbench split panes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SplitTree {
    Leaf { pane_id: String },
    Split {
        orientation: SplitOrientation,
        ratio: f32, // 0.0 to 1.0
        first: Box<SplitTree>,
        second: Box<SplitTree>,
    },
}

impl Default for SplitTree {
    fn default() -> Self {
        Self::Leaf {
            pane_id: DEFAULT_PANE_ID.to_string(),
        }
    }
}

/// History entry for a closed tab to enable "Reopen Closed Tab" (Cmd+Shift+T).
#[derive(Debug, Clone)]
pub struct ClosedTabInfo {
    pub pane_id: String,
    pub tab_state: RequestTabState,
    pub closed_at: DateTime<Utc>,
}

/// An individual editor pane holding a collection of tabs and an active tab index.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchPane {
    pub id: String,
    pub tabs: Vec<RequestTabState>,
    pub active_tab_index: usize,
}

impl WorkbenchPane {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
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
}

/// Comprehensive workbench state coordinator managing splits, tabs, and recovery.
#[derive(Debug, Clone)]
pub struct WorkbenchState {
    pub panes: HashMap<String, WorkbenchPane>,
    pub active_pane_id: String,
    pub split_tree: SplitTree,
    pub closed_tabs: Vec<ClosedTabInfo>,
    pub autosave_enabled: bool,
}

impl Default for WorkbenchState {
    fn default() -> Self {
        let mut panes = HashMap::new();
        panes.insert(DEFAULT_PANE_ID.to_string(), WorkbenchPane::new(DEFAULT_PANE_ID));
        Self {
            panes,
            active_pane_id: DEFAULT_PANE_ID.to_string(),
            split_tree: SplitTree::default(),
            closed_tabs: Vec::new(),
            autosave_enabled: false,
        }
    }
}

impl WorkbenchState {
    pub fn new() -> Self {
        Self::default()
    }

    // -----------------------------------------------------------------------
    // Pane Queries & Manipulation
    // -----------------------------------------------------------------------

    pub fn active_pane(&self) -> Option<&WorkbenchPane> {
        self.panes.get(&self.active_pane_id)
    }

    pub fn active_pane_mut(&mut self) -> Option<&mut WorkbenchPane> {
        self.panes.get_mut(&self.active_pane_id)
    }

    pub fn get_pane(&self, pane_id: &str) -> Option<&WorkbenchPane> {
        self.panes.get(pane_id)
    }

    pub fn get_pane_mut(&mut self, pane_id: &str) -> Option<&mut WorkbenchPane> {
        self.panes.get_mut(pane_id)
    }

    pub fn set_active_pane(&mut self, pane_id: impl Into<String>) -> bool {
        let id = pane_id.into();
        if self.panes.contains_key(&id) {
            self.active_pane_id = id;
            true
        } else {
            false
        }
    }

    /// Splits an existing pane horizontally or vertically, dividing available workbench space.
    pub fn split_pane(
        &mut self,
        target_pane_id: &str,
        orientation: SplitOrientation,
        ratio: f32,
    ) -> Result<String, CloseTabError> {
        if !self.panes.contains_key(target_pane_id) {
            return Err(CloseTabError::PaneNotFound(target_pane_id.to_string()));
        }

        let new_pane_id = format!("pane-{}", uuid::Uuid::new_v4());
        self.panes.insert(new_pane_id.clone(), WorkbenchPane::new(&new_pane_id));

        // Update split tree replacing target_pane leaf with Split node
        self.split_tree = Self::insert_split(
            self.split_tree.clone(),
            target_pane_id,
            &new_pane_id,
            orientation,
            ratio,
        );

        self.active_pane_id = new_pane_id.clone();
        Ok(new_pane_id)
    }

    fn insert_split(
        tree: SplitTree,
        target_id: &str,
        new_id: &str,
        orientation: SplitOrientation,
        ratio: f32,
    ) -> SplitTree {
        match tree {
            SplitTree::Leaf { pane_id } => {
                if pane_id == target_id {
                    SplitTree::Split {
                        orientation,
                        ratio,
                        first: Box::new(SplitTree::Leaf { pane_id }),
                        second: Box::new(SplitTree::Leaf {
                            pane_id: new_id.to_string(),
                        }),
                    }
                } else {
                    SplitTree::Leaf { pane_id }
                }
            }
            SplitTree::Split {
                orientation: o,
                ratio: r,
                first,
                second,
            } => SplitTree::Split {
                orientation: o,
                ratio: r,
                first: Box::new(Self::insert_split(*first, target_id, new_id, orientation, ratio)),
                second: Box::new(Self::insert_split(*second, target_id, new_id, orientation, ratio)),
            },
        }
    }

    /// Closes a pane, transferring any open tabs to the main pane.
    pub fn close_pane(&mut self, pane_id: &str) -> bool {
        if self.panes.len() <= 1 || !self.panes.contains_key(pane_id) {
            return false;
        }

        let removed = self.panes.remove(pane_id).unwrap();

        // Transfer remaining tabs to an existing pane
        let fallback_pane_id = self.panes.keys().next().cloned().unwrap();
        if let Some(fallback_pane) = self.panes.get_mut(&fallback_pane_id) {
            for tab in removed.tabs {
                fallback_pane.tabs.push(tab);
            }
        }

        if self.active_pane_id == pane_id {
            self.active_pane_id = fallback_pane_id;
        }

        // Simplify split tree removing the closed leaf
        self.split_tree = Self::prune_split_tree(self.split_tree.clone(), pane_id);
        true
    }

    fn prune_split_tree(tree: SplitTree, removed_id: &str) -> SplitTree {
        match tree {
            SplitTree::Leaf { pane_id } => SplitTree::Leaf { pane_id },
            SplitTree::Split {
                orientation,
                ratio,
                first,
                second,
            } => {
                if let SplitTree::Leaf { pane_id } = &*first {
                    if pane_id == removed_id {
                        return *second;
                    }
                }
                if let SplitTree::Leaf { pane_id } = &*second {
                    if pane_id == removed_id {
                        return *first;
                    }
                }

                SplitTree::Split {
                    orientation,
                    ratio,
                    first: Box::new(Self::prune_split_tree(*first, removed_id)),
                    second: Box::new(Self::prune_split_tree(*second, removed_id)),
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Tab Lifecycle Operations
    // -----------------------------------------------------------------------

    /// Opens a request in the active pane. If already open in this pane, selects it.
    pub fn open_request(&mut self, request: RequestDocument) -> usize {
        let req_id = request.id;
        let pane = self.panes.get_mut(&self.active_pane_id).unwrap();

        // Check if already open
        for (i, tab) in pane.tabs.iter().enumerate() {
            if tab.request.id == req_id {
                pane.active_tab_index = i;
                return i;
            }
        }

        let tab = RequestTabState::new(request);
        pane.tabs.push(tab);
        pane.active_tab_index = pane.tabs.len() - 1;
        pane.active_tab_index
    }

    /// Closes a tab with dirty protection: if dirty and autosave is disabled, returns an error.
    pub fn close_tab(
        &mut self,
        pane_id: &str,
        tab_index: usize,
    ) -> Result<RequestTabState, CloseTabError> {
        let pane = self
            .panes
            .get_mut(pane_id)
            .ok_or_else(|| CloseTabError::PaneNotFound(pane_id.to_string()))?;

        if tab_index >= pane.tabs.len() {
            return Err(CloseTabError::IndexOutOfBounds(tab_index));
        }

        // Check unsaved changes
        if pane.tabs[tab_index].is_dirty && !self.autosave_enabled {
            return Err(CloseTabError::UnsavedChanges(pane.tabs[tab_index].request.id));
        }

        let removed = pane.tabs.remove(tab_index);
        if tab_index < pane.active_tab_index {
            pane.active_tab_index = pane.active_tab_index.saturating_sub(1);
        } else if pane.active_tab_index >= pane.tabs.len() && !pane.tabs.is_empty() {
            pane.active_tab_index = pane.tabs.len() - 1;
        }

        // Record in closed tabs history stack for Cmd+Shift+T
        self.closed_tabs.push(ClosedTabInfo {
            pane_id: pane_id.to_string(),
            tab_state: removed.clone(),
            closed_at: Utc::now(),
        });
        if self.closed_tabs.len() > MAX_CLOSED_TABS_HISTORY {
            self.closed_tabs.remove(0);
        }

        Ok(removed)
    }

    /// Closes a tab immediately, discarding any unsaved changes without checking dirty status.
    pub fn force_close_tab(&mut self, pane_id: &str, tab_index: usize) -> Option<RequestTabState> {
        let pane = self.panes.get_mut(pane_id)?;
        if tab_index < pane.tabs.len() {
            let removed = pane.tabs.remove(tab_index);
            if tab_index < pane.active_tab_index {
                pane.active_tab_index = pane.active_tab_index.saturating_sub(1);
            } else if pane.active_tab_index >= pane.tabs.len() && !pane.tabs.is_empty() {
                pane.active_tab_index = pane.tabs.len() - 1;
            }
            self.closed_tabs.push(ClosedTabInfo {
                pane_id: pane_id.to_string(),
                tab_state: removed.clone(),
                closed_at: Utc::now(),
            });
            Some(removed)
        } else {
            None
        }
    }

    /// Reopens the most recently closed tab (Cmd+Shift+T / LIFO stack).
    pub fn reopen_closed_tab(&mut self) -> Option<&RequestTabState> {
        let closed = self.closed_tabs.pop()?;
        let target_pane_id = if self.panes.contains_key(&closed.pane_id) {
            closed.pane_id
        } else {
            self.active_pane_id.clone()
        };

        let pane = self.panes.get_mut(&target_pane_id)?;
        pane.tabs.push(closed.tab_state);
        pane.active_tab_index = pane.tabs.len() - 1;
        pane.tabs.last()
    }

    /// Pins a tab, preventing accidental closing or tab overflow pruning.
    pub fn pin_tab(&mut self, pane_id: &str, tab_index: usize) -> bool {
        if let Some(pane) = self.panes.get_mut(pane_id) {
            if let Some(tab) = pane.tabs.get_mut(tab_index) {
                tab.is_pinned = true;
                return true;
            }
        }
        false
    }

    /// Unpins a tab.
    pub fn unpin_tab(&mut self, pane_id: &str, tab_index: usize) -> bool {
        if let Some(pane) = self.panes.get_mut(pane_id) {
            if let Some(tab) = pane.tabs.get_mut(tab_index) {
                tab.is_pinned = false;
                return true;
            }
        }
        false
    }

    /// Duplicates an existing tab in the same pane.
    pub fn duplicate_tab(&mut self, pane_id: &str, tab_index: usize) -> Option<ResourceId> {
        let pane = self.panes.get_mut(pane_id)?;
        if tab_index < pane.tabs.len() {
            let original = &pane.tabs[tab_index];
            let mut cloned_req = original.request.clone();
            cloned_req.id = ResourceId::new();
            cloned_req.name = format!("{} (Copy)", cloned_req.name);

            let new_tab = RequestTabState::new(cloned_req);
            let new_id = new_tab.request.id;
            pane.tabs.insert(tab_index + 1, new_tab);
            pane.active_tab_index = tab_index + 1;
            Some(new_id)
        } else {
            None
        }
    }

    /// Reorders a tab within its parent pane from `from_index` to `to_index`.
    pub fn reorder_tabs(&mut self, pane_id: &str, from_index: usize, to_index: usize) -> bool {
        let pane = match self.panes.get_mut(pane_id) {
            Some(p) => p,
            None => return false,
        };

        if from_index >= pane.tabs.len() || to_index >= pane.tabs.len() {
            return false;
        }

        let tab = pane.tabs.remove(from_index);
        pane.tabs.insert(to_index, tab);
        pane.active_tab_index = to_index;
        true
    }

    /// Moves a tab between split panes.
    pub fn move_tab_to_pane(
        &mut self,
        from_pane_id: &str,
        tab_index: usize,
        to_pane_id: &str,
    ) -> bool {
        if from_pane_id == to_pane_id {
            return false;
        }

        let tab = {
            let from_pane = match self.panes.get_mut(from_pane_id) {
                Some(p) => p,
                None => return false,
            };
            if tab_index >= from_pane.tabs.len() {
                return false;
            }
            let t = from_pane.tabs.remove(tab_index);
            if from_pane.active_tab_index >= from_pane.tabs.len() && !from_pane.tabs.is_empty() {
                from_pane.active_tab_index = from_pane.tabs.len() - 1;
            }
            t
        };

        let to_pane = match self.panes.get_mut(to_pane_id) {
            Some(p) => p,
            None => return false,
        };
        to_pane.tabs.push(tab);
        to_pane.active_tab_index = to_pane.tabs.len() - 1;
        true
    }

    // -----------------------------------------------------------------------
    // Persistence Serialization
    // -----------------------------------------------------------------------

    /// Generates SQLite `TabStateRecord` entries for all open tabs across all panes.
    pub fn to_tab_state_records(&self, workspace_id: &str) -> Vec<TabStateRecord> {
        let mut records = Vec::new();
        for (pane_id, pane) in &self.panes {
            for (order, tab) in pane.tabs.iter().enumerate() {
                records.push(TabStateRecord {
                    id: tab.tab_id.to_string(),
                    workspace_id: workspace_id.to_string(),
                    resource_id: tab.request.id,
                    tab_order: order as i32,
                    is_pinned: tab.is_pinned,
                    pane_id: Some(pane_id.clone()),
                });
            }
        }
        records
    }

    /// Restores open tabs and panes from saved SQLite records using CollectionManager requests.
    pub fn restore_from_records(
        &mut self,
        records: &[TabStateRecord],
        manager: &CollectionManager,
    ) {
        if records.is_empty() {
            return;
        }

        for record in records {
            if let Some(req) = manager.requests().get(&record.resource_id) {
                let target_pane_id = record.pane_id.as_deref().unwrap_or(DEFAULT_PANE_ID);
                let pane = self
                    .panes
                    .entry(target_pane_id.to_string())
                    .or_insert_with(|| WorkbenchPane::new(target_pane_id));

                let mut tab = RequestTabState::new(req.clone());
                tab.is_pinned = record.is_pinned;
                pane.tabs.push(tab);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_domain::{HttpRequestPayload, ProtocolRequest};

    fn make_test_request(name: &str) -> RequestDocument {
        RequestDocument::new(
            name,
            ProtocolRequest::Http(HttpRequestPayload {
                method: "GET".to_string(),
                url: format!("https://api.test.com/{}", name),
            }),
        )
    }

    #[test]
    fn test_workbench_tabs_lifecycle() {
        let mut wb = WorkbenchState::new();
        let r1 = make_test_request("r1");
        let r2 = make_test_request("r2");

        wb.open_request(r1.clone());
        wb.open_request(r2.clone());

        let pane = wb.active_pane().unwrap();
        assert_eq!(pane.tabs.len(), 2);
        assert_eq!(pane.active_tab_index, 1);

        // Pin first tab
        wb.pin_tab(DEFAULT_PANE_ID, 0);
        assert!(wb.active_pane().unwrap().tabs[0].is_pinned);

        // Reorder tabs
        wb.reorder_tabs(DEFAULT_PANE_ID, 0, 1);
        assert_eq!(wb.active_pane().unwrap().tabs[0].request.id, r2.id);

        // Close tab
        let closed = wb.close_tab(DEFAULT_PANE_ID, 0).expect("close tab");
        assert_eq!(closed.request.id, r2.id);
        assert_eq!(wb.active_pane().unwrap().tabs.len(), 1);

        // Reopen closed tab (Cmd+Shift+T)
        let reopened = wb.reopen_closed_tab().expect("reopened");
        assert_eq!(reopened.request.id, r2.id);
        assert_eq!(wb.active_pane().unwrap().tabs.len(), 2);
    }

    #[test]
    fn test_workbench_dirty_protection() {
        let mut wb = WorkbenchState::new();
        let r1 = make_test_request("dirty_test");
        wb.open_request(r1);

        wb.active_pane_mut().unwrap().tabs[0].mark_dirty();

        // Attempt closing dirty tab without autosave
        let res = wb.close_tab(DEFAULT_PANE_ID, 0);
        assert!(res.is_err());
        match res.unwrap_err() {
            CloseTabError::UnsavedChanges(_) => {}
            other => panic!("Expected UnsavedChanges, got {:?}", other),
        }

        // Force close works
        let force_closed = wb.force_close_tab(DEFAULT_PANE_ID, 0);
        assert!(force_closed.is_some());
        assert_eq!(wb.active_pane().unwrap().tabs.len(), 0);
    }

    #[test]
    fn test_workbench_split_panes() {
        let mut wb = WorkbenchState::new();
        let r1 = make_test_request("pane1_req");
        wb.open_request(r1);

        // Split vertically
        let new_pane = wb
            .split_pane(DEFAULT_PANE_ID, SplitOrientation::Vertical, 0.5)
            .expect("split");
        assert_eq!(wb.panes.len(), 2);

        // Move tab from main to new pane
        let moved = wb.move_tab_to_pane(DEFAULT_PANE_ID, 0, &new_pane);
        assert!(moved);
        assert_eq!(wb.get_pane(DEFAULT_PANE_ID).unwrap().tabs.len(), 0);
        assert_eq!(wb.get_pane(&new_pane).unwrap().tabs.len(), 1);

        // Close pane merges tab back
        wb.close_pane(&new_pane);
        assert_eq!(wb.panes.len(), 1);
        assert_eq!(wb.active_pane().unwrap().tabs.len(), 1);
    }
}
