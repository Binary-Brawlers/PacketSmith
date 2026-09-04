# ADR-0003: Open, File-Based Workspace Manifest

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

Many API clients store user workspaces in opaque proprietary databases or cloud backends, causing severe vendor lock-in and unreadable Git diffs when collaborating via version control. PacketSmith must be Git-native and local-first.

## Considered Options

1. **Single SQLite database file per workspace:** Good for atomic queries, poor for Git diffs, merging, and PR reviews.
2. **Single huge JSON file (e.g. Postman collection format):** Prone to merge conflicts when multiple people edit different requests.
3. **Structured multi-file directory with `packetsmith.yaml` manifest:** Root manifest defines metadata and settings; collections and requests exist as discrete YAML files.

## Decision Outcome

Chosen option: **Structured multi-file directory with `packetsmith.yaml` root manifest.**

### Positive Consequences
- Exceptional Git friendliness: moving, adding, or modifying a request touches only that specific request file.
- Clean pull request diffs and simple conflict resolution.
- Secrets are excluded from versioned files by design.

### Negative Consequences
- Requires a file-system watcher to detect external modifications and synchronize application state.
