# PacketSmith — Open-Source API Development Platform
## Detailed Product, Architecture, and Engineering Plan

> **Working codename:** PacketSmith  
> **Primary implementation language:** Rust  
> **Desktop UI:** Zed GPUI  
> **Primary platforms:** macOS, Windows, Linux  
> **Product model:** Local-first, offline-first, open-source, Git-friendly, extensible  
> **Document status:** Initial master implementation plan  
> **Last updated:** September 2026

---

# 1. Executive Summary

PacketSmith is an open-source, native desktop API development platform intended to cover the complete API lifecycle: designing APIs, creating and sending requests, debugging traffic, managing environments and secrets, testing APIs, running collections, performance testing, mocking APIs, monitoring APIs, generating documentation, collaborating with teams, importing/exporting common API formats, and automating workflows through a CLI and plugin system.

The product should compete with full API platforms rather than behave like a thin REST client.

The most important differentiators should be:

1. **Local-first by default**
   - No account required.
   - Core functionality works completely offline.
   - User data remains on the user's machine unless sync is explicitly enabled.

2. **Open data formats**
   - Workspaces can be stored as human-readable files.
   - Git diffs should be meaningful.
   - No proprietary lock-in for core requests, environments, examples, scripts, or API specs.

3. **Native performance**
   - Rust for the core.
   - GPUI for a GPU-accelerated desktop UI.
   - Streaming response rendering.
   - Low idle memory.
   - Fast startup even with large workspaces.

4. **Protocol-first architecture**
   - HTTP/REST
   - GraphQL
   - gRPC
   - WebSocket
   - Socket.IO
   - MQTT
   - SOAP
   - Server-Sent Events
   - Unix domain sockets / named pipes
   - MCP
   - AI model APIs
   - Future protocols through plugins

5. **Extensibility**
   - Protocol adapters.
   - Import/export adapters.
   - Authentication providers.
   - Code generators.
   - Scripting packages.
   - UI extensions where safe.
   - Custom response renderers.

6. **Power-user ergonomics**
   - Command palette.
   - Keyboard-first interaction.
   - Tabs and split panes.
   - Global search.
   - Request history.
   - Fast environment switching.
   - Built-in console.
   - Git workflows.
   - CLI parity for automation.

The project should not begin by trying to implement every feature simultaneously. It should be architected for the complete vision from day one, but delivered through staged vertical slices.

---

# 2. Product Goals

## 2.1 Primary Goals

PacketSmith should allow a developer to:

- Create a workspace without registering.
- Send a request in seconds.
- Save requests into collections.
- Configure reusable variables and environments.
- Securely store credentials.
- Test responses with scripts and assertions.
- Run entire API test suites.
- Import projects from Postman, Insomnia, Bruno, Hoppscotch, OpenAPI, cURL, HAR, and other common formats.
- Export work without vendor lock-in.
- Debug API traffic.
- Work with REST and non-REST protocols.
- Generate API documentation.
- Run mock servers.
- Schedule API health monitors.
- Perform load and performance testing.
- Generate code snippets.
- Work directly with API specifications.
- Store work in Git.
- Collaborate in real time when desired.
- Run the same collections from a CLI in CI/CD.
- Extend the application without modifying the core.

## 2.2 Non-Goals for Early Releases

The following should be architecturally anticipated but should not block the initial usable release:

- Hosted enterprise control plane.
- Large-scale SaaS billing.
- Enterprise SSO/SAML/SCIM.
- Public API marketplace.
- Hosted global monitoring infrastructure.
- Hosted large-scale performance testing.
- Full browser extension ecosystem.
- AI features that require the maintainers to pay inference costs.

Those belong after the local desktop product is excellent.

---

# 3. Product Principles

## 3.1 Local-First

The application must remain useful with:

- no internet connection,
- no PacketSmith account,
- no hosted backend,
- no telemetry.

Hosted services should enhance the product rather than unlock basic functionality.

## 3.2 Source-Control Friendly

A user should be able to place a workspace inside a Git repository and review request changes like code.

Avoid using an opaque SQLite database as the only source of truth for user-authored API definitions.

Recommended model:

- **Human-authored resources:** files.
- **Derived indexes/cache/search/history:** SQLite.
- **Secrets:** OS keychain or encrypted vault.
- **Large binary artifacts:** object/blob storage directory.
- **Ephemeral UI state:** SQLite/settings file.

## 3.3 Secure by Default

- Never write secrets to logs.
- Never include secrets in exported work unless explicitly requested.
- Provide domain restrictions for secrets.
- Redact known credentials in the console.
- Support certificate verification by default.
- Make "disable TLS verification" visually explicit.
- Sandboxed scripts.
- Permissioned extensions.
- Explicit network permissions for scripts/plugins.

## 3.4 Fast Regardless of Workspace Size

Design targets:

- Cold startup: under 1 second on modern hardware for small workspaces.
- Large workspace startup should not parse every body/script synchronously.
- Virtualize long lists.
- Stream large responses.
- Avoid duplicating response bodies in memory.
- Lazy-load historical runs.
- Incrementally index changed files.

## 3.5 Interoperability Over Lock-In

Prefer compatibility with:

- Postman Collection v2.1
- OpenAPI 3.x
- AsyncAPI
- GraphQL SDL
- Protobuf
- cURL
- HAR
- WSDL/SOAP metadata where feasible
- common environment formats
- JUnit results
- JSON reports

---

# 4. Current Competitive Feature Baseline

The project should account for the major feature categories found in modern API platforms.

## 4.1 API Client

Support:

- HTTP
- REST
- GraphQL
- gRPC
- WebSocket
- Socket.IO
- MQTT
- SOAP
- SSE
- MCP
- AI model requests
- Unix domain sockets
- Windows named pipes where feasible

## 4.2 API Organization

- Workspaces
- Collections
- Nested folders
- Requests
- Saved response examples
- Environments
- Variables
- Global/workspace variables
- Collection variables
- Request-local variables
- Dynamic variables
- Secret references
- Tags
- Favorites
- Recently used
- Search

## 4.3 API Testing

- Pre-request scripts
- Post-response scripts
- Assertions
- Test results
- Data-driven collection runs
- Request chaining
- Variable mutation
- Reusable script packages
- Collection/folder/request script inheritance
- CLI execution
- CI-compatible reports

## 4.4 API Design

- OpenAPI editor
- AsyncAPI editor
- GraphQL schema editor
- Protobuf editor
- Smithy support as a later plugin/module
- Validation
- Linting
- Collection generation from specs
- Spec generation from collections
- Contract tests
- Schema-based request validation
- Schema-based response validation

## 4.5 Collaboration

- Git-native workflows
- Optional real-time sync
- Comments
- Activity history
- Diffs
- Reviews
- Fork/branch-like collaboration
- Team permissions
- Shared environments
- Shared secret references
- Conflict resolution

## 4.6 Operations

- Mock servers
- Scheduled monitors
- Alerts
- Run history
- Performance tests
- Traffic capture / proxy
- Cookie management
- Certificates
- DNS and network diagnostics

## 4.7 Developer Productivity

- Import/export
- Code snippets
- API docs
- Command palette
- CLI
- Plugin system
- Templates
- Console
- Git integration
- AI assistance as an optional layer

---

# 5. Recommended Product Editions

The open-source core should remain extremely capable.

## 5.1 PacketSmith Desktop

The main GPUI application.

Contains:

- API clients.
- local workspaces.
- collections.
- environments.
- scripting.
- runner.
- local mock servers.
- local monitors while app/daemon is running.
- local performance testing.
- specs.
- docs.
- traffic capture.
- import/export.
- Git integration.
- local vault.
- plugins.

## 5.2 PacketSmith CLI

Headless executable.

Example conceptual commands:

```bash
packetsmith request send ./requests/login.req.yaml
packetsmith collection run ./collections/auth
packetsmith test ./workspace
packetsmith mock start ./mocks/users.yaml
packetsmith monitor run api-health
packetsmith import postman collection.json
packetsmith export openapi ./collections/users
packetsmith lint ./specs/openapi.yaml
packetsmith env use staging
```

The CLI should call the same core crates as the desktop app rather than maintain a second implementation.

## 5.3 PacketSmith Daemon

Optional local background service.

Responsibilities:

- Scheduled local monitors.
- Long-running mock servers.
- traffic capture.
- collaboration/sync tasks.
- extension workers.
- optional local webhooks.
- optionally expose a local control API to the desktop app and CLI.

## 5.4 PacketSmith Server

Optional self-hostable collaboration backend.

Responsibilities:

- accounts.
- organizations.
- teams.
- workspace sync.
- RBAC.
- comments.
- presence.
- shared vault metadata.
- monitor scheduling.
- webhook delivery.
- hosted mocks if deployed publicly.

