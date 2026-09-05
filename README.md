# PacketSmith

> **The Open-Source, Local-First API Development Platform.**  
> Built with Rust & Zed GPUI for blazing performance, open human-readable workspace files, and full protocol versatility.

---

## Highlights

- **Local-First & Offline-First:** No accounts required, no forced telemetry, and complete local privacy.
- **Git-Friendly Workspaces:** Resources and requests are saved as structured files that generate clean diffs.
- **GPU-Accelerated UI:** High-performance native desktop client built on Zed's GPUI framework.
- **Protocol-First Architecture:** Modular engines for HTTP/REST, GraphQL, gRPC, WebSocket, Socket.IO, MQTT, MCP, and AI endpoints.
- **Desktop & CLI Parity:** Execute the exact same requests and collection test suites in CI/CD via the headless CLI.

---

## Workspace Architecture

```text
packetsmith/
├── Cargo.toml                  # Root Cargo workspace manifest
├── rust-toolchain.toml         # Pinned Rust toolchain
├── crates/
│   ├── packetsmith-app/        # GPUI desktop application shell
│   ├── ps-domain/              # Protocol-neutral domain models & entities
│   ├── ps-workspace/           # Workspace manifest & file serialization
│   ├── ps-storage/             # Local SQLite database cache & indexing
│   ├── ps-request-engine/      # Protocol execution traits, context & event stream
│   ├── ps-http/                # HTTP request/response models & execution engine
│   ├── ps-ui-components/       # UI design tokens, theme palettes & components
│   ├── ps-settings/            # App and workspace settings schemas & store
│   ├── ps-test-support/        # Test utilities, fixtures & mock HTTP servers
│   └── xtask/                  # Developer task automation CLI
├── docs/                       # Architecture overviews, ADRs & platform guides
└── .github/                    # Workflows, issue templates & PR template
```

---

## Getting Started

### Prerequisites
- **Rust:** Install through `rustup`; this repository selects the version pinned in `rust-toolchain.toml`, including `rustfmt` and `clippy`.
- **macOS:** Xcode command line tools.
- **Linux:** `vulkan-loader`, `libxkbcommon-dev`, `libfontconfig1-dev`.
- **Windows:** MSVC C++ build tools.

Refer to [`docs/setup/`](docs/setup/) for platform-specific instructions.

### Start the Application

From the repository root, run:

```bash
cargo run -p packetsmith-app --features gpui-ui
```

This opens the native desktop window. The `gpui-ui` feature is required; without it, the application initializes the core shell and exits without opening a window. The first launch may take longer while Cargo downloads and compiles dependencies.

### Current Desktop Workflow

The app opens the **Requests** workbench. Enter an HTTP method and URL, add headers
or a raw/JSON body, then choose **Send request**. Use **Cancel** to stop an in-flight
request. Response status, timing, size, body, and headers appear below the editor.

- **New request** opens another draft; select a tab to switch drafts.
- **Cmd/Ctrl+Enter** sends the active request; **Cmd/Ctrl+T** creates a draft.
- **Environments** opens workspace loading and environment management. Its active
  environment is used when resolving variables for requests.
- Saved HTTP requests from the loaded workspace appear in the left sidebar.

This is an initial desktop integration: drafts are session-only, response storage
is limited to 2 MiB, and redirects are not followed. Saving edited drafts, history,
splits, advanced body editors, and vault controls are not connected to this screen yet.

If controls appear without text, see [desktop troubleshooting](docs/testing/desktop-workbench.md).

### Common Developer Commands

```bash
# Check all workspace crates
cargo check --workspace

# Run developer automation tasks
cargo xtask --help
cargo xtask fmt
cargo xtask lint
cargo xtask test
```

---

## Documentation

- [Product Principles](docs/principles.md)
- [Architecture Overview](docs/architecture/overview.md)
- [Crate Boundaries & Dependency Rules](docs/architecture/crate_map.md)
- [GPUI Pinning & Upgrade Strategy](docs/architecture/gpui_upgrade.md)
- [Architecture Decision Records (ADRs)](docs/adr/)

---

## License

PacketSmith is licensed under the [Apache License 2.0](LICENSE).
