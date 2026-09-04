//! Hierarchical resource tree, UI representation, breadcrumb resolution, and Quick Open search.
//!
//! Converts raw collection, folder, and request documents into an expandable, navigable tree
//! structure suitable for sidebar components and fuzzy command search.

use std::collections::HashSet;
use ps_domain::{ProtocolRequest, ResourceId};
use serde::{Deserialize, Serialize};
use crate::collection_manager::{CollectionManager, ResourceParent};

/// Request leaf item in the resource tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestNode {
    pub id: ResourceId,
    pub name: String,
    pub method: String,
    pub url: String,
    pub examples_count: usize,
}

/// Nested folder branch in the resource tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderNode {
    pub id: ResourceId,
    pub collection_id: ResourceId,
    pub parent_id: Option<ResourceId>,
    pub name: String,
    pub folders: Vec<FolderNode>,
    pub requests: Vec<RequestNode>,
    pub is_expanded: bool,
}

/// Top-level collection node in the resource tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionNode {
    pub id: ResourceId,
    pub name: String,
    pub description: Option<String>,
    pub folders: Vec<FolderNode>,
    pub requests: Vec<RequestNode>,
    pub is_expanded: bool,
}

/// A breadcrumb segment identifying a location in the hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreadcrumbItem {
    pub id: ResourceId,
    pub name: String,
    pub kind: String, // "collection", "folder", "request"
}

/// A match candidate from Quick Open search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuickOpenResult {
    pub resource_id: ResourceId,
    pub name: String,
    pub method: Option<String>,
    pub url: Option<String>,
    pub breadcrumb_path: String,
    pub score: usize,
}

/// Complete hierarchical resource tree representing the sidebar workbench structure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceTree {
    pub collections: Vec<CollectionNode>,
    pub expanded_ids: HashSet<ResourceId>,
    pub selected_id: Option<ResourceId>,
}

impl ResourceTree {
    /// Builds the resource tree from the in-memory state of a CollectionManager.
    pub fn from_manager(manager: &CollectionManager) -> Self {
        let mut tree = Self::default();
        let mut collection_list: Vec<_> = manager.collections().values().cloned().collect();
        collection_list.sort_by_key(|a| a.name.to_lowercase());

        for col in collection_list {
            let col_id = col.id;
            let mut col_node = CollectionNode {
                id: col_id,
                name: col.name.clone(),
                description: col.description.clone(),
                folders: Vec::new(),
                requests: Vec::new(),
                is_expanded: false,
            };

            // Build top-level folders within this collection
            let mut top_folders: Vec<_> = manager
                .folders()
                .values()
                .filter(|f| f.collection_id == col_id && f.parent_id.is_none())
                .cloned()
                .collect();
            top_folders.sort_by_key(|a| a.name.to_lowercase());

            for folder in top_folders {
                col_node.folders.push(Self::build_folder_node(manager, &folder));
            }

            // Build top-level requests directly under this collection
            for req in manager.requests().values() {
                if let Some(ResourceParent::Collection(c_id)) = manager.get_resource_parent(&req.id) {
                    if c_id == col_id {
                        col_node.requests.push(Self::build_request_node(req));
                    }
                }
            }
            col_node.requests.sort_by_key(|a| a.name.to_lowercase());

            tree.collections.push(col_node);
        }

        tree
    }

    fn build_folder_node(manager: &CollectionManager, folder: &ps_domain::FolderDocument) -> FolderNode {
        let mut node = FolderNode {
            id: folder.id,
            collection_id: folder.collection_id,
            parent_id: folder.parent_id,
            name: folder.name.clone(),
            folders: Vec::new(),
            requests: Vec::new(),
            is_expanded: false,
        };

        // Subfolders
        let mut sub_folders: Vec<_> = manager
            .folders()
            .values()
            .filter(|f| f.parent_id == Some(folder.id))
            .cloned()
            .collect();
        sub_folders.sort_by_key(|a| a.name.to_lowercase());

        for sub in sub_folders {
            node.folders.push(Self::build_folder_node(manager, &sub));
        }

        // Requests in this folder
        for req in manager.requests().values() {
            if let Some(ResourceParent::Folder(f_id)) = manager.get_resource_parent(&req.id) {
                if f_id == folder.id {
                    node.requests.push(Self::build_request_node(req));
                }
            }
        }
        node.requests.sort_by_key(|a| a.name.to_lowercase());

        node
    }