Do not require this server for local usage.

---

# 6. High-Level System Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                     GPUI Desktop Shell                      │
├─────────────────────────────────────────────────────────────┤
│ UI State │ Commands │ Panels │ Editors │ Themes │ Keymaps   │
├─────────────────────────────────────────────────────────────┤
│                     Application Services                    │
│ Workspaces | History | Runner | Specs | Mocks | Monitors    │
├─────────────────────────────────────────────────────────────┤
│                        Domain Model                         │
│ Requests | Collections | Vars | Auth | Scripts | Examples   │
├─────────────────────────────────────────────────────────────┤
│                      Protocol Engines                       │
│ HTTP | GraphQL | gRPC | WS | Socket.IO | MQTT | MCP | AI   │
├─────────────────────────────────────────────────────────────┤
│        Runtime Services / Security / Persistence            │
│ JS Runtime | Vault | SQLite | Files | Git | Search | Logs   │
├─────────────────────────────────────────────────────────────┤
│                 OS / Network / Crypto Layer                 │
└─────────────────────────────────────────────────────────────┘
```

The UI must never directly own networking, persistence, or protocol behavior.

Use command/service boundaries so the same behavior can be used by:

- GPUI desktop,
- CLI,
- daemon,
- tests,
- future editor extensions.

---

# 7. Rust Workspace Structure

Recommended initial Cargo workspace:

```text
packetsmith/
├── Cargo.toml
├── rust-toolchain.toml
├── crates/
│   ├── packetsmith-app/
│   ├── packetsmith-cli/
│   ├── packetsmith-daemon/
│   ├── ps-ui/
│   ├── ps-ui-components/
│   ├── ps-command/
│   ├── ps-domain/
│   ├── ps-workspace/
│   ├── ps-storage/
│   ├── ps-search/
│   ├── ps-history/
│   ├── ps-vault/
│   ├── ps-variable/
│   ├── ps-auth/
│   ├── ps-request-engine/
│   ├── ps-http/
│   ├── ps-graphql/
│   ├── ps-grpc/
│   ├── ps-websocket/
│   ├── ps-socketio/
│   ├── ps-mqtt/
│   ├── ps-mcp/
│   ├── ps-ai/
│   ├── ps-soap/
│   ├── ps-sse/
│   ├── ps-script-runtime/
│   ├── ps-test-engine/
│   ├── ps-runner/
│   ├── ps-loadtest/
│   ├── ps-mock/
│   ├── ps-monitor/
│   ├── ps-spec/
│   ├── ps-docs/
│   ├── ps-import/
│   ├── ps-export/
│   ├── ps-codegen/
│   ├── ps-capture/
│   ├── ps-git/
│   ├── ps-sync/
│   ├── ps-collab/
│   ├── ps-plugin-api/
│   ├── ps-plugin-host/
│   ├── ps-settings/
│   ├── ps-telemetry/
│   └── ps-test-support/
├── schemas/
├── docs/
├── fixtures/
├── examples/
├── scripts/
└── installers/
```

Do not create every crate on day one if it slows development. The boundaries above are the target modular architecture.

---

# 8. GPUI Architecture

## 8.1 GPUI Usage Strategy

Use GPUI for:

- application shell,
- windows,
- panels,
- menus,
- tab strips,
- split panes,
- list virtualization,
- response rendering,
- keyboard handling,
- command palette,
- dialogs,
- notifications,
- settings UI.

Create a thin internal component/design-system layer.

Example:

```text
ps-ui-components
├── button
├── icon_button
├── text_input
├── code_input
├── dropdown
├── table
├── virtual_list
├── tree
├── tabs
├── split_pane
├── modal
├── popover
├── tooltip
├── toast
├── command_palette
├── context_menu
├── badge
├── progress
└── charts
```

This prevents the application from becoming coupled to raw GPUI details.

## 8.2 GPUI Versioning Rule

Pin GPUI deliberately.

Recommended strategy:

- Use a fixed released GPUI version where possible.
- If a Git revision is required, pin the exact commit.
- Update GPUI in isolated upgrade PRs.
- Do not depend unnecessarily on GPL-licensed Zed application crates.
- Confirm the license of every Zed crate before reusing it.
- GPUI itself is Apache-2.0, while much of the Zed application is GPL-3.0-or-later.

## 8.3 Application State

Suggested state roots:

```text
AppState
├── WorkspaceManager
├── WindowManager
├── CommandRegistry
├── SettingsStore
├── ThemeRegistry
├── KeymapRegistry
├── RequestExecutionService
├── HistoryService
├── VaultService
├── PluginManager
└── SyncManager
```

Per workspace:

```text
WorkspaceState
├── ResourceTree
├── OpenTabs
├── ActiveEnvironment
├── GitState
├── SearchIndex
├── PendingChanges
└── CollaborationState
```

Per request tab:

```text
RequestTabState
├── DraftRequest
├── DirtyState
├── ResolvedPreview
├── ActiveRun
├── ResponseState
├── ConsoleEvents
└── TestResults
```

---

# 9. Main Desktop UI

## 9.1 Main Window Layout

```text
┌───────────────────────────────────────────────────────────────┐
│ Title Bar / Workspace / Environment / Search / Commands       │
├───────────────┬───────────────────────────────────────────────┤
│ Sidebar       │ Tab Strip                                     │
│               ├───────────────────────────────────────────────┤
│ Collections   │ Request Builder                               │
│ Environments  │                                               │
│ Specs         │ URL / endpoint / method / protocol controls   │
│ Mocks         │ Params | Auth | Headers | Body | Scripts      │
│ Monitors      ├───────────────────────────────────────────────┤
│ Flows         │ Response                                      │
│ History       │ Body | Headers | Cookies | Tests | Timing     │
│ Git           │                                               │
├───────────────┴───────────────────────────────────────────────┤
│ Console / Network / Logs / Problems                           │
└───────────────────────────────────────────────────────────────┘
```

## 9.2 Required UX Behaviors

- Multiple tabs.
- Pin tabs.
- Duplicate tabs.
- Reopen closed tab.
- Dirty markers.
- Split vertically/horizontally.
- Drag tabs between splits.
- Detachable windows later.
- Breadcrumb navigation.
- Context menus everywhere relevant.
- Command palette.
- Search by request name, URL, header, script content, spec operation.
- Recent workspaces.
- Quick Open.
- Keyboard shortcuts.
- Configurable keymaps.
- Light/dark/system theme.
- UI zoom.
- Density setting.
- Autosave toggle.
- crash recovery for unsaved tabs.

---

# 10. Request Domain Model

Use a protocol-neutral request envelope.

```rust
pub struct RequestDocument {
    pub id: ResourceId,
    pub name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub protocol: ProtocolRequest,
    pub auth: AuthConfig,
    pub scripts: RequestScripts,
    pub settings: RequestSettings,
    pub examples: Vec<ResponseExampleRef>,
}
```

Conceptual protocol enum:

```rust
pub enum ProtocolRequest {
    Http(HttpRequest),
    GraphQl(GraphQlRequest),
    Grpc(GrpcRequest),
    WebSocket(WebSocketRequest),
    SocketIo(SocketIoRequest),
    Mqtt(MqttRequest),
    Mcp(McpRequest),
    Ai(AiRequest),
    Soap(SoapRequest),
}
```

Avoid forcing every protocol into HTTP-shaped fields.

---

# 11. HTTP Client

## 11.1 HTTP Methods

Support standard and custom methods:

- GET
- POST
- PUT
- PATCH
- DELETE
- HEAD
- OPTIONS
- TRACE
- CONNECT
- custom method string

## 11.2 URL Builder

Features:

- raw URL editor.
- parsed query parameter table.
- path variable detection.
- variable interpolation.
- percent encoding.
- enable/disable individual parameters.
- repeated query keys.
- URL normalization.
- resolved URL preview.
- copy resolved URL.
- URL encode/decode utilities.

## 11.3 Headers

- ordered key/value rows.
- enable/disable.
- duplicate names.
- auto-generated headers shown separately.
- hidden sensitive header values.
- presets.
- bulk edit mode.
- variable interpolation.
- header descriptions from specs.

## 11.4 Request Bodies

Support:

- none.
- raw text.
- JSON.
- JSON5 optional.
- XML.
- HTML.
- JavaScript.
- plain text.
- GraphQL.
- `application/x-www-form-urlencoded`.
- multipart form-data.
- binary file.
- streamed file.
- custom content type.

Multipart fields:

- text.
- file.
- multiple files.
- custom content type.
- custom filename.
- per-part headers.

## 11.5 HTTP Versions

Support where networking stack allows:

- HTTP/1.0
- HTTP/1.1
- HTTP/2
- HTTP/3 later

Expose negotiated protocol in response diagnostics.

## 11.6 Redirects

Options:

- follow redirects.
- maximum redirects.
- preserve/drop authorization on cross-origin redirects.
- show redirect chain.
- resend method behavior.
- manually follow next redirect.

## 11.7 Compression

Handle:

- gzip
- brotli
- deflate
- zstd where supported

Allow inspecting raw/compressed metadata.

## 11.8 TLS

- certificate verification on by default.
- custom CA bundle.
- client certificate.
- private key.
- PKCS#12/PFX.
- certificate passphrase.
- SNI.
- TLS version diagnostics.
- certificate chain inspector.
- expiry warnings.

## 11.9 Proxy

Support:

- system proxy.
- HTTP proxy.
- HTTPS proxy.
- SOCKS5.
- proxy auth.
- `NO_PROXY`.
- per-request override.
- PAC later.

---

# 12. Authentication System

Authentication should use a provider interface.

```rust
pub trait AuthProvider {
    fn id(&self) -> &'static str;
    fn apply(&self, request: &mut PreparedRequest, ctx: &AuthContext)
        -> Result<()>;
}
```

Support:

- No Auth
- Inherit Auth
- API Key
- Bearer Token
- Basic Auth
- Digest Auth
- OAuth 1.0
- OAuth 2.0
- JWT Bearer
- AWS Signature v4
- Hawk
- NTLM
- Akamai EdgeGrid later
- custom plugin auth

## 12.1 OAuth 2.0

Support:

- Authorization Code
- Authorization Code + PKCE
- Client Credentials
- Device Authorization
- Resource Owner Password only for compatibility
- Implicit only for legacy compatibility
- token refresh.
- token expiry.
- scopes.
- audience.
- custom auth parameters.
- callback handling using localhost callback.
- browser launch.
- token sharing policies.
- store tokens in vault rather than workspace files.

## 12.2 Auth Inheritance

Auth may exist at:

- workspace default.
- collection.
- folder.
- request.

A request can inherit the closest ancestor configuration.

---

# 13. Variables and Environments

## 13.1 Variable Scopes

Support:

- built-in dynamic values.
- local/temporary variables.
- iteration data.
- request variables.
- environment variables.
- collection variables.
- workspace/global variables.
- vault secrets.

## 13.2 Variable Types

- string
- number
- boolean
- JSON
- secret reference

## 13.3 Variable Features

- initial/default value.
- current/local value.
- secure flag.
- description.
- type.
- enabled/disabled.
- masked values.
- import/export.
- duplicate key warnings.
- unresolved variable warnings.
- hover preview.
- jump to definition.
- usage search.

## 13.4 Dynamic Variables

Provide built-ins such as:

- UUID.
- timestamp.
- ISO timestamp.
- random integer.
- random string.
- random email.
- random first/last name.
- random IP.
- random MAC.
- random color.
- random date.
- cryptographically secure bytes.

Make the dynamic generator extensible.

## 13.5 Resolution Engine

Variable resolution must:

1. tokenize templates.
2. identify references.
3. resolve by scope precedence.
4. prevent accidental recursive loops.
5. support nested object lookup where appropriate.
6. return both final value and provenance.
7. maintain redaction metadata so secrets remain secret after interpolation.

Never resolve secrets into logs.

---

# 14. Secure Vault

## 14.1 Storage

Use platform secure storage where available:

- macOS Keychain.
- Windows Credential Manager.
- Linux Secret Service/libsecret.

For a portable encrypted vault:

- authenticated encryption.
- key derived from user passphrase or stored OS key.
- versioned format.
- salt.
- nonce per record.
- secure memory handling where practical.

## 14.2 Vault Features

- create/edit/delete secret.
- allowed domains.
- tags.
- copy with timed clipboard clearing.
- reveal with deliberate action.
- secret references in requests.
- secret references in scripts.
- never sync local vault by default.
- encrypted export.
- optional plaintext export with warning.
- external vault adapters later:
  - HashiCorp Vault
  - 1Password CLI
  - AWS Secrets Manager
  - GCP Secret Manager
  - Azure Key Vault

---

# 15. Cookie System

Implement an RFC-aware cookie jar.

Features:

- per-domain cookies.
- path matching.
- Secure.
- HttpOnly.
- SameSite.
- expiry.
- session cookies.
- manual edit.
- enable/disable cookie sending.
- import/export cookie jar.
- clear by domain.
- inspect cookies received during a response.
- persistent and incognito jars.

---

# 16. Response Viewer

## 16.1 Response Metadata

Show:

- status code and status text.
- response time.
- response size.
- content type.
- remote IP.
- negotiated HTTP version.
- TLS information.
- redirect count.
- DNS/connect/TLS/TTFB/download timing when available.

## 16.2 Response Body Views

- Pretty.
- Raw.
- Preview.
- Hex.
- Visualize.
- Diff.

Pretty formatters:

- JSON.
- XML.
- HTML.
- CSS.
- JavaScript.
- YAML.
- GraphQL.
- protobuf JSON representation.

## 16.3 Large Responses

Requirements:

- stream to disk over a configured threshold.
- incremental byte counter.
- cancellable download.
- virtualized text rendering.
- line indexing.
- avoid loading entire response into a `String`.
- warning for extremely large syntax highlighting.
- optional sampling.
- save body to file.

## 16.4 Response Search

- plain text.
- regex.
- case sensitivity.
- JSONPath.
- XPath.
- JMESPath later.
- copy selected subtree.
- collapse/expand JSON nodes.

## 16.5 Response Comparison

Allow comparing:

- current vs previous response.
- two saved examples.
- environment A vs B.
- status.
- headers.
- JSON semantic diff.
- text diff.
- timing.

---

# 17. Request History

Every execution can create a history entry containing:

- timestamp.
- request ID if saved.
- resolved URL with secrets redacted.
- method/protocol.
- response status.
- duration.
- response size.
- environment.
- request snapshot metadata.
- failure type.
- optional response body reference.

History features:

- search.
- filter.
- group by day.
- filter by workspace.
- filter by status.
- replay.
- save to collection.
- compare.
- favorite.
- configurable retention.
- clear.
- incognito mode.

---

# 18. Collections

## 18.1 Structure

Collections can contain:

- folders.
- nested folders.
- requests.
- scripts.
- auth.
- variables.
- documentation.
- test configuration.
- runner defaults.

## 18.2 Collection Operations

- create.
- rename.
- duplicate.
- move.
- reorder.
- drag/drop.
- export.
- import.
- fork/branch later.
- archive.
- search.
- bulk edit.
- generate docs.
- run.
- mock.
- monitor.

## 18.3 Saved Examples

A response example should include:

- name.
- request snapshot.
- status.
- headers.
- response body.
- content type.
- example-specific documentation.

Examples become inputs for:

- mocks.
- docs.
- schema inference.
- tests.

---

# 19. Scripting Runtime

Postman-style API testing requires JavaScript compatibility.

## 19.1 Runtime Architecture

Do not run user scripts in the GPUI process.

Recommended:

```text
Desktop/CLI
   │
   ├── IPC
   ▼
