# ADR-0007: SQLite Indexing, Search, and Ephemeral Cache Architecture

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

As workspaces grow to hundreds of requests, folders, and historical logs, linear file system scans for Quick Open, history search, and tab state restoration degrade performance. We need high-speed derived indexing without mutating or bloating human-authored YAML files in version control.

## Considered Options

1. **In-memory HashMaps only:** Fast, but requires re-scanning and re-parsing all workspace files on cold startup, violating the < 1s startup target.
2. **Dedicated search engine (e.g. Tantivy/Meilisearch):** Adds heavy binary dependencies and complex indexing daemons.
3. **SQLite derived index (`.packetsmith/cache.db`):** Fast, embedded, zero-maintenance, supports WAL mode, and easily rebuildable if deleted.

## Decision Outcome

Chosen option: **SQLite derived index (`.packetsmith/cache.db`)**.

### Positive Consequences
- Fast sub-millisecond full-text queries for Quick Open and history retrieval.
- Preserves disk and Git cleanliness by adding `.packetsmith/` to `.gitignore`.
- If the cache database is removed or corrupted, `ps-storage` regenerates it automatically.

### Negative Consequences
- Changes to files on disk require invalidation and incremental indexing via `WorkspaceWatcher`.
