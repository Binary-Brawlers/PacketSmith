# Crate Map & Dependency Direction Rules

To prevent circular dependencies and architectural drift, PacketSmith enforces strict dependency direction rules across its crates.

## Crate Responsibilities

| Crate | Responsibility | Permitted Internal Dependencies |
|---|---|---|
| `ps-domain` | Base domain models, entity IDs, auth types, variable scopes | *(None)* |
| `ps-workspace` | Native `packetsmith.yaml` manifest, file loader, layout | `ps-domain` |
| `ps-storage` | SQLite cache, execution history, indexing | `ps-domain` |
| `ps-variable` | Template parser, scoped resolution, dynamic values, provenance, redaction | `ps-domain` |
| `ps-request-engine` | `ProtocolExecutor` trait, execution context, event streaming | `ps-domain`, `ps-variable` |
| `ps-http` | HTTP models, URL parser, HTTP execution driver | `ps-domain`, `ps-request-engine` |
| `ps-ui-components` | Design tokens, color scales, UI primitives | *(None)* |
| `ps-settings` | Application and workspace settings schema | `ps-ui-components` |
| `ps-test-support` | Test fixtures, mock generators, test workspaces | `ps-domain`, `ps-workspace`, `ps-storage`, `ps-http` |
| `packetsmith-app` | Desktop binary, GPUI window boot, app coordinator | `ps-domain`, `ps-workspace`, `ps-storage`, `ps-variable`, `ps-request-engine`, `ps-http`, `ps-ui-components`, `ps-settings` |
| `xtask` | Developer automation CLI | *(Standalone)* |

## Dependency Rules

1. `ps-domain` is the foundation. It must not depend on any other PacketSmith crate.
2. `ps-ui-components` contains visual design system tokens and must not depend on network or storage crates.
3. Neither `ps-http` nor `ps-request-engine` may depend on `packetsmith-app` or `ps-ui-components`.
4. Circular dependencies are strictly forbidden and will fail compilation.
5. `ps-variable` owns resolution policy and must not depend on protocol, storage, or UI crates.

## ps-vault

OS credential storage and domain-restricted secret service; depends on ps-variable,
never GPUI or workspace persistence. See ADR 0013 for integration boundaries.