Script Worker Process
   ├── JS engine
   ├── sandbox API
   ├── package loader
   └── resource limits
```

Benefits:

- crashes do not crash the UI.
- memory limits.
- timeouts.
- kill hung scripts.
- tighter permissions.

## 19.2 Engine Choice

Evaluate:

### Option A — `deno_core` / V8
Pros:
- modern JavaScript compatibility.
- strong ES feature support.
- easier path to package loading.
- mature runtime semantics.

Cons:
- large binary.
- heavier build.
- memory overhead.

### Option B — QuickJS via `rquickjs`
Pros:
- much smaller.
- fast startup.
- simpler embedding.

Cons:
- package compatibility work.
- less compatibility with some npm expectations.

### Recommendation

Start with **QuickJS/rquickjs** for the first scripting engine abstraction, but keep the runtime behind a process-level protocol so a V8 worker can be introduced without changing the product model.

If maximum Postman script compatibility is the highest priority, use V8/`deno_core` from the beginning.

## 19.3 Script Phases

Support:

- collection pre-request.
- folder pre-request.
- request pre-request.
- request execution.
- request post-response.
- folder post-response.
- collection post-response.

Define exact ordering and test it thoroughly.

## 19.4 Script API

Provide a compatibility-oriented API, initially:

```javascript
ps.test(...)
ps.expect(...)
ps.request
ps.response
ps.variables
ps.environment
ps.collectionVariables
ps.globals
ps.cookies
ps.sendRequest(...)
ps.execution
ps.vault
ps.visualizer
```

A compatibility namespace such as `pm` can be added where legally and technically appropriate for imported Postman collections.

## 19.5 Script Security

Per script execution:

- CPU timeout.
- wall timeout.
- memory ceiling.
- network permission.
- file system disabled by default.
- environment process variables disabled by default.
- subprocess spawning disabled.
- secret access requires explicit API.
- redact values from exceptions.

---

# 20. Testing and Assertions

## 20.1 Test API

Support assertions for:

- status.
- response time.
- headers.
- cookies.
- body.
- JSON schema.
- XML.
- strings.
- arrays.
- objects.
- numeric comparisons.
- custom predicates.

## 20.2 Test Output

Each request displays:

- passed.
- failed.
- skipped.
- duration.
- failure message.
- source line.
- stack trace.
- logs.

## 20.3 Contract Testing

Allow validation against:

- OpenAPI request schema.
- OpenAPI response schema.
- GraphQL schema.
- protobuf message type.
- JSON Schema.
- user-defined schema.

---

# 21. Collection Runner

The collection runner should support:

- entire collection.
- folder subset.
- selected requests.
- ordering.
- iterations.
- delay between requests.
- data file.
- environment.
- stop on failure.
- keep running on failure.
- retry policy.
- concurrency mode later.
- save response bodies optionally.
- persist variable changes optionally.
- run summary.

Data files:

- CSV.
- JSON.
- NDJSON later.

Outputs:

- interactive UI report.
- JSON.
- JUnit XML.
- HTML report through plugin.
- machine-readable event stream.

---

# 22. Performance / Load Testing

Implement as a separate engine from ordinary collection running.

## 22.1 Test Models

- fixed virtual users.
- ramp-up.
- ramp-down.
- stages.
- constant arrival rate later.
- target throughput later.

## 22.2 Metrics

Capture:

- requests/sec.
- iterations/sec.
- error rate.
- bytes/sec.
- p50.
- p75.
- p90.
- p95.
- p99.
- min/max.
- active virtual users.
- status distribution.
- test assertion failures.

## 22.3 Safety

- clear local-machine resource warnings.
- max configurable VUs.
- target confirmation for public hosts at high load.
- cancellation.
- connection limits.
- backpressure.
- no infinite body retention.

## 22.4 Export

- JSON summary.
- CSV metrics.
- JUnit for assertions.
- Prometheus/OpenTelemetry output later.

---

# 23. GraphQL Client

Features:

- endpoint.
- query editor.
- variables editor.
- headers.
- auth.
- introspection.
- schema explorer.
- autocomplete.
- field documentation.
- validation.
- fragments.
- operation selector.
- saved queries.
- query history.
- subscriptions over WebSocket.
- generate queries from schema.
- prettify/minify.

Use Tree-sitter or a GraphQL parser for syntax-aware editing.

---

# 24. gRPC Client

Features:

- import `.proto` files.
- import proto directory.
- resolve imports.
- server reflection.
- service browser.
- method browser.
- message editor.
- request validation.
- metadata.
- auth.
- unary.
- server streaming.
- client streaming.
- bidirectional streaming.
- TLS.
- client certificates.
- deadlines.
- cancellation.
- message timeline.
- save examples.

Recommended core technologies:

- tonic.
- prost.
- prost-reflect or equivalent dynamic descriptors.

---

# 25. WebSocket Client

Features:

- ws/wss.
- headers.
- query parameters.
- auth.
- cookies.
- subprotocols.
- connect/disconnect/reconnect.
- text messages.
- binary messages.
- message history.
- timestamp.
- direction indicator.
- filter.
- search.
- save reusable messages.
- variable interpolation.
- ping/pong.
- connection diagnostics.

---

# 26. Socket.IO Client

Features:

- Socket.IO protocol version handling.
- namespaces.
- connect params.
- custom headers.
- auth payload.
- emit event.
- listen for events.
- acknowledgements.
- binary payloads.
- event history.
- saved event templates.
- auto-reconnect.

Keep Socket.IO separate from raw WebSockets.

---

# 27. MQTT Client

Features:

- MQTT 3.1.1.
- MQTT 5 where dependency supports it.
- TCP.
- TLS.
- WebSocket transport later.
- broker URL.
- client ID.
- username/password.
- client cert.
- clean session/start.
- keepalive.
- QoS 0/1/2.
- retained flag.
- topic subscriptions.
- wildcard topics.
- publish.
- last will.
- properties for MQTT 5.
- live message stream.
- message filtering.
- saved publishers/subscriptions.

---

# 28. MCP Client

MCP should be treated as a first-class protocol.

Support:

- stdio server launch.
- Streamable HTTP.
- server capabilities.
- initialize handshake.
- tools/list.
- tools/call.
- resources/list.
- resources/read.
- prompts/list.
- prompts/get.
- logging/notifications.
- structured result viewer.
- JSON-RPC inspector.
- saved server configurations.
- environment variables and secret references.
- process lifecycle.
- permission prompts for launching local servers.

Later:

- MCP server testing suites.
- mock MCP server.
- protocol trace viewer.

---

# 29. AI Request Client

Optional but useful for parity with modern API clients.

Support provider presets:

- OpenAI-compatible.
- Anthropic.
- Gemini.
- Ollama.
- LM Studio.
- custom OpenAI-compatible endpoint.

Features:

- model selector.
- messages.
- system prompt.
- tool definitions.
- streaming.
- temperature.
- top-p.
- max tokens.
- structured output.
- images/files where provider supports them.
- usage/cost metadata.
- raw HTTP inspection.
- response comparison.
- evaluation test assertions.
- attach configured MCP servers later.

Do not force a hosted PacketSmith AI service. BYOK should be the default.

---

# 30. SOAP

Support through the HTTP engine plus SOAP-specific tooling:

- WSDL import.
- service/method explorer.
- XML request templates.
- SOAPAction.
- namespaces.
- WS-Security basic support later.
- schema validation.
- formatted XML response.

---

# 31. SSE

Support:

- standard HTTP setup.
- event stream viewer.
- event ID.
- event type.
- data.
- reconnect behavior.
- copy/save events.
- search/filter.
- timing.

---

# 32. API Specifications

## 32.1 Supported Specs

Initial:

- OpenAPI 3.0 / 3.1.
- Swagger 2.0 import.
- GraphQL SDL.
- protobuf.

Later:

- AsyncAPI.
- Smithy.

## 32.2 Spec Editor

Features:

- syntax highlighting.
- validation.
- diagnostics.
- outline.
- autocomplete.
- hover documentation.
- jump to reference.
- `$ref` resolution.
- multi-file specs.
- local and remote references.
- formatting.
- lint rules.
- preview.
- operation list.

## 32.3 Collection Generation

Generate collections from:

- paths.
- operations.
- servers.
- examples.
- schemas.
- auth definitions.

## 32.4 Synchronization

Track generation metadata so users can:

- regenerate.
- preview changes.
- preserve local scripts where possible.
- resolve conflicts.
- update request definitions without replacing everything.

---

# 33. API Documentation

Generate documentation from:

- collection descriptions.
- folder descriptions.
- request descriptions.
- examples.
- schemas.
- authentication.
- parameters.
- scripts/test summaries.

Documentation modes:

- built-in live preview.
- static site export.
- Markdown export.
- self-hosted documentation server.

Features:

- code samples.
- language selector.
- search.
- navigation.
- try-it console later.
- theming.
- custom logo.
- versioning.
- OpenAPI-derived reference pages.

---

# 34. Mock Servers

## 34.1 Local Mock Server

Users can map:

- method + path.
- query parameters.
- headers.
- request body matchers.
- response status.
- response headers.
- response body.
- latency.
- probabilistic errors.
- dynamic templating.

## 34.2 Matching Priority

Define deterministic matching:

1. method/path.
2. required query.
3. selected headers.
4. request body matcher.
5. example priority.
6. fallback.

## 34.3 Dynamic Responses

Template variables:

- request path.
- query.
- headers.
- body.
- random data.
- environment variables.
- script-generated values.

## 34.4 Mock Log

Show:

- incoming request.
- matched rule.
- response.
- duration.
- unmatched reason.

---

# 35. Monitoring

## 35.1 Monitor Definition

A monitor references:

- collection/folder.
- environment.
- schedule.
- timeout.
- retry policy.
- alert destinations.
- execution location.

## 35.2 Local Monitoring

PacketSmith Daemon can execute schedules locally.

Support:

- cron.
- fixed interval.
- run now.
- pause.
- history.
- failure streak.
- uptime percentage.
- response time graph.

## 35.3 Alerts

Later adapters:

- email through configured SMTP.
- Slack webhook.
- Discord webhook.
- generic webhook.
- PagerDuty.
- Opsgenie.
- desktop notification.

## 35.4 Hosted Monitoring

The self-hosted server may coordinate distributed runners later.

---

# 36. Traffic Capture and Proxy

This is a major advanced feature and should be isolated in its own subsystem.

## 36.1 Capture Modes

- explicit local HTTP proxy.
- HTTPS MITM with locally generated CA.
- system proxy helper.
- HAR import.
- browser extension later.

## 36.2 Capture Session

A capture session stores:

- start/stop time.
- request.
- response.
- cookies.
- timing.
- remote destination.
- TLS metadata.
- process metadata where OS APIs allow it.

## 36.3 Filters

Filter by:

- host.
- path.
- method.
- status.
- content type.
- process.
- time range.
- header.

## 36.4 Save to Collection

Any captured request can become:

- new request.
- example.
- collection.
- sequence.

## 36.5 Security Requirements

- never silently install a CA.
- show exactly what HTTPS interception means.
- allow regeneration/removal.
- keep private CA key secured.
- never sync private CA keys.

---

# 37. Request Console and Network Diagnostics

Global console should capture:

- resolved request metadata.
- script logs.
- redirects.
- TLS errors.
- DNS failures.
- proxy behavior.
- connection retries.
- cookie changes.
- test failures.
- runtime warnings.

Network diagnostics panel can later expose:

- DNS lookup.
- TCP connect.
- TLS handshake.
- TTFB.
- transfer.
- remote IP.
- certificate subject.
- ALPN.
- HTTP version.

Secrets must be redacted before events enter the logging pipeline.

---

# 38. Visualizer

Provide response visualization through a safe renderer.

Possible model:

- user script produces a declarative visualization JSON model.
- built-in chart/table components render it natively.

Preferred over arbitrary HTML execution inside the main app.

Built-ins:

- table.
- JSON tree.
- key/value cards.
- bar chart.
- line chart.
- scatter chart.
- pie/donut.
- metric cards.
- Markdown.
- image preview.

If HTML visualizers are supported later, render them in a strongly sandboxed webview process.

---

# 39. Code Snippet Generation

Create an intermediate request representation then render language-specific snippets.

Initial generators:

- cURL.
- HTTPie.
- JavaScript `fetch`.
- Node.js `https`.
- Axios.
- Rust reqwest.
- Python requests.
- Python httpx.
- Go `net/http`.
- Java.
- Kotlin.
- C# HttpClient.
- PHP cURL.
- Ruby Net::HTTP.
- Swift URLSession.
- Dart http.

Codegen should be plugin-extensible.

Do not generate secrets unless the user explicitly selects "include resolved secrets."

---

# 40. Import System

Use an adapter interface.

```rust
pub trait Importer {
    fn detect(&self, input: &ImportInput) -> DetectionScore;
    fn inspect(&self, input: &ImportInput) -> Result<ImportPreview>;
    fn import(&self, input: &ImportInput, options: ImportOptions)
        -> Result<ImportResult>;
}
```

## 40.1 Initial Importers

- Postman Collection v2/v2.1.
- Postman environments.
- OpenAPI.
- Swagger.
- cURL.
- HAR.
- Insomnia.
- Bruno.
- Hoppscotch.
- Thunder Client.
- raw URL.
- GraphQL schema.
- protobuf.

## 40.2 Import Preview

Never immediately mutate the workspace.

Show:

- detected format.
- resources found.
- conflicts.
- unsupported fields.
- warnings.
- destination.
- conversion summary.

---

# 41. Export System

Export:

- PacketSmith native format.
- Postman Collection v2.1.
- OpenAPI.
- cURL.
- HAR where applicable.
- environments.
- run reports.
- docs.

Export must clearly distinguish:

- unresolved variables.
- resolved values.
- secret redaction.

---

# 42. Native Workspace File Format

Recommended top-level structure:

```text
my-api/
├── packetsmith.yaml
├── collections/
│   └── users/
│       ├── collection.yaml
│       ├── list-users.req.yaml
│       ├── create-user.req.yaml
│       └── examples/
├── environments/
│   ├── development.env.yaml
│   └── production.env.yaml
├── specs/
│   └── openapi.yaml
├── mocks/
├── monitors/
├── flows/
├── packages/
└── .packetsmith/
    ├── cache.db
    ├── history.db
    ├── blobs/
    └── state.json
