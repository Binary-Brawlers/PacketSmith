# PacketSmith Architecture Overview

PacketSmith is a modular, local-first API development platform engineered in Rust.

## System Layers

```text
┌─────────────────────────────────────────────────────────────┐
│                     GPUI Desktop Shell                      │
│                  (crates/packetsmith-app)                   │
├─────────────────────────────────────────────────────────────┤
│ UI Design System | Commands | Panels | Tabs | Keymaps       │
│                  (crates/ps-ui-components)                 │
├─────────────────────────────────────────────────────────────┤
│                        Domain Model                         │
│   RequestDocument | ProtocolRequest | Auth | ResourceId     │
│                     (crates/ps-domain)                      │
├─────────────────────────────────────────────────────────────┤
│                     Protocol Engines                        │
│            HTTP (crates/ps-http) | Future: gRPC, WS...       │
├─────────────────────────────────────────────────────────────┤
│               Request Engine & Lifecycle Sink               │
│               (crates/ps-request-engine)                    │
├─────────────────────────────────────────────────────────────┤
│                 Workspace & Storage Layer                   │
│   YAML Files (ps-workspace) | SQLite Cache (ps-storage)     │
├─────────────────────────────────────────────────────────────┤
│                      Settings Engine                        │
│                    (crates/ps-settings)                     │
└─────────────────────────────────────────────────────────────┘
```

## Architectural Rules

1. **Protocol Engines are Independent from GPUI:**
   Protocol execution drivers (such as `ps-http`) must never reference UI crates or GPUI types. This enables the headless CLI, daemon, and desktop app to share 100% of network and execution logic.

2. **UI Never Directly Touches Raw Disk Files or Networking:**
   The UI interacts with domain services via structured commands, event channels, and abstractions.

3. **Separation of Authoring vs Derived State:**
   - User-authored resources (requests, environments, collections) are persisted as human-readable YAML/JSON files.
   - Derived indexing, history, and transient layouts live in the SQLite cache (`.packetsmith/cache.db`).
