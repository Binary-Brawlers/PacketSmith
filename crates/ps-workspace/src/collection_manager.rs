//! File-backed collection manager, directory crawler, and resource operations for PacketSmith.
//!
//! Provides the primary persistence abstraction for human-readable, Git-friendly API collections.
//! Maintains in-memory indexes while synchronizing all changes to the workspace filesystem.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::Utc;
use ps_domain::{
    CollectionDocument, FolderDocument, RequestDocument, ResourceId, ResponseExample,
    TrashMetadata,
};
use serde::{Deserialize, Serialize};
use crate::{
    parse_resource_from_yaml, serialize_resource_to_yaml, slugify, validate_folder_hierarchy,
    WorkspaceError, COLLECTION_EXT, FOLDER_EXT, REQUEST_EXT,
};

/// File extension for saved response examples.
pub const EXAMPLE_EXT: &str = ".example.yaml";

/// Trash directory relative path within workspace.
pub const TRASH_DIR_REL: &str = ".packetsmith/trash";

/// Parent container reference for requests and folders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum ResourceParent {
    Collection(ResourceId),
    Folder(ResourceId),
}

/// Central manager orchestrating collection persistence, tree operations, and filesystem sync.
#[derive(Debug, Clone)]
pub struct CollectionManager {
    workspace_root: PathBuf,
    collections: HashMap<ResourceId, CollectionDocument>,
    folders: HashMap<ResourceId, FolderDocument>,
    requests: HashMap<ResourceId, RequestDocument>,
    examples: HashMap<ResourceId, ResponseExample>,
    resource_paths: HashMap<ResourceId, PathBuf>,
    resource_parents: HashMap<ResourceId, ResourceParent>,
}