```

`.packetsmith/` should normally be Git-ignored except for intentionally shareable metadata.

## 42.1 File Format Requirements

- version field.
- stable IDs.
- deterministic serialization.
- comments where YAML parser supports preservation, or avoid rewriting untouched files.
- no secret values.
- forward-compatible unknown-field handling.
- schema validation.
- migrations.

---

# 43. SQLite Usage

SQLite is recommended for:

- full-text search index.
- execution history.
- run results.
- cache.
- UI metadata.
- sync journal.
- activity events.
- blob metadata.
- recent files.
- diagnostics.

Suggested tables:

```text
history_entries
history_headers
runs
run_iterations
run_requests
run_tests
search_documents
workspace_state
tabs
recent_resources
sync_ops
blobs
monitor_runs
mock_logs
capture_sessions
capture_entries
```

Consider FTS5 for search.

---

# 44. Search

Global search should index:

- resource names.
- URLs.
- methods.
- descriptions.
- headers.
- request bodies.
- scripts.
- environment variable names.
- specs.
- docs.
- mock definitions.

Modes:

- fuzzy resource search.
- text search.
- regex search.
- filter syntax.

Example:

```text
type:request method:POST "login"
host:api.example.com status:500
type:spec users
```

---

# 45. Git Integration

## 45.1 Core Git Features

- detect repository.
- branch.
- status.
- changed resources.
- diff.
- stage.
- unstage.
- commit.
- history.
- blame later.
- pull/push via system Git initially.
- conflict detection.

## 45.2 Semantic Diff

For PacketSmith resources, show semantic differences:

- method changed.
- URL changed.
- header added.
- auth changed.
- script changed.
- example changed.

This is more useful than raw YAML.

## 45.3 Git Safety

Do not automatically commit.

Provide secret scanner before staging PacketSmith files.

---

# 46. Collaboration and Sync

Do not make collaboration a prerequisite for local architecture.

## 46.1 Sync Model

Recommended:

- each resource has stable ID.
- each mutation emits an operation/event.
- local file state remains authoritative for local workspace.
- sync layer sends resource revisions.
- server tracks versions.
- conflicts are explicit.

For advanced real-time collaborative editing, evaluate a CRDT layer only where it adds value:

- text/spec editing.
- descriptions.
- scripts.

Do not CRDT every domain object without need.

## 46.2 Presence

Optional:

- current workspace.
- current request/spec.
- cursor selection for shared editors later.
- user avatar/status.

## 46.3 Permissions

Roles:

- Owner.
- Admin.
- Editor.
- Commenter.
- Viewer.

Resource-level permissions can come later.

---

# 47. Comments, Reviews, and Activity

Support comments on:

- collection.
- folder.
- request.
- example.
- spec.
- line/range in spec or script later.

Activity timeline:

- created.
- edited.
- moved.
- deleted.
- restored.
- run.
- commented.
- imported.
- exported.
- sync conflict.
- branch change.

---

# 48. Trash and Recovery

Resources should use soft delete where practical.

Features:

- trash.
- restore.
- permanent delete.
- autosave.
- crash recovery.
- backup snapshots.
- migration rollback.

---

# 49. Flows / Visual API Workflows

This is a later major module.

## 49.1 Canvas

Node types:

- HTTP request.
- GraphQL request.
- gRPC request.
- condition.
- switch.
- loop.
- delay.
- transform.
- variable.
- JSON parse.
- script.
- output.
- input.
- webhook trigger.
- schedule trigger.

## 49.2 Execution

Use a DAG/runtime engine.

Requirements:

- deterministic execution.
- cancellation.
- node-level logs.
- retries.
- timeout.
- parallel branches.
- persisted run state.
- typed ports where possible.

Keep flow runtime independent from GPUI canvas rendering.

---

# 50. Plugin System

A strong plugin architecture is one of the highest-value differentiators.

## 50.1 Plugin Categories

- protocol adapter.
- auth provider.
- importer.
- exporter.
- code generator.
- response formatter.
- visualizer.
- spec linter.
- secret provider.
- notification provider.
- theme.
- command.
- dynamic variables.

## 50.2 Plugin Runtime

Prefer WebAssembly/WASI for third-party extensions.

Advantages:

- sandboxing.
- cross-platform.
- language-neutral plugin development.
- explicit permissions.

Plugin manifest:

```yaml
id: com.example.my-plugin
name: My Plugin
version: 1.0.0
api_version: 1
permissions:
  - network:api.example.com
  - workspace:read