    fn build_request_node(req: &ps_domain::RequestDocument) -> RequestNode {
        let (method, url) = match &req.protocol {
            ProtocolRequest::Http(h) => (h.method.clone(), h.url.clone()),
            ProtocolRequest::GraphQl(g) => ("GRAPHQL".to_string(), g.endpoint.clone()),
            ProtocolRequest::Grpc(g) => ("GRPC".to_string(), format!("{}/{}", g.service, g.method)),
            ProtocolRequest::WebSocket(w) => ("WS".to_string(), w.url.clone()),
            ProtocolRequest::SocketIo(s) => ("SOCKET.IO".to_string(), s.url.clone()),
            ProtocolRequest::Mqtt(m) => ("MQTT".to_string(), format!("{}/{}", m.broker_url, m.topic)),
            ProtocolRequest::Mcp(m) => ("MCP".to_string(), m.tool_name.clone()),
            ProtocolRequest::Ai(a) => ("AI".to_string(), format!("{}:{}", a.provider, a.model)),
            ProtocolRequest::Soap(s) => ("SOAP".to_string(), s.endpoint.clone()),
        };

        RequestNode {
            id: req.id,
            name: req.name.clone(),
            method,
            url,
            examples_count: req.examples.len(),
        }
    }

    // -----------------------------------------------------------------------
    // Tree Expansion & Selection
    // -----------------------------------------------------------------------

    pub fn expand_node(&mut self, id: ResourceId) {
        self.expanded_ids.insert(id);
        self.apply_expansion();
    }

    pub fn collapse_node(&mut self, id: ResourceId) {
        self.expanded_ids.remove(&id);
        self.apply_expansion();
    }

    pub fn toggle_expand(&mut self, id: ResourceId) {
        if self.expanded_ids.contains(&id) {
            self.expanded_ids.remove(&id);
        } else {
            self.expanded_ids.insert(id);
        }
        self.apply_expansion();
    }

    pub fn select_node(&mut self, id: Option<ResourceId>) {
        self.selected_id = id;
    }

    fn apply_expansion(&mut self) {
        for col in &mut self.collections {
            col.is_expanded = self.expanded_ids.contains(&col.id);
            for folder in &mut col.folders {
                Self::apply_folder_expansion(folder, &self.expanded_ids);
            }
        }
    }

    fn apply_folder_expansion(folder: &mut FolderNode, expanded: &HashSet<ResourceId>) {
        folder.is_expanded = expanded.contains(&folder.id);
        for sub in &mut folder.folders {
            Self::apply_folder_expansion(sub, expanded);
        }
    }

    // -----------------------------------------------------------------------
    // Breadcrumbs Resolution
    // -----------------------------------------------------------------------

    /// Resolves the breadcrumb path from the root collection down to the given resource.
    pub fn resolve_breadcrumbs(
        &self,
        manager: &CollectionManager,
        target_id: ResourceId,
    ) -> Vec<BreadcrumbItem> {
        let mut path = Vec::new();

        // 1. Check if target is a collection
        if let Some(col) = manager.collections().get(&target_id) {
            path.push(BreadcrumbItem {
                id: col.id,
                name: col.name.clone(),
                kind: "collection".to_string(),
            });
            return path;
        }

        // 2. Check if target is a folder
        if let Some(folder) = manager.folders().get(&target_id) {
            path.push(BreadcrumbItem {
                id: folder.id,
                name: folder.name.clone(),
                kind: "folder".to_string(),
            });
            let mut curr_parent = folder.parent_id;
            while let Some(parent_id) = curr_parent {
                if let Some(p_folder) = manager.folders().get(&parent_id) {
                    path.push(BreadcrumbItem {
                        id: p_folder.id,
                        name: p_folder.name.clone(),
                        kind: "folder".to_string(),
                    });
                    curr_parent = p_folder.parent_id;
                } else {
                    break;
                }
            }
            if let Some(col) = manager.collections().get(&folder.collection_id) {
                path.push(BreadcrumbItem {
                    id: col.id,
                    name: col.name.clone(),
                    kind: "collection".to_string(),
                });
            }
            path.reverse();
            return path;
        }

        // 3. Check if target is a request
        if let Some(req) = manager.requests().get(&target_id) {
            path.push(BreadcrumbItem {
                id: req.id,
                name: req.name.clone(),
                kind: "request".to_string(),
            });

            if let Some(parent) = manager.get_resource_parent(&target_id) {
                match parent {
                    ResourceParent::Folder(folder_id) => {
                        let folder_crumbs = self.resolve_breadcrumbs(manager, folder_id);
                        let mut full_path = folder_crumbs;
                        full_path.extend(path);
                        return full_path;
                    }
                    ResourceParent::Collection(col_id) => {
                        if let Some(col) = manager.collections().get(&col_id) {
                            path.push(BreadcrumbItem {
                                id: col.id,
                                name: col.name.clone(),
                                kind: "collection".to_string(),
                            });
                        }
                    }
                }
            }
            path.reverse();
            return path;
        }

        path
    }

