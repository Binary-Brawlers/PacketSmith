# PacketSmith Product Principles

PacketSmith is designed around uncompromising core product principles that dictate architectural and product choices.

---

## 1. Local-First by Default
- All core functionalities—sending requests, creating and organizing collections, managing environments, running local tests, and inspecting history—work completely offline.
- No remote account or login is required to use PacketSmith.
- User data remains on the local machine unless remote synchronization or collaboration is explicitly enabled.

## 2. Open Workspace Formats & Source-Control Friendly
- Workspaces and requests are stored as structured, human-readable files (YAML/JSON) inside the file system.
- Files produce clean, deterministic Git diffs.
- Avoid opaque SQLite databases as the single source of truth for user-authored API resources. SQLite is used strictly for derived cache, indices, execution history, and transient UI state.

## 3. Secret-Safe by Design
- Secrets, tokens, and credentials are never written to unencrypted project files, commit history, or debug logs.
- Secrets are stored in the OS keychain or an encrypted local vault.
- Exporting collections or environments strips or redacts secrets by default.
- Masked headers and values are protected in console output.

## 4. Desktop and CLI Core Parity
- The headless CLI and desktop application share identical domain models, execution engines, and storage layers.
- Any request, collection run, or test suite that can be run in the desktop GUI can be executed in CI/CD via the CLI with identical behavior.

## 5. Protocol-Independent Core
- PacketSmith treats protocols as modular engines rather than forcing all protocols into an HTTP shape.
- Core architecture natively accommodates REST/HTTP, GraphQL, gRPC, WebSocket, Socket.IO, MQTT, MCP, AI endpoints, and SOAP.

## 6. Native Performance & Ergonomics
- Written in Rust with GPU-accelerated UI via GPUI.
- Fast cold startup (< 1s for typical workspaces), low memory consumption, and non-blocking streaming for massive payloads.
- Keyboard-first interaction model with command palette, customizable keymaps, split panes, and tabs.

## 7. Explicit Cloud & Optional Telemetry
- Telemetry is strictly opt-in, fully transparent, and anonymized.
- Cloud features (such as team collaboration or hosted mocks) enhance the platform but are never required for local productivity.