capabilities:
  - importer
  - command
```

## 50.3 Permissions

Possible permissions:

- workspace read.
- workspace write.
- network domain.
- secrets by name.
- clipboard.
- notifications.
- local files selected by user.
- process execution should generally be forbidden.

---

# 51. Command System

Every meaningful UI action should be represented as a command/action.

Examples:

- New Request.
- Send Request.
- Cancel Request.
- Save Request.
- Duplicate Request.
- Switch Environment.
- Open Command Palette.
- Run Collection.
- Open Console.
- Toggle Sidebar.
- Format Body.
- Import.
- Export.

Benefits:

- keyboard mappings.
- menus.
- command palette.
- plugins.
- accessibility.
- testability.

---

# 52. Keyboard-First UX

Must support configurable shortcuts.

Suggested defaults:

- `Cmd/Ctrl + N` — new request.
- `Cmd/Ctrl + Enter` — send.
- `Cmd/Ctrl + S` — save.
- `Cmd/Ctrl + P` — quick open.
- `Cmd/Ctrl + Shift + P` — command palette.
- `Cmd/Ctrl + L` — focus URL.
- `Cmd/Ctrl + W` — close tab.
- `Cmd/Ctrl + Shift + T` — reopen tab.
- `Cmd/Ctrl + ,` — settings.
- `Cmd/Ctrl + J` — console.

Avoid hardcoding shortcuts inside view widgets.

---

# 53. Editor Components

PacketSmith needs editors for:

- URLs.
- JSON.
- XML.
- YAML.
- JavaScript.
- GraphQL.
- protobuf.
- Markdown.
- raw text.

Important licensing decision:

GPUI is Apache-2.0, but most of the Zed application/editor code is GPL-3.0-or-later. If PacketSmith does not want GPL obligations for the complete application, do not casually copy or link GPL Zed editor components.

Options:

1. Build a focused GPUI text/code editor component.
2. Use another permissively licensed text editing component if compatible.
3. License PacketSmith under GPL and intentionally reuse compatible Zed components after reviewing individual file licenses.

The code editor is substantial enough to treat as its own project subsystem.

Required editor features:

- Unicode.
- selection.
- clipboard.
- undo/redo.
- multi-cursor later.
- line numbers.
- scrolling.
- syntax highlighting.
- bracket matching.
- search/replace.
- formatting.
- diagnostics.
- autocomplete.
- large-file mode.

---

# 54. Syntax Parsing

Use Tree-sitter where helpful for:

- JSON.
- JavaScript.
- TypeScript if scripts expand.
- YAML.
- GraphQL.
- Markdown.
- XML if grammar quality is acceptable.

JSON formatting and parsing should still use a proper JSON parser rather than relying on Tree-sitter semantics.

---

# 55. Settings System

Settings layers:

1. defaults.
2. user.
3. workspace.
4. resource-specific.

Settings categories:

- appearance.
- theme.
- editor.
- network.
- TLS.
- proxy.
- privacy.
- telemetry.
- history.
- scripts.
- runner.
- performance tests.
- Git.
- sync.
- plugins.
- updates.

Store settings in a documented format.

---

# 56. Themes

Support:

- light.
- dark.
- system.
- custom themes.

Theme tokens:

- background.
- panel.
- elevated panel.
- border.
- text.
- muted text.
- accent.
- success.
- warning.
- error.
- HTTP method colors.
- editor tokens.
- chart tokens.

Do not let feature code hardcode colors.

---

# 57. Accessibility

Minimum requirements:

- keyboard navigation.
- focus indicators.
- screen reader metadata through GPUI accessibility APIs.
- scalable text.
- sufficient contrast.
- reduced motion.
- semantic roles.
- accessible error states.
- no information conveyed only by color.

---

# 58. Networking Architecture

Use a protocol-neutral execution API.

```rust
#[async_trait]
pub trait RequestExecutor {
    type Request;
    type Event;

