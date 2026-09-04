# ADR-0010: File-Backed Collections, Resource Tree, and Multi-Pane Workbench Architecture

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

PacketSmith is local-first and source-control friendly by design. Developers require:
1. Organizing requests into hierarchical collections and nested folders directly on disk using human-readable, Git-diffable YAML files.
2. Safe resource operations (create, rename, move across folders, duplicate, and reorder).
3. Non-destructive deletion via soft-delete/trash archiving with lossless restoration capability.
4. Saving response snapshots as named examples linked to requests for documentation and mocks.
5. Flexible multi-pane workbench layout with vertical/horizontal splits, tab pinning, dirty state protection, closed-tab restoration (`Cmd+Shift+T`), layout persistence in SQLite, and Quick Open fuzzy search.

## Considered Options

1. **Single SQLite Database for Collections and Workspaces:**
   - Pros: Simple relational queries and foreign keys.
   - Cons: Violates the core local-first and Git-native architectural principles. Diffs would be opaque binary blobs, preventing meaningful version control.
2. **File-Backed Hierarchy with In-Memory Indexing and SQLite Derived Cache (`ps-workspace` + `ps-storage`):**
   - Collections, folders, requests, and examples are stored as human-readable YAML (`.col.yaml`, `.folder.yaml`, `.req.yaml`, `.example.yaml`).
   - In-memory `CollectionManager` and `ResourceTree` provide fast traversal, expansion tracking, breadcrumbs, and Quick Open search.
   - SQLite cache stores only derived state (open tabs, split proportions, and history).
   - Soft-deleted items are safely archived to `.packetsmith/trash/<id>/` with metadata for one-click restoration.

## Decision Outcome

Chosen option: **Option 2: File-Backed Hierarchy with In-Memory Indexing and SQLite Derived Cache**.

### Positive Consequences
- True Git compatibility: Every request, folder, and collection is tracked as individual YAML files with clean diffs.
- Safety: Deleting resources moves them to `.packetsmith/trash/` rather than immediate unrecoverable filesystem removal.
- Ergonomics: Multi-pane workbench supports arbitrary horizontal and vertical splits, tab pinning prevents accidental closure, and closed tabs can be reopened using LIFO stack.
- Quick Open: Fast fuzzy search across request titles and endpoints with complete hierarchical breadcrumbs.
- Resilience: Open tabs and workbench split layout survive application restarts by synchronizing with SQLite cache.

### Negative Consequences
- Renaming or moving folders requires updating on-disk paths and rescanning or syncing in-memory indices.