    // -----------------------------------------------------------------------
    // Quick Open Search
    // -----------------------------------------------------------------------

    /// Fast search across requests, matching name or URL substring with ranking.
    pub fn quick_open(&self, manager: &CollectionManager, query: &str, limit: usize) -> Vec<QuickOpenResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let q_lower = query.to_lowercase();
        let mut results = Vec::new();

        for req in manager.requests().values() {
            let name_lower = req.name.to_lowercase();
            let (method, url) = match &req.protocol {
                ProtocolRequest::Http(h) => (Some(h.method.clone()), Some(h.url.clone())),
                ProtocolRequest::GraphQl(g) => (Some("GRAPHQL".into()), Some(g.endpoint.clone())),
                _ => (None, None),
            };

            let url_str = url.as_deref().unwrap_or("").to_lowercase();

            let name_match = name_lower.contains(&q_lower);
            let url_match = url_str.contains(&q_lower);

            if name_match || url_match {
                let mut score = 0;
                if name_lower.starts_with(&q_lower) {
                    score += 100;
                } else if name_match {
                    score += 50;
                }
                if url_match {
                    score += 30;
                }

                let crumbs = self.resolve_breadcrumbs(manager, req.id);
                let breadcrumb_path = crumbs
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" > ");

                results.push(QuickOpenResult {
                    resource_id: req.id,
                    name: req.name.clone(),
                    method,
                    url,
                    breadcrumb_path,
                    score,
                });
            }
        }

        results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
        if results.len() > limit {
            results.truncate(limit);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_domain::HttpRequestPayload;

    fn temp_workspace(prefix: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("packetsmith-test-{}-{}", prefix, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_resource_tree_and_breadcrumbs() {
        let root = temp_workspace("tree");
        let mut manager = CollectionManager::scan(&root).expect("scan");

        let col = manager.create_collection("Github API", None).expect("create col");
        let folder = manager.create_folder(col.id, None, "Repositories").expect("create folder");
        let req = manager
            .create_request(
                col.id,
                Some(folder.id),
                "List Repos",
                ProtocolRequest::Http(HttpRequestPayload {
                    method: "GET".to_string(),
                    url: "https://api.github.com/user/repos".to_string(),
                }),
            )
            .expect("create req");

        let tree = ResourceTree::from_manager(&manager);
        assert_eq!(tree.collections.len(), 1);
        assert_eq!(tree.collections[0].name, "Github API");
        assert_eq!(tree.collections[0].folders.len(), 1);
        assert_eq!(tree.collections[0].folders[0].requests.len(), 1);

        // Test breadcrumbs
        let crumbs = tree.resolve_breadcrumbs(&manager, req.id);
        assert_eq!(crumbs.len(), 3);
        assert_eq!(crumbs[0].name, "Github API");
        assert_eq!(crumbs[1].name, "Repositories");
        assert_eq!(crumbs[2].name, "List Repos");

        // Test Quick Open
        let matches = tree.quick_open(&manager, "repos", 10);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "List Repos");
        assert!(matches[0].breadcrumb_path.contains("Github API > Repositories > List Repos"));

        let _ = std::fs::remove_dir_all(&root);
    }
}