    async fn execute(
        &self,
        request: Self::Request,
        ctx: ExecutionContext,
        sink: EventSink<Self::Event>,
    ) -> Result<ExecutionSummary>;
}
```

Execution should stream events:

```text
Preparing
ResolvingVariables
RunningPreRequestScript
Connecting
TlsHandshake
RequestHeadersSent
RequestBodyProgress
ResponseHeadersReceived
ResponseBodyChunk
RunningTests
Completed
Failed
Cancelled
```

This lets the UI update live without special-casing each engine.

---

# 59. Suggested Rust Dependencies

Exact dependencies should be validated before adoption.

Likely candidates:

## Core

- `serde`
- `serde_json`
- `serde_yaml` or maintained YAML alternative
- `thiserror`
- `anyhow` at app boundaries
- `tracing`
- `uuid`
- `url`
- `chrono` or `time`

## Async

- `tokio` for network/runtime services where compatible.
- channels from `tokio`, `async-channel`, or internal abstraction.

## HTTP

- `reqwest` for high-level client.
- `hyper` where lower-level control is needed.
- `rustls`.
- `tower`.
- `http`.
- `cookie` / cookie store implementation.

## gRPC

- `tonic`
- `prost`
- dynamic protobuf reflection library.

## WebSocket

- `tokio-tungstenite`

## MQTT

- `rumqttc` or another maintained MQTT crate.

## Storage

- `rusqlite` or `sqlx`.
- Prefer `rusqlite` if keeping SQLite operations in dedicated worker threads.
- Prefer `sqlx` if async DB access is valuable enough to justify it.

## Git

- start with system `git` process integration for maximum compatibility.
- evaluate `git2` or `gix` later.

## Search

- SQLite FTS5 initially.
- Tantivy later only if needed.

## Crypto

- audited RustCrypto primitives.
- zeroization where secrets pass through memory.

## CLI

- `clap`.

## File Watching

- `notify`.

## Parsing

- `tree-sitter`.
- dedicated OpenAPI/GraphQL/protobuf libraries.

---

# 60. Error Model

Use structured error categories.

Example:

```rust
pub enum ExecutionError {
    InvalidUrl,
    VariableResolution,
    Dns,
    Connect,
    Proxy,
    Tls,
    Timeout,
    RequestBuild,
    RequestBody,
    ResponseDecode,
    Cancelled,
    Script,
    Auth,
    Protocol,
}
```

Each error should include:

- user-facing summary.
- technical details.
- diagnostic code.
- source chain.
- safe metadata.
- suggested next action where possible.

Do not expose raw secret-containing strings.

---

# 61. Cancellation

Every long-running operation should be cancellable:

- request.
- upload.
- download.
- collection run.
- performance test.
- import.
- spec generation.
- search.
- sync.
- mock server.
- monitor.
- capture session.

Cancellation should propagate through the engine rather than merely hide the UI.

---

# 62. Logging and Observability

Use structured `tracing`.

Log targets:

- application.
- networking.
- scripting.
- storage.
- sync.
- plugins.
- mocks.
- monitors.
- capture.

Provide:

- rotating local log file.
- in-app diagnostics.
- "copy diagnostic report" with secrets redacted.
- log-level setting.

Telemetry must be opt-in or clearly configurable for an open-source privacy-focused product.

---

# 63. Security Threat Model

Threats include:

- malicious imported collections.
- malicious scripts.
- malicious plugins.
- accidental secret commits.
- MITM proxy private key compromise.
- OAuth tokens in logs.
- unsafe TLS disabling.
- arbitrary file reads.
- arbitrary command execution.
- malicious mock requests.
- untrusted protobuf/spec references.
- zip/path traversal in imports.
- dependency supply-chain attacks.

Required defenses:

- script sandbox.
- plugin sandbox.
- strict file path normalization.
- archive extraction checks.
- size limits.
- network timeouts.
- secret redaction.
- secure temp files.
- TLS verification on.
- signed releases.
- dependency audit.
- SBOM.
- reproducible-build effort.
- explicit update verification.

---

# 64. License Strategy

Recommended possibilities:

## Option A — Apache-2.0

Good for wide adoption and commercial reuse.

## Option B — MPL-2.0

File-level copyleft while allowing commercial integration.

## Option C — GPL-3.0-or-later

Strong copyleft and more compatible if intentionally reusing GPL portions of Zed.

If the app is intended to use only GPUI and independently developed components, Apache-2.0 or MPL-2.0 are strong options.

Do not copy GPL Zed editor/application code into an Apache/MIT application without understanding the licensing impact.

---

# 65. Repository Governance

Include from the beginning:

- `LICENSE`.
- `CONTRIBUTING.md`.
- `CODE_OF_CONDUCT.md`.
- `SECURITY.md`.
- issue templates.
- pull-request template.
- architecture decision records.
- contributor guide.
- release process.
- dependency policy.

Labels:

- `good first issue`
- `help wanted`
- `protocol:http`
- `protocol:grpc`
- `ui`
- `performance`
- `security`
- `plugin`
- `import`
- `compatibility`
- `breaking-change`

---

# 66. Testing Strategy

## 66.1 Unit Tests

For:

- variable resolution.
- auth signing.
- importers.
- exporters.
- URL encoding.
- serializers.
- cookie matching.
- request preparation.
- response parsing.
- mocks.
- monitor scheduling.
- file migrations.

## 66.2 Integration Tests

Use local fixture servers for:

- redirects.
- TLS.
- chunked response.
- gzip/brotli.
- slow responses.
- connection failures.
- OAuth callback.
- WebSockets.
- GraphQL.
- gRPC.
- MQTT.
- SSE.

## 66.3 Golden Tests

Golden fixtures for:

- Postman import/export.
- OpenAPI conversion.
- native workspace serialization.
- code snippets.
- semantic diffs.

## 66.4 UI Tests

Test:

- request creation.
- send.
- cancellation.
- tabs.
- keyboard commands.
- environment switching.
- import.
- runner.
- response search.
- crash recovery.

Use GPUI test support where feasible.

## 66.5 Compatibility Corpus

Maintain a repository of real-world public:

- OpenAPI specs.
- Postman collections.
- protobuf projects.
- GraphQL schemas.

Run import/validation tests continuously.

---

# 67. Performance Benchmarks

Track:

- startup.
- open 10k request workspace.
- global search.
- send overhead.
- response throughput.
- render 10 MB JSON.
- render 100 MB raw body.
- import large collection.
- collection runner throughput.
- memory at idle.
- memory after 1000 requests.
- script runtime startup.
- mock server RPS.

Regression thresholds should fail benchmark CI only after stable baselines exist.

---

# 68. CI/CD

CI matrix:

- macOS.
- Linux.
- Windows.

Checks:

- `cargo fmt`.
- `cargo clippy`.
- unit tests.
- integration tests.
- license audit.
- dependency audit.
- secret scan.
- SBOM generation.
- selected benchmarks.
- installer smoke tests.

Release channels:

- stable.
- beta.
- nightly.

---

# 69. Packaging

## macOS

- universal or separate Intel/Apple Silicon builds.
- `.dmg`.
- code signing.
- notarization.
- auto-update signature.

## Windows

- x64.
- ARM64 when ready.
- MSI/MSIX or installer executable.
- code signing.

## Linux

- AppImage.
- `.deb`.
- `.rpm`.
- tarball.
- Flatpak later.

---

# 70. Auto Updates

Use a signed manifest.

Requirements:

- stable/beta/nightly channel.
- signature verification.
- release notes.
- download progress.
- restart to update.
- never execute unsigned update payloads.

Allow package-manager installs to disable built-in updating.

---

# 71. CLI Design

CLI principles:

- non-interactive by default in CI.
- stable exit codes.
- JSON output.
- color only when terminal supports it.
- secret-safe output.
- same engine as desktop.
- local file and synced workspace identifiers.

Example:

```bash
packetsmith run collection:users \
  --env staging \
  --report junit:./reports/results.xml \
  --report json:./reports/results.json