impl CollectionManager {
    /// Creates a new, empty CollectionManager for a given workspace root directory.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            collections: HashMap::new(),
            folders: HashMap::new(),
            requests: HashMap::new(),
            examples: HashMap::new(),
            resource_paths: HashMap::new(),
            resource_parents: HashMap::new(),
        }
    }

    /// Recursively scans the workspace directory, loading all collections, folders, requests, and examples.
    pub fn scan(workspace_root: impl Into<PathBuf>) -> Result<Self, WorkspaceError> {
        let root: PathBuf = workspace_root.into();
        let mut manager = Self::new(root.clone());
        let collections_dir = root.join("collections");

        if !collections_dir.exists() {
            fs::create_dir_all(&collections_dir)?;
            return Ok(manager);
        }

        manager.scan_directory(&collections_dir, None, None)?;
        Ok(manager)
    }

    fn scan_directory(
        &mut self,
        dir: &Path,
        current_collection: Option<ResourceId>,
        current_folder: Option<ResourceId>,
    ) -> Result<(), WorkspaceError> {
        let entries = fs::read_dir(dir)?;
        let mut subdirs = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                subdirs.push(path);
                continue;
            }

            if file_name.ends_with(COLLECTION_EXT) {
                let content = fs::read_to_string(&path)?;
                let doc: CollectionDocument = parse_resource_from_yaml(&content)?;
                let rel_path = path.strip_prefix(&self.workspace_root).unwrap_or(&path).to_path_buf();
                let col_id = doc.id;
                self.resource_paths.insert(col_id, rel_path);
                self.collections.insert(col_id, doc);
            } else if file_name.ends_with(FOLDER_EXT) {
                let content = fs::read_to_string(&path)?;
                let doc: FolderDocument = parse_resource_from_yaml(&content)?;
                let rel_path = path.strip_prefix(&self.workspace_root).unwrap_or(&path).to_path_buf();
                let folder_id = doc.id;
                self.resource_paths.insert(folder_id, rel_path);
                if let Some(parent) = doc.parent_id {
                    self.resource_parents.insert(folder_id, ResourceParent::Folder(parent));
                } else {
                    self.resource_parents.insert(folder_id, ResourceParent::Collection(doc.collection_id));
                }
                self.folders.insert(folder_id, doc);
            } else if file_name.ends_with(REQUEST_EXT) {
                let content = fs::read_to_string(&path)?;
                let doc: RequestDocument = parse_resource_from_yaml(&content)?;
                let rel_path = path.strip_prefix(&self.workspace_root).unwrap_or(&path).to_path_buf();
                let req_id = doc.id;
                self.resource_paths.insert(req_id, rel_path);

                if let Some(folder_id) = current_folder {
                    self.resource_parents.insert(req_id, ResourceParent::Folder(folder_id));
                } else if let Some(col_id) = current_collection {
                    self.resource_parents.insert(req_id, ResourceParent::Collection(col_id));
                }
                self.requests.insert(req_id, doc);
            } else if file_name.ends_with(EXAMPLE_EXT) {
                let content = fs::read_to_string(&path)?;
                let doc: ResponseExample = parse_resource_from_yaml(&content)?;
                let rel_path = path.strip_prefix(&self.workspace_root).unwrap_or(&path).to_path_buf();
                self.resource_paths.insert(doc.id, rel_path);
                self.examples.insert(doc.id, doc);
            }
        }

        // Now process subdirectories with context
        for subdir in subdirs {
            let dirname = subdir.file_name().unwrap_or_default().to_string_lossy().to_string();
            // Skip examples directory or hidden directories
            if dirname.ends_with(".examples") || dirname.starts_with('.') {
                // If it's an examples directory, scan it for example files
                if dirname.ends_with(".examples") {
                    for ex_entry in fs::read_dir(&subdir)? {
                        let ex_entry = ex_entry?;
                        let ex_path = ex_entry.path();
                        if ex_path.to_string_lossy().ends_with(EXAMPLE_EXT) {
                            let content = fs::read_to_string(&ex_path)?;
                            let doc: ResponseExample = parse_resource_from_yaml(&content)?;
                            let rel = ex_path.strip_prefix(&self.workspace_root).unwrap_or(&ex_path).to_path_buf();
                            self.resource_paths.insert(doc.id, rel);
                            self.examples.insert(doc.id, doc);
                        }
                    }
                }
                continue;
            }

            // Check if this subdirectory is a collection folder (top-level) or a nested folder
            let mut matched_collection_id = current_collection;
            let mut matched_folder_id = None;

            if current_collection.is_none() {
                // Find collection document in this dir
                for file_entry in fs::read_dir(&subdir)? {
                    let file_entry = file_entry?;
                    if file_entry.file_name().to_string_lossy().ends_with(COLLECTION_EXT) {
                        let content = fs::read_to_string(file_entry.path())?;
                        let doc: CollectionDocument = parse_resource_from_yaml(&content)?;
                        matched_collection_id = Some(doc.id);
                        break;
                    }
                }
            } else {
                // Inside a collection, this is a folder
                for file_entry in fs::read_dir(&subdir)? {
                    let file_entry = file_entry?;
                    if file_entry.file_name().to_string_lossy().ends_with(FOLDER_EXT) {
                        let content = fs::read_to_string(file_entry.path())?;
                        let doc: FolderDocument = parse_resource_from_yaml(&content)?;
                        matched_folder_id = Some(doc.id);
                        break;
                    }
                }
            }

            self.scan_directory(&subdir, matched_collection_id, matched_folder_id)?;
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Collection Operations
    // -----------------------------------------------------------------------

    /// Creates a new collection, writes its manifest to disk, and indexes it.
    pub fn create_collection(
        &mut self,
        name: &str,
        description: Option<String>,
    ) -> Result<CollectionDocument, WorkspaceError> {
        let slug = slugify(name);
        let col_dir = self.workspace_root.join("collections").join(&slug);
        fs::create_dir_all(&col_dir)?;

        let mut doc = CollectionDocument::new(name);
        doc.description = description;

        let file_path = col_dir.join(format!("{}{}", slug, COLLECTION_EXT));
        let yaml = serialize_resource_to_yaml(&doc)?;
        fs::write(&file_path, yaml)?;

        let rel_path = file_path.strip_prefix(&self.workspace_root).unwrap_or(&file_path).to_path_buf();
        self.resource_paths.insert(doc.id, rel_path);
        self.collections.insert(doc.id, doc.clone());

        Ok(doc)
    }

    /// Updates collection metadata (description, variables, auth, scripts, runner settings).
    pub fn update_collection(&mut self, mut collection: CollectionDocument) -> Result<(), WorkspaceError> {
        collection.updated_at = Utc::now();
        let path = self.get_full_path(&collection.id)?;
        let yaml = serialize_resource_to_yaml(&collection)?;
        fs::write(path, yaml)?;
        self.collections.insert(collection.id, collection);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Folder Operations
    // -----------------------------------------------------------------------

    /// Creates a new folder inside a collection or parent folder.
    pub fn create_folder(
        &mut self,
        collection_id: ResourceId,
        parent_folder_id: Option<ResourceId>,
        name: &str,
    ) -> Result<FolderDocument, WorkspaceError> {
        let parent_dir = if let Some(p_id) = parent_folder_id {
            self.get_full_path(&p_id)?.parent().unwrap().to_path_buf()
        } else {
            self.get_full_path(&collection_id)?.parent().unwrap().to_path_buf()
        };

        let slug = slugify(name);
        let folder_dir = parent_dir.join(&slug);
        fs::create_dir_all(&folder_dir)?;

        let mut doc = FolderDocument::new(collection_id, name);
        doc.parent_id = parent_folder_id;

        let file_path = folder_dir.join(format!("{}{}", slug, FOLDER_EXT));
        let yaml = serialize_resource_to_yaml(&doc)?;
        fs::write(&file_path, yaml)?;

        let rel_path = file_path.strip_prefix(&self.workspace_root).unwrap_or(&file_path).to_path_buf();
        self.resource_paths.insert(doc.id, rel_path);

        // Append to parent item_order
        if let Some(p_id) = parent_folder_id {
            let p_path = self.get_full_path(&p_id)?;
            if let Some(parent) = self.folders.get_mut(&p_id) {
                parent.item_order.push(doc.id);
                fs::write(p_path, serialize_resource_to_yaml(parent)?)?;
            }
            self.resource_parents.insert(doc.id, ResourceParent::Folder(p_id));
        } else {
            let col_path = self.get_full_path(&collection_id)?;
            if let Some(col) = self.collections.get_mut(&collection_id) {
                col.item_order.push(doc.id);
                fs::write(col_path, serialize_resource_to_yaml(col)?)?;
            }
            self.resource_parents.insert(doc.id, ResourceParent::Collection(collection_id));
        }

        self.folders.insert(doc.id, doc.clone());
        Ok(doc)
    }

    /// Updates folder configuration and metadata.
    pub fn update_folder(&mut self, folder: FolderDocument) -> Result<(), WorkspaceError> {
        let path = self.get_full_path(&folder.id)?;
        let yaml = serialize_resource_to_yaml(&folder)?;
        fs::write(path, yaml)?;
        self.folders.insert(folder.id, folder);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Request Operations
    // -----------------------------------------------------------------------

    /// Creates a new request document inside a collection or folder.
    pub fn create_request(
        &mut self,
        collection_id: ResourceId,
        folder_id: Option<ResourceId>,
        name: &str,
        protocol: ps_domain::ProtocolRequest,
    ) -> Result<RequestDocument, WorkspaceError> {
        let parent_dir = if let Some(f_id) = folder_id {
            self.get_full_path(&f_id)?.parent().unwrap().to_path_buf()
        } else {
            self.get_full_path(&collection_id)?.parent().unwrap().to_path_buf()
        };

        let slug = self.find_available_slug(&parent_dir, &slugify(name), REQUEST_EXT);
        let file_path = parent_dir.join(format!("{}{}", slug, REQUEST_EXT));

        let doc = RequestDocument::new(name, protocol);
        let yaml = serialize_resource_to_yaml(&doc)?;
        fs::write(&file_path, yaml)?;

        let rel_path = file_path.strip_prefix(&self.workspace_root).unwrap_or(&file_path).to_path_buf();
        self.resource_paths.insert(doc.id, rel_path);

        // Append to parent item_order
        if let Some(f_id) = folder_id {
            let f_path = self.get_full_path(&f_id)?;
            if let Some(folder) = self.folders.get_mut(&f_id) {
                folder.item_order.push(doc.id);
                fs::write(f_path, serialize_resource_to_yaml(folder)?)?;
            }
            self.resource_parents.insert(doc.id, ResourceParent::Folder(f_id));
        } else {
            let col_path = self.get_full_path(&collection_id)?;
            if let Some(col) = self.collections.get_mut(&collection_id) {
                col.item_order.push(doc.id);
                fs::write(col_path, serialize_resource_to_yaml(col)?)?;
            }
            self.resource_parents.insert(doc.id, ResourceParent::Collection(collection_id));
        }

        self.requests.insert(doc.id, doc.clone());
        Ok(doc)
    }

    /// Persists an updated RequestDocument to disk.
    pub fn save_request(&mut self, mut request: RequestDocument) -> Result<(), WorkspaceError> {
        request.updated_at = Utc::now();
        let path = self.get_full_path(&request.id)?;
        let yaml = serialize_resource_to_yaml(&request)?;
        fs::write(path, yaml)?;
        self.requests.insert(request.id, request);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Rename, Move, Duplicate, Reorder
    // -----------------------------------------------------------------------

    /// Renames a resource (collection, folder, or request) on disk and updates in-memory paths.
    pub fn rename_resource(&mut self, id: ResourceId, new_name: &str) -> Result<(), WorkspaceError> {
        let current_path = self.get_full_path(&id)?;
        let new_slug = slugify(new_name);

        if let Some(col) = self.collections.get_mut(&id) {
            col.name = new_name.to_string();
            col.updated_at = Utc::now();
            let parent = current_path.parent().and_then(|p| p.parent()).unwrap();
            let new_dir = parent.join(&new_slug);
            fs::rename(current_path.parent().unwrap(), &new_dir)?;

            let old_file_name = current_path.file_name().unwrap();
            let new_file_path = new_dir.join(format!("{}{}", new_slug, COLLECTION_EXT));
            let old_file_in_new_dir = new_dir.join(old_file_name);
            if old_file_in_new_dir.exists() && old_file_in_new_dir != new_file_path {
                fs::rename(old_file_in_new_dir, &new_file_path)?;
            }

            fs::write(&new_file_path, serialize_resource_to_yaml(col)?)?;
            let rel = new_file_path.strip_prefix(&self.workspace_root).unwrap_or(&new_file_path).to_path_buf();
            self.resource_paths.insert(id, rel);
            let _ = self.refresh_paths();
            return Ok(());
        }

        if let Some(folder) = self.folders.get_mut(&id) {
            folder.name = new_name.to_string();
            let parent_dir = current_path.parent().and_then(|p| p.parent()).unwrap();
            let new_dir = parent_dir.join(&new_slug);
            fs::rename(current_path.parent().unwrap(), &new_dir)?;

            let old_file_name = current_path.file_name().unwrap();
            let new_file_path = new_dir.join(format!("{}{}", new_slug, FOLDER_EXT));
            let old_file_in_new_dir = new_dir.join(old_file_name);
            if old_file_in_new_dir.exists() && old_file_in_new_dir != new_file_path {
                fs::rename(old_file_in_new_dir, &new_file_path)?;
            }

            fs::write(&new_file_path, serialize_resource_to_yaml(folder)?)?;
            let rel = new_file_path.strip_prefix(&self.workspace_root).unwrap_or(&new_file_path).to_path_buf();
            self.resource_paths.insert(id, rel);
            let _ = self.refresh_paths();
            return Ok(());
        }

        if let Some(req) = self.requests.get_mut(&id) {
            req.name = new_name.to_string();
            req.updated_at = Utc::now();
            let parent_dir = current_path.parent().unwrap();
            let new_file_path = parent_dir.join(format!("{}{}", new_slug, REQUEST_EXT));

            if current_path != new_file_path {
                fs::rename(&current_path, &new_file_path)?;
            }

            fs::write(&new_file_path, serialize_resource_to_yaml(req)?)?;
            let rel = new_file_path.strip_prefix(&self.workspace_root).unwrap_or(&new_file_path).to_path_buf();
            self.resource_paths.insert(id, rel);
            return Ok(());
        }

        Err(WorkspaceError::NotFound(current_path))
    }

    /// Moves a request or folder into a new collection or folder container.
    pub fn move_resource(&mut self, id: ResourceId, new_parent: ResourceParent) -> Result<(), WorkspaceError> {
        let current_path = self.get_full_path(&id)?;
        let file_name = current_path.file_name().unwrap().to_os_string();

        // 1. Remove from old parent item_order
        if let Some(old_parent) = self.resource_parents.get(&id).copied() {
            self.remove_from_item_order(old_parent, id)?;
        }

        // 2. Validate hierarchy for folders (prevent cyclic ancestry)
        if let Some(folder) = self.folders.get_mut(&id) {
            if let ResourceParent::Folder(new_f_id) = new_parent {
                if new_f_id == id {
                    return Err(WorkspaceError::CircularHierarchy(id));
                }
                folder.parent_id = Some(new_f_id);
            } else if let ResourceParent::Collection(new_c_id) = new_parent {
                folder.parent_id = None;
                folder.collection_id = new_c_id;
            }

            let all_folders: Vec<FolderDocument> = self.folders.values().cloned().collect();
            validate_folder_hierarchy(&all_folders)?;
        }

        // 3. Move on filesystem
        let target_dir = match new_parent {
            ResourceParent::Collection(c_id) => self.get_full_path(&c_id)?.parent().unwrap().to_path_buf(),
            ResourceParent::Folder(f_id) => self.get_full_path(&f_id)?.parent().unwrap().to_path_buf(),
        };

        if self.folders.contains_key(&id) {
            let folder_dir = current_path.parent().unwrap();
            let folder_dirname = folder_dir.file_name().unwrap();
            let new_dir = target_dir.join(folder_dirname);
            fs::rename(folder_dir, &new_dir)?;
            let new_file_path = new_dir.join(&file_name);
            let rel = new_file_path.strip_prefix(&self.workspace_root).unwrap_or(&new_file_path).to_path_buf();
            self.resource_paths.insert(id, rel);
            let _ = self.refresh_paths();
        } else if self.requests.contains_key(&id) {
            let new_file_path = target_dir.join(&file_name);
            fs::rename(&current_path, &new_file_path)?;
            let rel = new_file_path.strip_prefix(&self.workspace_root).unwrap_or(&new_file_path).to_path_buf();
            self.resource_paths.insert(id, rel);
        }

        // 4. Add to new parent item_order
        match new_parent {
            ResourceParent::Collection(c_id) => {
                let p = self.get_full_path(&c_id)?;
                if let Some(col) = self.collections.get_mut(&c_id) {
                    col.item_order.push(id);
                    fs::write(p, serialize_resource_to_yaml(col)?)?;
                }
            }
            ResourceParent::Folder(f_id) => {
                let p = self.get_full_path(&f_id)?;
                if let Some(folder) = self.folders.get_mut(&f_id) {
                    folder.item_order.push(id);
                    fs::write(p, serialize_resource_to_yaml(folder)?)?;
                }
            }
        }

        self.resource_parents.insert(id, new_parent);
        Ok(())
    }

    /// Reorders child items within a collection or folder.
    pub fn reorder_items(&mut self, parent: ResourceParent, new_order: Vec<ResourceId>) -> Result<(), WorkspaceError> {
        match parent {
            ResourceParent::Collection(c_id) => {
                let path = self.get_full_path(&c_id)?;
                if let Some(col) = self.collections.get_mut(&c_id) {
                    col.item_order = new_order;
                    fs::write(path, serialize_resource_to_yaml(col)?)?;
                }
            }
            ResourceParent::Folder(f_id) => {
                let path = self.get_full_path(&f_id)?;
                if let Some(folder) = self.folders.get_mut(&f_id) {
                    folder.item_order = new_order;
                    fs::write(path, serialize_resource_to_yaml(folder)?)?;
                }
            }
        }
        Ok(())
    }

    /// Duplicates an existing request and writes a copy to disk.
    pub fn duplicate_request(&mut self, id: ResourceId) -> Result<RequestDocument, WorkspaceError> {
        let req = self.requests.get(&id).cloned().ok_or_else(|| {
            WorkspaceError::NotFound(self.workspace_root.join(format!("{}.req.yaml", id)))
        })?;

        let parent = self.resource_parents.get(&id).copied().ok_or_else(|| {
            WorkspaceError::NotFound(self.workspace_root.join(format!("{}.req.yaml", id)))
        })?;

        let (col_id, folder_id) = match parent {
            ResourceParent::Collection(c) => (c, None),
            ResourceParent::Folder(f) => {
                let f_doc = self.folders.get(&f).unwrap();
                (f_doc.collection_id, Some(f))
            }
        };

        let copy_name = format!("{} (Copy)", req.name);
        let mut cloned = req.clone();
        cloned.id = ResourceId::new();
        cloned.name = copy_name;
        cloned.created_at = Utc::now();
        cloned.updated_at = cloned.created_at;

        let parent_dir = if let Some(f_id) = folder_id {
            self.get_full_path(&f_id)?.parent().unwrap().to_path_buf()
        } else {
            self.get_full_path(&col_id)?.parent().unwrap().to_path_buf()
        };

        let slug = self.find_available_slug(&parent_dir, &slugify(&cloned.name), REQUEST_EXT);
        let file_path = parent_dir.join(format!("{}{}", slug, REQUEST_EXT));

        fs::write(&file_path, serialize_resource_to_yaml(&cloned)?)?;
        let rel_path = file_path.strip_prefix(&self.workspace_root).unwrap_or(&file_path).to_path_buf();
        self.resource_paths.insert(cloned.id, rel_path);

        if let Some(f_id) = folder_id {
            let f_path = self.get_full_path(&f_id)?;
            if let Some(folder) = self.folders.get_mut(&f_id) {
                folder.item_order.push(cloned.id);
                fs::write(f_path, serialize_resource_to_yaml(folder)?)?;
            }
            self.resource_parents.insert(cloned.id, ResourceParent::Folder(f_id));
        } else {
            let col_path = self.get_full_path(&col_id)?;
            if let Some(col) = self.collections.get_mut(&col_id) {
                col.item_order.push(cloned.id);
                fs::write(col_path, serialize_resource_to_yaml(col)?)?;
            }
            self.resource_parents.insert(cloned.id, ResourceParent::Collection(col_id));
        }

        self.requests.insert(cloned.id, cloned.clone());
        Ok(cloned)
    }

    // -----------------------------------------------------------------------
    // Soft-Delete (Trash) & Restoration
    // -----------------------------------------------------------------------

    /// Moves a resource to `.packetsmith/trash/<id>/` with metadata for restoration.
    pub fn trash_resource(&mut self, id: ResourceId) -> Result<TrashMetadata, WorkspaceError> {
        let current_path = self.get_full_path(&id)?;
        let trash_dir = self.workspace_root.join(TRASH_DIR_REL).join(id.to_string());
        fs::create_dir_all(&trash_dir)?;

        let (resource_name, resource_type, target_file_or_dir) = if let Some(col) = self.collections.remove(&id) {
            (col.name, "collection".to_string(), current_path.parent().unwrap().to_path_buf())
        } else if let Some(folder) = self.folders.remove(&id) {
            (folder.name, "folder".to_string(), current_path.parent().unwrap().to_path_buf())
        } else if let Some(req) = self.requests.remove(&id) {
            (req.name, "request".to_string(), current_path.clone())
        } else if let Some(ex) = self.examples.remove(&id) {
            (ex.name, "example".to_string(), current_path.clone())
        } else {
            return Err(WorkspaceError::NotFound(current_path));
        };

        // Remove from parent item_order
        if let Some(parent) = self.resource_parents.remove(&id) {
            self.remove_from_item_order(parent, id)?;
        }

        let original_path_str = self.resource_paths.remove(&id).unwrap().to_string_lossy().to_string();
        let metadata = TrashMetadata {
            resource_id: id,
            original_path: original_path_str,
            resource_name,
            resource_type,
            deleted_at: Utc::now(),
        };

        // Write metadata JSON
        let meta_file = trash_dir.join("trash_info.json");
        fs::write(meta_file, serde_json::to_string_pretty(&metadata).unwrap())?;

        // Move the file or folder
        let dest = trash_dir.join(target_file_or_dir.file_name().unwrap());
        fs::rename(target_file_or_dir, dest)?;

        Ok(metadata)
    }

    /// Restores a previously trashed resource to its original path and re-indexes it.
    pub fn restore_resource(&mut self, id: ResourceId) -> Result<(), WorkspaceError> {
        let trash_dir = self.workspace_root.join(TRASH_DIR_REL).join(id.to_string());
        let meta_file = trash_dir.join("trash_info.json");
        if !meta_file.exists() {
            return Err(WorkspaceError::NotFound(meta_file));
        }

        let meta_content = fs::read_to_string(&meta_file)?;
        let meta: TrashMetadata = serde_json::from_str(&meta_content).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;

        let original_full_path = self.workspace_root.join(&meta.original_path);
        let dest_parent = original_full_path.parent().unwrap();
        fs::create_dir_all(dest_parent)?;

        let entries = fs::read_dir(&trash_dir)?;
        for entry in entries {
            let entry = entry?;
            let p = entry.path();
            if p.file_name().unwrap() != "trash_info.json" {
                if p.is_dir() {
                    let dest = dest_parent.join(p.file_name().unwrap());
                    fs::rename(&p, dest)?;
                } else {
                    fs::rename(&p, &original_full_path)?;
                }
            }
        }

        // Clean up trash directory
        let _ = fs::remove_dir_all(&trash_dir);

        // Rescan workspace to cleanly re-index restored hierarchy
        let refreshed = Self::scan(&self.workspace_root)?;
        self.collections = refreshed.collections;
        self.folders = refreshed.folders;
        self.requests = refreshed.requests;
        self.examples = refreshed.examples;
        self.resource_paths = refreshed.resource_paths;
        self.resource_parents = refreshed.resource_parents;

        Ok(())
    }

    /// Lists all resources currently in the trash.
    pub fn list_trash(&self) -> Result<Vec<TrashMetadata>, WorkspaceError> {
        let trash_root = self.workspace_root.join(TRASH_DIR_REL);
        if !trash_root.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        for entry in fs::read_dir(trash_root)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let meta_path = entry.path().join("trash_info.json");
                if meta_path.exists() {
                    if let Ok(content) = fs::read_to_string(&meta_path) {
                        if let Ok(meta) = serde_json::from_str::<TrashMetadata>(&content) {
                            results.push(meta);
                        }
                    }
                }
            }
        }

        results.sort_by_key(|b| std::cmp::Reverse(b.deleted_at));
        Ok(results)
    }

    /// Permanently deletes a resource from the trash.
    pub fn permanent_delete_trash(&self, id: ResourceId) -> Result<(), WorkspaceError> {
        let trash_dir = self.workspace_root.join(TRASH_DIR_REL).join(id.to_string());
        if trash_dir.exists() {
            fs::remove_dir_all(trash_dir)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Response Examples CRUD
    // -----------------------------------------------------------------------

    /// Saves a response snapshot as a named example linked to a request document.
    pub fn save_response_as_example(
        &mut self,
        request_id: ResourceId,
        name: &str,
        status_code: u16,
        status_text: &str,
        headers: HashMap<String, String>,
        body: &str,
    ) -> Result<ResponseExample, WorkspaceError> {
        let req_path = self.get_full_path(&request_id)?;
        let parent_dir = req_path.parent().unwrap();
        let req_slug = req_path.file_stem().unwrap().to_string_lossy().replace(".req", "");
        let examples_dir = parent_dir.join(format!("{}.examples", req_slug));
        fs::create_dir_all(&examples_dir)?;

        let ex_slug = slugify(name);
        let ex_file = examples_dir.join(format!("{}{}", ex_slug, EXAMPLE_EXT));

        let example = ResponseExample::new(request_id, name, status_code, status_text, headers, body);
        let yaml = serialize_resource_to_yaml(&example)?;
        fs::write(&ex_file, yaml)?;

        let rel = ex_file.strip_prefix(&self.workspace_root).unwrap_or(&ex_file).to_path_buf();
        self.resource_paths.insert(example.id, rel);
        self.examples.insert(example.id, example.clone());

        // Link example ref in RequestDocument
        let r_path = self.get_full_path(&request_id)?;
        if let Some(req) = self.requests.get_mut(&request_id) {
            req.examples.push(example.to_ref());
            fs::write(r_path, serialize_resource_to_yaml(req)?)?;
        }

        Ok(example)
    }

    /// Updates an existing response example.
    pub fn edit_example(&mut self, example: ResponseExample) -> Result<(), WorkspaceError> {
        let path = self.get_full_path(&example.id)?;
        let yaml = serialize_resource_to_yaml(&example)?;
        fs::write(path, yaml)?;

        // Update name/status in RequestDocument example ref if changed
        let r_path = self.get_full_path(&example.request_id)?;
        if let Some(req) = self.requests.get_mut(&example.request_id) {
            if let Some(r) = req.examples.iter_mut().find(|r| r.id == example.id) {
                r.name = example.name.clone();
                r.status_code = example.status_code;
            }
            fs::write(r_path, serialize_resource_to_yaml(req)?)?;
        }

        self.examples.insert(example.id, example);
        Ok(())
    }

    /// Duplicates an example with a new ID.
    pub fn duplicate_example(&mut self, example_id: ResourceId) -> Result<ResponseExample, WorkspaceError> {
        let ex = self.examples.get(&example_id).cloned().ok_or_else(|| {
            WorkspaceError::NotFound(self.workspace_root.join(format!("{}.example.yaml", example_id)))
        })?;

        let new_name = format!("{} (Copy)", ex.name);
        self.save_response_as_example(
            ex.request_id,
            &new_name,
            ex.status_code,
            &ex.status_text,
            ex.headers,
            &ex.body,
        )
    }

    /// Deletes an example from disk and unlinks it from its parent request.
    pub fn delete_example(&mut self, example_id: ResourceId) -> Result<(), WorkspaceError> {
        if let Some(ex) = self.examples.remove(&example_id) {
            let path = self.get_full_path(&example_id)?;
            if path.exists() {
                let _ = fs::remove_file(path);
            }
            self.resource_paths.remove(&example_id);

            // Unlink from request
            let r_path = self.get_full_path(&ex.request_id)?;
            if let Some(req) = self.requests.get_mut(&ex.request_id) {
                req.examples.retain(|r| r.id != example_id);
                fs::write(r_path, serialize_resource_to_yaml(req)?)?;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Getters & Query Utilities
    // -----------------------------------------------------------------------

    pub fn collections(&self) -> &HashMap<ResourceId, CollectionDocument> {
        &self.collections
    }

    pub fn folders(&self) -> &HashMap<ResourceId, FolderDocument> {
        &self.folders
    }

    pub fn requests(&self) -> &HashMap<ResourceId, RequestDocument> {
        &self.requests
    }

    pub fn examples(&self) -> &HashMap<ResourceId, ResponseExample> {
        &self.examples
    }

    pub fn get_full_path(&self, id: &ResourceId) -> Result<PathBuf, WorkspaceError> {
        self.resource_paths
            .get(id)
            .map(|rel| self.workspace_root.join(rel))
            .ok_or_else(|| WorkspaceError::NotFound(PathBuf::from(format!("Resource {}", id))))
    }

    pub fn get_relative_path(&self, id: &ResourceId) -> Option<&PathBuf> {
        self.resource_paths.get(id)
    }

    pub fn get_resource_parent(&self, id: &ResourceId) -> Option<ResourceParent> {
        self.resource_parents.get(id).copied()
    }

    fn remove_from_item_order(&mut self, parent: ResourceParent, child_id: ResourceId) -> Result<(), WorkspaceError> {
        match parent {
            ResourceParent::Collection(c_id) => {
                let p = self.get_full_path(&c_id)?;
                if let Some(col) = self.collections.get_mut(&c_id) {
                    col.item_order.retain(|&id| id != child_id);
                    fs::write(p, serialize_resource_to_yaml(col)?)?;
                }
            }
            ResourceParent::Folder(f_id) => {
                let p = self.get_full_path(&f_id)?;
                if let Some(folder) = self.folders.get_mut(&f_id) {
                    folder.item_order.retain(|&id| id != child_id);
                    fs::write(p, serialize_resource_to_yaml(folder)?)?;
                }
            }
        }
        Ok(())
    }

    fn find_available_slug(&self, parent_dir: &Path, base_slug: &str, ext: &str) -> String {
        let mut slug = base_slug.to_string();
        let mut counter = 1;
        while parent_dir.join(format!("{}{}", slug, ext)).exists() {
            slug = format!("{}-{}", base_slug, counter);
            counter += 1;
        }
        slug
    }

    fn refresh_paths(&mut self) -> Result<(), WorkspaceError> {
        let rescanned = Self::scan(&self.workspace_root)?;
        self.resource_paths = rescanned.resource_paths;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ps_domain::{HttpRequestPayload, ProtocolRequest};

    fn temp_workspace(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("packetsmith-test-{}-{}", prefix, uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_collection_manager_crud_and_hierarchy() {
        let root = temp_workspace("crud");
        let mut manager = CollectionManager::scan(&root).expect("scan empty");

        // 1. Create collection
        let col = manager
            .create_collection("E-Commerce API", Some("Store endpoints".into()))
            .expect("create col");
        assert_eq!(col.name, "E-Commerce API");

        // 2. Create folder
        let folder = manager
            .create_folder(col.id, None, "Orders")
            .expect("create folder");
        assert_eq!(folder.name, "Orders");
        assert_eq!(folder.collection_id, col.id);

        // 3. Create request
        let req = manager
            .create_request(
                col.id,
                Some(folder.id),
                "Get Order By ID",
                ProtocolRequest::Http(HttpRequestPayload {
                    method: "GET".to_string(),
                    url: "https://api.store.com/orders/123".to_string(),
                }),
            )
            .expect("create req");
        assert_eq!(req.name, "Get Order By ID");

        // 4. Save response example
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let example = manager
            .save_response_as_example(
                req.id,
                "200 OK Response",
                200,
                "OK",
                headers,
                "{\"order_id\": 123, \"status\": \"shipped\"}",
            )
            .expect("save example");
        assert_eq!(example.status_code, 200);

        // Verify rescanning loads the tree properly
        let rescanned = CollectionManager::scan(&root).expect("rescan");
        assert!(rescanned.collections().contains_key(&col.id));
        assert!(rescanned.folders().contains_key(&folder.id));
        assert!(rescanned.requests().contains_key(&req.id));
        assert!(rescanned.examples().contains_key(&example.id));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn test_trash_and_restore() {
        let root = temp_workspace("trash");
        let mut manager = CollectionManager::scan(&root).expect("scan empty");

        let col = manager.create_collection("Test Col", None).expect("create col");
        let req = manager
            .create_request(
                col.id,
                None,
                "Login Request",
                ProtocolRequest::Http(HttpRequestPayload {
                    method: "POST".to_string(),
                    url: "https://api.test.com/login".to_string(),
                }),
            )
            .expect("create req");

        // Soft-delete request
        let trash_meta = manager.trash_resource(req.id).expect("trash req");
        assert_eq!(trash_meta.resource_name, "Login Request");
        assert_eq!(trash_meta.resource_type, "request");
        assert!(!manager.requests().contains_key(&req.id));

        // List trash
        let trash_list = manager.list_trash().expect("list trash");
        assert_eq!(trash_list.len(), 1);
        assert_eq!(trash_list[0].resource_id, req.id);

        // Restore resource
        manager.restore_resource(req.id).expect("restore");
        assert!(manager.requests().contains_key(&req.id));
        assert_eq!(manager.list_trash().expect("list trash").len(), 0);

        let _ = fs::remove_dir_all(&root);
    }
}