```

Exit code policy:

- `0` success.
- `1` test failure.
- `2` invalid config.
- `3` execution/network failure.
- `4` auth/vault failure.
- other reserved codes documented.

---

# 72. Public Internal API

Define stable internal contracts between components using Rust traits and versioned serialized messages.

For process boundaries:

- script worker.
- daemon.
- plugin host.

Use a versioned IPC protocol.

Possible transports:

- local Unix socket / named pipe.
- framed JSON or MessagePack initially.
- protobuf if compatibility demands it.

---

# 73. Crash Recovery

Persist enough ephemeral state to restore:

- open workspaces.
- tabs.
- split layout.
- unsaved request drafts.
- active environment.
- sidebar state.

Do not restore:

- plaintext secret reveal state.
- OAuth browser state after expiration.
- unsafe running operations without explicit recovery logic.

---

# 74. Data Migrations

Every native file/database schema has an explicit version.

Migration principles:

- backup before destructive migration.
- idempotent migrations.
- test fixtures from every supported historical version.
- forward incompatibility must produce a clear error.
- never silently discard unknown fields.

---

# 75. AI Assistance Layer

AI must not be coupled to core functionality.

Potential actions:

- explain response.
- generate tests.
- generate documentation.
- generate request from natural language.
- derive collection from API description.
- detect likely auth failure.
- compare responses.
- propose mocks.
- generate example bodies.
- inspect OpenAPI errors.

Providers should be user-configurable.

Privacy UI should show exactly what context will leave the machine before sending it to an AI provider.

---

# 76. Project Roadmap

The roadmap below intentionally builds vertical slices.

## Phase 0 — Technical Foundation

Deliver:

- Cargo workspace.
- GPUI boot.
- design tokens.
- command system.
- settings.
- tracing.
- crash reporting infrastructure.
- file format prototype.
- SQLite cache.
- test server infrastructure.
- CI.

Exit condition:

> A cross-platform window opens, commands work, settings persist, workspace loads, and automated tests run on all target OSes.

## Phase 1 — Excellent HTTP Client

Deliver:

- request tabs.
- URL/method.
- params.
- headers.
- bodies.
- HTTP execution.
- response viewer.
- cancellation.
- timing.
- request history.
- save request.
- basic collections.

Exit condition:

> A developer can comfortably replace a basic REST client with PacketSmith.

## Phase 2 — Environments, Auth, Cookies, Vault

Deliver:

- variables.
- environments.
- interpolation.
- auth providers.
- OAuth.
- cookie jar.
- certificates.
- proxy.
- local vault.

Exit condition:

> Real production APIs with non-trivial authentication can be used safely.

## Phase 3 — Collections and Import/Export

Deliver:

- tree operations.
- nested folders.
- examples.
- Postman import.
- OpenAPI import.
- cURL.
- Insomnia/Bruno/Hoppscotch adapters.
- export.

Exit condition:

> Existing users can migrate realistic projects without rebuilding them manually.

## Phase 4 — Scripting and Testing

Deliver:

- isolated JS worker.
- pre/post scripts.
- test API.
- script console.
- variable mutation.
- package abstraction.

Exit condition:

> Imported scripted collections can execute and test responses.

## Phase 5 — Collection Runner + CLI

Deliver:

- runner UI.
- data-driven runs.
- reports.
- CLI.
- CI examples.
- JUnit output.

Exit condition:

> PacketSmith becomes usable for automated API testing.

## Phase 6 — GraphQL + WebSocket + SSE

Deliver dedicated clients.

Exit condition:

> Major web API styles beyond REST are first-class.

## Phase 7 — gRPC + Socket.IO + MQTT

Deliver dedicated protocol clients.

Exit condition:

> PacketSmith is a credible multi-protocol API workbench.

## Phase 8 — Specs and Documentation

Deliver:

- OpenAPI editor.
- validation.
- collection generation.
- docs generator.
- static export.

Exit condition:

> PacketSmith covers design-to-documentation workflows.

## Phase 9 — Mocks and Monitoring

Deliver:

- local mock server.
- daemon.
- monitor scheduler.
- run history.
- webhook alerts.

Exit condition:

> PacketSmith can support APIs outside active manual development.

## Phase 10 — Performance Testing

Deliver:

- load engine.
- virtual users.
- stages.
- metrics.
- charts.
- reports.

Exit condition:

> Functional test collections can be reused for local performance tests.

## Phase 11 — Traffic Capture

Deliver:

- local proxy.
- HTTPS capture.
- CA management.
- capture filters.
- save-to-collection.

Exit condition:

> PacketSmith can inspect and reconstruct real application API traffic.

## Phase 12 — Git and Collaboration

Deliver:

- semantic Git diff.
- Git panel.
- self-hosted sync backend.
- users/teams.
- comments.
- presence.
- permissions.

Exit condition:

> Teams can collaborate without sacrificing local-first workflows.

## Phase 13 — Plugins

Deliver:

- WASM SDK.
- manifest.
- permissions.
- plugin manager.
- marketplace/index format.
- importer/codegen/auth extension points.

Exit condition:

> Major new integrations can ship outside the core repository.

## Phase 14 — Flows

Deliver visual workflow builder.

## Phase 15 — AI and Advanced Platform Features

Deliver:

- AI assistance.
- MCP testing tools.
- cloud runners if desired.
- enterprise features if a hosted business is created.

---

# 77. MVP Definition

The MVP should **not** mean "all features."

A credible MVP is:

- macOS/Windows/Linux desktop.
- HTTP client.
- JSON/XML/text bodies.
- params/headers.
- response viewer.
- collections/folders.
- environments.
- variables.
- API key/basic/bearer/OAuth 2.
- request history.
- local secure vault.
- cURL import.
- Postman Collection v2.1 import.
- OpenAPI import.
- native file format.
- Git-friendly workspace.
- dark/light theme.
- command palette.
- keyboard shortcuts.

This is already a meaningful open-source release.

---

# 78. v1.0 Definition

A stronger v1.0 should include:

- all MVP capabilities.
- scripting/tests.
- collection runner.
- CLI.
- GraphQL.
- WebSocket.
- gRPC.
- code generation.
- mocks.
- specs.
- documentation.
- import/export compatibility.
- plugin foundation.
- robust crash recovery.
- signed installers.
- migration policy.
- security documentation.

MQTT, traffic capture, real-time collaboration, flows, and AI can land after v1.0 if necessary without compromising the architecture.

---

# 79. Quality Gates Before v1.0

Do not call the product 1.0 until:

- no known critical secret leakage.
- import/export round-trip tests pass.
- crash recovery is tested.
- SQLite corruption recovery path exists.
- large responses do not freeze the UI.
- TLS verification works correctly.
- OAuth flows are stable.
- collection runner is deterministic.
- CLI is scriptable.
- Windows/macOS/Linux releases are reproducible enough for maintainers.
- security policy is published.
- file format is versioned.
- plugin API is either explicitly unstable or versioned.

---

# 80. Recommended Initial Team Split

For a small team:

## Engineer A — Desktop/UI

- GPUI shell.
- design system.
- tabs.
- tree.
- request builder.
- response viewer.
- settings.

## Engineer B — Core/Networking

- request engine.
- HTTP.
- auth.
- variables.
- cookies.
- TLS.
- proxy.

## Engineer C — Persistence/Tooling

- workspace files.
- SQLite.
- history.
- import/export.
- search.
- CLI.

## Engineer D — Runtime/Testing

- scripting.
- assertions.
- runner.
- reports.
- test fixtures.

As the team expands, protocol modules become independent workstreams.

---

# 81. First 30 Engineering Tasks

1. Create Cargo workspace.
2. Pin Rust toolchain.
3. Pin GPUI version/revision.
4. Establish license policy.
5. Add CI on macOS/Linux/Windows.
6. Create GPUI application shell.
7. Define command registry.
8. Define theme tokens.
9. Build basic buttons/inputs/tabs/tree primitives.
10. Define `RequestDocument`.
11. Define `HttpRequest`.
12. Implement native workspace manifest.
13. Implement deterministic YAML serialization.
14. Add SQLite cache.
15. Add tracing.
16. Create local HTTP fixture server.
17. Implement URL parser.
18. Implement HTTP execution service.
19. Stream execution events.
20. Build request method/URL bar.
21. Build headers editor.
22. Build params editor.
23. Build body editor.
24. Build response metadata header.
25. Build streaming raw response viewer.
26. Add JSON formatter.
27. Add cancellation.
28. Add request history.
29. Add save/open request.
30. Ship internal alpha.

---

# 82. Architectural Decisions to Record as ADRs

Create Architecture Decision Records for:

- GPUI pinning.
- application license.
- workspace file format.
- SQLite role.
- async runtime.
- HTTP stack.
- TLS stack.
- script engine.
- plugin sandbox.
- process boundaries.
- sync model.
- secret storage.
- code editor implementation.
- updater.
- crash reporting.
- telemetry.
- Git implementation.
- object/blob storage.

---

# 83. Naming Suggestions

Before final branding, perform proper domain, GitHub organization, package-name, trademark, and app-store checks.

## Strong Options

### 1. PacketSmith
Meaning: a tool for crafting, inspecting, and manipulating network/API traffic.

Pros:
- memorable.
- technical.
- works beyond REST.
- fits traffic capture and protocol tooling.
- "Smith" communicates building/crafting.

### 2. FluxReq
Meaning: requests and data flowing through a system.

Pros:
- short.
- modern.
- protocol-neutral.
- works well as a CLI name.

### 3. RequestForge
Meaning: forge requests, tests, mocks, and API workflows.

Pros:
- clear product meaning.
- developer-oriented.
- strong open-source feel.

### 4. EmberAPI
Meaning: lightweight, fast, Rust-adjacent branding without saying "Rust."

Pros:
- memorable.
- visually brandable.
- broad enough for a platform.

### 5. Copperline
Meaning: communications moving over a line; also subtly evokes systems tooling.

Pros:
- distinctive.
- does not lock the product to REST.
- strong desktop-tool identity.

## Additional Options

- WireSmith
- CallForge
- ReqForge
- ApiFoundry
- Wirebench
- RequestBench
- EndpointForge
- FluxAPI
- SocketSmith
- ProbeDeck
- Payload
- PayloadLab
- ProtocolLab
- RelayBench
- EndpointLab
- RequestDock
- WireDesk
- ApiWorkbench
- RequestKit
- RequestLab

## Names to Avoid

Avoid names that:

- look too close to "Postman."
- imply the project is an official Postman fork.
- contain "Rust" unless you want the brand tied permanently to implementation language.
- mention only REST when the product supports many protocols.
- conflict with prominent existing API clients or Rust crates.

---

# 84. Suggested Branding Direction

If **PacketSmith** is selected:

Tagline options:

- **Craft, test, and understand every API.**
- **The open API workbench.**
- **A native, open-source API platform for developers.**
- **Forge requests. Test systems. Own your workflow.**

Visual identity:

- geometric anvil/packet symbol.
- packet nodes or connection lines.
- avoid copying Postman's orange brand language.
- emphasize precision, speed, and native tooling.

---

# 85. Recommended Strategic Decisions

The project should make the following decisions early:

1. **Choose local-first as a permanent product principle.**
2. **Use human-readable workspace files as the shareable source of truth.**
3. **Use SQLite only for derived/local operational data.**
4. **Keep protocol engines independent from GPUI.**
5. **Keep scripts outside the UI process.**
6. **Use an explicit plugin sandbox.**
7. **Build the CLI on the same core libraries.**
8. **Implement Postman migration early, not after launch.**
9. **Do not block local use behind an account.**
10. **Treat secrets and redaction as architecture, not polish.**
11. **Treat large-response streaming as a core requirement.**
12. **Do not reuse GPL Zed code accidentally if choosing a permissive project license.**
13. **Prioritize a great HTTP experience before implementing every protocol.**
14. **Create compatibility fixtures continuously.**
15. **Do not let cloud collaboration dictate the local domain model.**

---

# 86. Definition of Done for the Complete Vision

PacketSmith reaches the full platform vision when a user can:

- design an API from a spec,
- generate or create requests,
- send those requests across major API protocols,
- authenticate securely,
- organize them into collections,
- parameterize them with environments,
- script them,
- test them,
- run them from desktop or CLI,
- load test them,
- mock them,
- monitor them,
- document them,
- capture real traffic,
- import/export industry formats,
- version them in Git,
- collaborate when desired,
- extend behavior through plugins,
- automate workflows visually,
- and keep full ownership of their local data.

That is the target.

---

# 87. Research Baseline / References

This plan was scoped against current public feature documentation from Postman and current GPUI/Zed information.

Useful references:

- Postman getting started / platform overview  
  https://learning.postman.com/docs/getting-started/overview/

- Postman elements: collections, environments, Flows, Spec Hub, mocks, monitors  
  https://learning.postman.com/docs/getting-started/basics/postman-elements/

- Postman request protocols  
  https://learning.postman.com/docs/use/send-requests/create-requests/request-basics

- Postman testing and scripts  
  https://learning.postman.com/docs/tests-and-scripts/tests-and-scripts/

- Postman performance testing  
  https://learning.postman.com/docs/tests-and-scripts/test-apis/performance-testing

- Postman Vault  
  https://learning.postman.com/docs/use/postman-vault/postman-vault-secrets

- Postman traffic capture  
  https://learning.postman.com/docs/use/capturing-request-data/capture-overview

- Postman import/export  
  https://learning.postman.com/docs/getting-started/importing-and-exporting/importing-and-exporting-overview/

- GPUI package metadata  
  https://github.com/zed-industries/zed/blob/main/crates/gpui/Cargo.toml

- Zed software licensing overview  
  https://zed.dev/software-overview

---

# 88. Final Recommendation

Build PacketSmith as a **native local-first API workbench**, not simply an open-source visual HTTP client.

The winning architecture is:

```text
GPUI Desktop
    ↓
Reusable Application/Core Services
    ↓
Protocol Engines + Script/Test Runtime
    ↓
Open Workspace Files + Secure Vault + SQLite Operational Data
    ↓
CLI / Daemon / Optional Self-Hosted Collaboration Server
```

The critical sequence is:

```text
Foundation
→ HTTP excellence
→ Variables/Auth/Vault
→ Collections + Migration
→ Scripts/Tests
→ Runner + CLI
→ Multi-protocol
→ Specs/Docs
→ Mocks/Monitors
→ Performance
→ Capture
→ Collaboration
→ Plugins
→ Flows/AI
```

Do not reverse that order. A fast, stable HTTP workflow with excellent migration support will earn users; the platform capabilities can then compound on top of that foundation.
