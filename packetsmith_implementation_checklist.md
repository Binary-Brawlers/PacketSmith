# PacketSmith — Master Implementation Checklist
## Open-Source GPUI API Platform

> **Working codename:** PacketSmith  
> **Language:** Rust  
> **UI:** Zed GPUI  
> **Target:** macOS + Windows + Linux  
> **Purpose:** End-to-end implementation checklist accompanying `packetsmith_project_plan.md`

---

# How to Use This Checklist

Rules:

- Do not check an item merely because code exists.
- Check an item only when it is implemented, tested, and integrated.
- Every phase has an **Exit Gate**.
- Do not begin multiple advanced phases while foundational exit gates are failing.
- Add issue/PR references beside completed items.
- Security-sensitive items require tests.
- Cross-platform items are complete only after verification on all supported OSes unless explicitly marked platform-specific.

Legend:

- `[ ]` not started / incomplete.
- `[x]` completed.
- `BLOCKED:` add explanation beside the item.
- `DEFERRED:` use only when deliberately moved to a later milestone.

---

# 0. Project and Product Decisions

## 0.1 Identity

- [x] Choose final project name.
- [ ] Check GitHub organization/repository availability.
- [ ] Check crates.io names.
- [ ] Check CLI binary-name conflicts.
- [ ] Check package-manager naming conflicts.
- [ ] Perform trademark search.
- [ ] Check major domains.
- [ ] Choose logo direction.
- [x] Choose tagline.
- [ ] Reserve social/community handles if needed.

## 0.2 Licensing

- [x] Choose application license.
- [x] Document GPUI Apache-2.0 usage.
- [x] Define policy for GPL Zed source reuse.
- [x] Add `LICENSE`.
- [ ] Add third-party notices strategy.
- [x] Add automated dependency-license audit.
- [x] Define acceptable dependency licenses.
- [x] Document contribution licensing.

## 0.3 Product Principles

- [x] Commit to local-first.
- [x] Commit to no-account-required core.
- [x] Commit to open workspace format.
- [x] Commit to secret-safe exports.
- [x] Commit to desktop/CLI core parity.
- [x] Commit to protocol-independent core.
- [x] Commit to optional telemetry.
- [x] Commit to explicit cloud opt-in.

---

# 1. Repository Foundation

## 1.1 Repository

- [x] Create repository.
- [x] Add `.gitignore`.
- [x] Add `.editorconfig`.
- [x] Add `README.md`.
- [x] Add `CONTRIBUTING.md`.
- [x] Add `CODE_OF_CONDUCT.md`.
- [x] Add `SECURITY.md`.
- [x] Add pull request template.
- [x] Add bug issue template.
- [x] Add feature issue template.
- [x] Add architecture decision record template.
- [x] Add changelog strategy.

## 1.2 Rust Toolchain

- [x] Add `rust-toolchain.toml`.
- [x] Pin supported Rust version.
- [x] Configure rustfmt.
- [x] Configure clippy.
- [x] Deny warnings in CI where appropriate.
- [x] Add workspace lint configuration.
- [x] Add cargo aliases for common tasks.

## 1.3 Cargo Workspace

- [x] Create root Cargo workspace.
- [x] Create desktop app crate.
- [x] Create domain crate.
- [x] Create workspace/storage crate.
- [x] Create request engine crate.
- [x] Create HTTP crate.
- [x] Create UI components crate.
- [x] Create settings crate.
- [x] Create test support crate.
- [x] Define dependency direction rules.
- [x] Add architecture lint/documentation preventing circular ownership.

## 1.4 GPUI

- [x] Select GPUI release or commit.
- [x] Pin GPUI exactly.
- [x] Boot minimal GPUI application.
- [ ] Verify macOS build.
- [ ] Verify Windows build.
- [ ] Verify Linux X11 build.
- [ ] Verify Linux Wayland build.
- [x] Document GPUI upgrade process.
- [ ] Add an isolated GPUI upgrade test branch/workflow.

---

# 2. CI and Developer Experience

## 2.1 CI

- [x] CI on macOS.
- [x] CI on Ubuntu.
- [x] CI on Windows.
- [x] `cargo fmt --check`.
- [x] `cargo clippy`.
- [x] unit tests.
- [ ] integration tests.
- [x] dependency vulnerability audit.
- [x] dependency license audit.
- [ ] secret scanning.
- [ ] build release binaries.
- [ ] artifact retention for CI builds.

## 2.2 Local Commands

- [x] `cargo xtask` or equivalent automation crate.
- [ ] developer bootstrap command.
- [ ] run app command.
- [ ] run fixture servers command.
- [x] run all tests command.
- [x] run formatting command.
- [x] run lint command.
- [ ] build installers command.
- [ ] generate SBOM command.

## 2.3 Documentation

- [x] architecture overview.
- [x] crate map.
- [x] build prerequisites.
- [x] macOS setup.
- [x] Linux setup.
- [x] Windows setup.
- [ ] debugging guide.
- [ ] adding a protocol guide.
- [ ] adding an importer guide.

---

# 3. Application Shell

## 3.1 Window

- [ ] Main native window.
- [ ] custom/native titlebar decision.
- [ ] minimum window size.
- [ ] window size persistence.
- [ ] window position persistence.
- [ ] maximize state persistence.
- [ ] multi-monitor sanity checks.
- [ ] DPI scaling.
- [ ] system theme detection.

## 3.2 Application State

- [ ] `AppState`.
- [ ] workspace manager.
- [ ] window manager.
- [ ] command registry.
- [ ] settings store.
- [ ] theme registry.
- [ ] keymap registry.
- [ ] notification/toast manager.
- [ ] shutdown coordinator.

## 3.3 Command System

- [ ] command trait/type.
- [ ] command registry.
- [ ] command IDs.
- [ ] labels.
- [ ] keybinding metadata.
- [ ] enabled/disabled state.
- [ ] command dispatch.
- [ ] context-aware command dispatch.
- [ ] menu integration.
- [ ] command palette integration.
- [ ] command tests.

---

# 4. Design System

## 4.1 Tokens

- [ ] spacing scale.
- [ ] radius scale.
- [ ] typography scale.
- [ ] border tokens.
- [ ] elevation tokens.
- [ ] focus tokens.
- [ ] semantic colors.
- [ ] HTTP method colors.
- [ ] dark theme.
- [ ] light theme.
- [ ] system theme.

## 4.2 Core Components

- [ ] button.
- [ ] icon button.
- [ ] checkbox.
- [ ] radio.
- [ ] toggle.
- [ ] text input.
- [ ] password input.
- [ ] search input.
- [ ] textarea.
- [ ] number input.
- [ ] dropdown.
- [ ] combobox.
- [ ] multi-select.
- [ ] tabs.
- [ ] tab strip.
- [ ] split pane.
- [ ] resize handle.
- [ ] tree.
- [ ] virtual list.
- [ ] table.
- [ ] context menu.
- [ ] dropdown menu.
- [ ] popover.
- [ ] tooltip.
- [ ] modal.
- [ ] confirmation dialog.
- [ ] alert dialog.
- [ ] toast.
- [ ] badge.
- [ ] spinner.
- [ ] progress bar.
- [ ] empty state.
- [ ] error state.
- [ ] skeleton/loading state.

## 4.3 Accessibility

- [ ] focus traversal.
- [ ] visible focus.
- [ ] semantic roles.
- [ ] screen reader labels.
- [ ] keyboard activation.
- [ ] contrast audit.
- [ ] text scaling.
- [ ] reduced motion.
- [ ] no color-only status indicators.

---

# 5. Workspace File Format

## 5.1 Native Manifest

- [ ] Define `packetsmith.yaml`.
- [ ] schema version.
- [ ] workspace stable ID.
- [ ] workspace name.
- [ ] resource roots.
- [ ] settings reference.
- [ ] format documentation.

## 5.2 Resource IDs

- [ ] stable UUID strategy.
- [ ] resource ID serialization.
- [ ] collision handling.
- [ ] copy behavior.
- [ ] import behavior.
- [ ] move/rename preserves ID.

## 5.3 Resource Serialization

- [ ] deterministic serialization.
- [ ] request schema.
- [ ] collection schema.
- [ ] folder schema.
- [ ] environment schema.
- [ ] example schema.
- [ ] mock schema.
- [ ] monitor schema.
- [ ] flow schema placeholder.
- [ ] unknown field preservation policy.
- [ ] schema validation.
- [ ] parser error diagnostics.

## 5.4 Migrations

- [ ] migration framework.
- [ ] migration version registry.
- [ ] backup before migration.
- [ ] idempotency tests.
- [ ] failure recovery.
- [ ] fixture for every historical schema version.

---

# 6. Local Storage and SQLite

## 6.1 Database

- [ ] create `.packetsmith/cache.db`.
- [ ] migrations.
- [ ] WAL mode decision.
- [ ] corruption handling.
- [ ] busy timeout.
- [ ] graceful shutdown.
- [ ] backup strategy.

## 6.2 Tables

- [ ] history.
- [ ] history metadata.
- [ ] search index.
- [ ] open tabs.
- [ ] recent resources.
- [ ] run metadata.
- [ ] test results.
- [ ] blobs.
- [ ] sync journal placeholder.
- [ ] monitor run history.
- [ ] mock access log.
- [ ] capture sessions.

## 6.3 File Watcher

- [ ] watch workspace files.
- [ ] debounce changes.
- [ ] ignore app-owned cache.
- [ ] detect external edits.
- [ ] reload clean resource.
- [ ] conflict UI for dirty local resource.
- [ ] rename detection where possible.

---

# 7. Settings

## 7.1 Settings Engine

- [ ] defaults.
- [ ] user settings.
- [ ] workspace settings.
- [ ] merge precedence.
- [ ] schema.
- [ ] validation.
- [ ] migration.
- [ ] reset setting.
- [ ] reset all.

## 7.2 Settings UI

- [ ] search.
- [ ] appearance.
- [ ] editor.
- [ ] network.
- [ ] proxy.
- [ ] TLS.
- [ ] history.
- [ ] scripts.
- [ ] runner.
- [ ] Git.
- [ ] privacy.
- [ ] updates.
- [ ] plugins placeholder.

---

# 8. Text and Code Editing Foundation

## 8.1 Plain Editor

- [ ] Unicode input.
- [ ] cursor.
- [ ] selection.
- [ ] copy.
- [ ] cut.
- [ ] paste.
- [ ] undo.
- [ ] redo.
- [ ] scroll.
- [ ] mouse selection.
- [ ] keyboard selection.
- [ ] Home/End.
- [ ] word navigation.
- [ ] line navigation.

## 8.2 Code Editor

- [ ] line numbers.
- [ ] syntax highlighting.
- [ ] bracket matching.
- [ ] auto indentation.
- [ ] search.
- [ ] replace.
- [ ] go to line.
- [ ] formatter integration.
- [ ] diagnostics.
- [ ] completion architecture.
- [ ] large-file mode.
- [ ] theme token integration.

## 8.3 Languages

- [ ] JSON.
- [ ] XML.
- [ ] YAML.
- [ ] JavaScript.
- [ ] GraphQL.
- [ ] Markdown.
- [ ] protobuf.
- [ ] HTML.
- [ ] plain text.

---

# 9. Request Domain Model

- [ ] `RequestDocument`.
- [ ] `ProtocolRequest`.
- [ ] `HttpRequest`.
- [ ] `GraphQlRequest`.
- [ ] `GrpcRequest`.
- [ ] `WebSocketRequest`.
- [ ] `SocketIoRequest`.
- [ ] `MqttRequest`.
- [ ] `McpRequest`.
- [ ] `AiRequest`.
- [ ] `SoapRequest`.
- [ ] request settings.
- [ ] request scripts.
- [ ] auth config.
- [ ] examples references.
- [ ] protocol migration strategy.
- [ ] serialization tests.

---

# 10. Execution Engine Foundation

## 10.1 Execution API

- [ ] protocol executor trait.
- [ ] execution context.
- [ ] cancellation token.
- [ ] event sink.
- [ ] structured execution events.
- [ ] execution summary.
- [ ] structured errors.
- [ ] redaction metadata.

## 10.2 Lifecycle

- [ ] preparing.
- [ ] resolving variables.
- [ ] pre-request script hook.
- [ ] auth application.
- [ ] connect.
- [ ] upload progress.
- [ ] response headers.
- [ ] download progress.
- [ ] post-response script hook.
- [ ] tests.
- [ ] complete.
- [ ] fail.
- [ ] cancel.

## 10.3 Concurrency

- [ ] never block GPUI thread.
- [ ] background runtime.
- [ ] bounded channels.
- [ ] request cancellation.
- [ ] shutdown cancellation.
- [ ] orphan task detection.

---

# 11. HTTP Client — Core

## 11.1 Request Bar

- [ ] method dropdown.
- [ ] custom method.
- [ ] URL input.
- [ ] variable highlighting.
- [ ] Send button.
- [ ] Cancel button.
- [ ] Save button.
- [ ] request dirty marker.
- [ ] environment selector visible in shell.

## 11.2 URL Parsing

- [ ] HTTP URL parser.
- [ ] HTTPS.
- [ ] query parsing.
- [ ] repeated keys.
- [ ] path detection.
- [ ] percent encoding.
- [ ] resolved preview.
- [ ] invalid URL diagnostics.

## 11.3 Params

- [ ] key/value table.
- [ ] enable toggle.
- [ ] duplicate key.
- [ ] bulk edit.
- [ ] descriptions.
- [ ] sync table with raw URL.
- [ ] variable support.

## 11.4 Headers

- [ ] key/value table.
- [ ] enable toggle.
- [ ] duplicate header names.
- [ ] bulk edit.
- [ ] generated headers.
- [ ] sensitive masking.
- [ ] variable support.
- [ ] presets.

## 11.5 Body

- [ ] none.
- [ ] raw.
- [ ] JSON.
- [ ] XML.
- [ ] text.
- [ ] HTML.
- [ ] form URL encoded.
- [ ] multipart form-data.
- [ ] binary file.
- [ ] streamed file.
- [ ] content type selector.
- [ ] automatic Content-Type where appropriate.
- [ ] body size indicator.

## 11.6 Execution

- [ ] GET.
- [ ] POST.
- [ ] PUT.
- [ ] PATCH.
- [ ] DELETE.
- [ ] HEAD.
- [ ] OPTIONS.
- [ ] custom methods.
- [ ] request timeout.
- [ ] connect timeout.
- [ ] cancellation.
- [ ] upload progress.
- [ ] download progress.

---

# 12. HTTP Networking — Advanced

## 12.1 Redirects

- [ ] follow toggle.
- [ ] max redirects.
- [ ] redirect chain.
- [ ] auth stripping policy.
- [ ] redirect diagnostics.

## 12.2 Compression

- [ ] gzip.
- [ ] deflate.
- [ ] brotli.
- [ ] zstd if supported.
- [ ] content decoding diagnostics.

## 12.3 HTTP Versions

- [ ] HTTP/1.1.
- [ ] HTTP/2.
- [ ] negotiated version display.
- [ ] HTTP/3 research spike.
- [ ] HTTP/3 implementation if selected.

## 12.4 Proxy

- [ ] system proxy.
- [ ] HTTP proxy.
- [ ] HTTPS proxy.
- [ ] SOCKS5.
- [ ] proxy auth.
- [ ] no-proxy rules.
- [ ] per-request override.

## 12.5 TLS

- [ ] default certificate verification.
- [ ] disable verification setting with warning.
- [ ] custom CA.
- [ ] client certificate.
- [ ] client private key.
- [ ] PFX/PKCS#12.
- [ ] certificate passphrase.
- [ ] TLS diagnostics.
- [ ] certificate chain viewer.
- [ ] certificate expiry display.

---

# 13. Response Viewer

## 13.1 Header

- [ ] status code.
- [ ] status text.
- [ ] duration.
- [ ] size.
- [ ] content type.
- [ ] HTTP version.
- [ ] remote IP when available.

## 13.2 Tabs

- [ ] Body.
- [ ] Headers.
- [ ] Cookies.
- [ ] Tests.
- [ ] Timing.
- [ ] Console link.
- [ ] Visualization placeholder.

## 13.3 Body Modes

- [ ] Pretty.
- [ ] Raw.
- [ ] Preview.
- [ ] Hex.
- [ ] Diff placeholder.
- [ ] Visualization placeholder.

## 13.4 Formatters

- [ ] JSON.
- [ ] XML.
- [ ] HTML.
- [ ] YAML.
- [ ] JavaScript.
- [ ] plain text fallback.

## 13.5 Large Responses

- [ ] stream chunks.
- [ ] body threshold config.
- [ ] spool to disk.
- [ ] virtualized rendering.
- [ ] disable expensive formatting over threshold.
- [ ] save to file.
- [ ] partial preview.
- [ ] cancellation.
- [ ] memory benchmark at 100 MB.
- [ ] memory benchmark at 1 GB streamed body.

## 13.6 Search

- [ ] text.
- [ ] case-sensitive.
- [ ] regex.
- [ ] JSON tree search.
- [ ] JSONPath.
- [ ] XPath.

---

# 14. History

- [ ] persist execution metadata.
- [ ] redact secrets.
- [ ] group by date.
- [ ] search.
- [ ] filter method.
- [ ] filter status.
- [ ] filter workspace.
- [ ] replay.
- [ ] open in new tab.
- [ ] save as request.
- [ ] compare with current.
- [ ] retention settings.
- [ ] clear history.
- [ ] incognito mode.

---

# 15. Collections and Resource Tree

## 15.1 Resource Tree

- [ ] collection nodes.
- [ ] folder nodes.
- [ ] request nodes.
- [ ] expand/collapse.
- [ ] virtualize large trees.
- [ ] drag/drop.
- [ ] keyboard navigation.
- [ ] context menu.
- [ ] multi-select later.

## 15.2 Operations

- [ ] create collection.
- [ ] create folder.
- [ ] create request.
- [ ] rename.
- [ ] move.
- [ ] reorder.
- [ ] duplicate.
- [ ] delete.
- [ ] trash.
- [ ] restore.
- [ ] permanent delete.
- [ ] copy resource path.
- [ ] reveal file.

## 15.3 Collection Metadata

- [ ] description.
- [ ] variables.
- [ ] auth.
- [ ] pre-request script.
- [ ] post-response script.
- [ ] runner settings.

## 15.4 Examples

- [ ] save current response as example.
- [ ] name example.
- [ ] edit response status.
- [ ] edit response headers.
- [ ] edit body.
- [ ] link request snapshot.
- [ ] delete example.
- [ ] duplicate example.
- [ ] use examples in docs.
- [ ] use examples in mocks.

---

# 16. Tabs, Splits, and Workbench

- [ ] open request tab.
- [ ] close tab.
- [ ] dirty close confirmation.
- [ ] autosave option.
- [ ] pin tab.
- [ ] duplicate tab.
- [ ] reorder tabs.
- [ ] reopen closed tab.
- [ ] tab overflow menu.
- [ ] vertical split.
- [ ] horizontal split.
- [ ] move tab between splits.
- [ ] persist layout.
- [ ] crash restore tabs.
- [ ] Quick Open.
- [ ] breadcrumbs.

---

# 17. Variables

## 17.1 Parser

- [ ] detect `{{variable}}`.
- [ ] parse names safely.
- [ ] escaped templates.
- [ ] unresolved references.
- [ ] cyclic reference detection.
- [ ] nested resolution policy.

## 17.2 Scopes

- [ ] workspace/global.
- [ ] collection.
- [ ] environment.
- [ ] request-local.
- [ ] iteration data.
- [ ] temporary/local runtime.
- [ ] vault reference.
- [ ] built-in dynamic.

## 17.3 UI

- [ ] variable highlight.
- [ ] hover resolved value.
- [ ] hide secret.
- [ ] show provenance.
- [ ] jump to variable.
- [ ] create variable from unresolved reference.
- [ ] find usages.

## 17.4 Dynamic Variables

- [ ] UUID.
- [ ] timestamp.
- [ ] ISO timestamp.
- [ ] random integer.
- [ ] random string.
- [ ] random email.
- [ ] random IP.
- [ ] random date.
- [ ] cryptographic bytes.
- [ ] extension API.

---

# 18. Environments

- [ ] create environment.
- [ ] rename.
- [ ] duplicate.
- [ ] delete.
- [ ] environment table.
- [ ] initial/default value.
- [ ] current/local value.
- [ ] secret flag.
- [ ] type.
- [ ] description.
- [ ] environment selector.
- [ ] quick switch keyboard shortcut.
- [ ] import.
- [ ] export.
- [ ] diff environments.
- [ ] clone environment.
- [ ] detect missing production values.

---

# 19. Vault

## 19.1 OS Storage

- [ ] macOS Keychain integration.
- [ ] Windows Credential Manager integration.
- [ ] Linux Secret Service integration.
- [ ] fallback encrypted vault.

## 19.2 Vault UX

- [ ] create secret.
- [ ] edit.
- [ ] delete.
- [ ] reveal.
- [ ] copy.
- [ ] clipboard auto-clear.
- [ ] allowed domains.
- [ ] tags.
- [ ] search.
- [ ] `{{vault:name}}` references.
- [ ] insert secret reference picker.

## 19.3 Security Tests

- [ ] secret absent from workspace files.
- [ ] secret absent from history.
- [ ] secret absent from logs.
- [ ] secret absent from crash reports.
- [ ] secret absent from default exports.
- [ ] domain restriction works.
- [ ] encrypted vault migration test.

---

# 20. Cookies

- [ ] cookie parser.
- [ ] cookie jar.
- [ ] domain matching.
- [ ] path matching.
- [ ] expiry.
- [ ] Secure.
- [ ] HttpOnly.
- [ ] SameSite.
- [ ] response Set-Cookie handling.
- [ ] request Cookie generation.
- [ ] cookie manager UI.
- [ ] edit cookie.
- [ ] delete.
- [ ] clear domain.
- [ ] disable jar.
- [ ] incognito jar.

---

# 21. Authentication

## 21.1 Architecture

- [ ] auth provider trait.
- [ ] auth inheritance.
- [ ] collection auth.
- [ ] folder auth.
- [ ] request auth.
- [ ] redaction-aware auth output.

## 21.2 Providers

- [ ] No Auth.
- [ ] Inherit.
- [ ] API Key.
- [ ] Bearer.
- [ ] Basic.
- [ ] Digest.
- [ ] OAuth 1.0.
- [ ] OAuth 2.0.
- [ ] JWT Bearer.
- [ ] AWS Signature v4.
- [ ] Hawk.
- [ ] NTLM.
- [ ] custom plugin provider placeholder.

## 21.3 OAuth 2.0

- [ ] Authorization Code.
- [ ] PKCE.
- [ ] Client Credentials.
- [ ] Device Code.
- [ ] legacy Password grant compatibility.
- [ ] scopes.
- [ ] audience.
- [ ] custom parameters.
- [ ] callback listener.
- [ ] browser launch.
- [ ] CSRF/state validation.
- [ ] token refresh.
- [ ] expiry.
- [ ] vault-backed token storage.
- [ ] revoke/delete token.
- [ ] multiple tokens per config.

---

# 22. Import

## 22.1 Framework

- [ ] importer trait.
- [ ] detection scoring.
- [ ] import preview.
- [ ] warnings.
- [ ] conflict resolution.
- [ ] transactional import.
- [ ] import rollback.
- [ ] unknown field reporting.

## 22.2 Formats

- [ ] cURL.
- [ ] Postman Collection v2.1.
- [ ] Postman environment.
- [ ] OpenAPI 3.
- [ ] Swagger 2.
- [ ] HAR.
- [ ] Insomnia.
- [ ] Bruno.
- [ ] Hoppscotch.
- [ ] Thunder Client.
- [ ] GraphQL SDL.
- [ ] protobuf.
- [ ] raw URL.

## 22.3 Import Sources

- [ ] file.
- [ ] folder.
- [ ] drag/drop.
- [ ] clipboard text.
- [ ] URL.
- [ ] Git repository later.

---

# 23. Export

- [ ] native workspace.
- [ ] native collection.
- [ ] Postman Collection v2.1.
- [ ] environment.
- [ ] OpenAPI.
- [ ] cURL.
- [ ] request as HAR where sensible.
- [ ] run results JSON.
- [ ] JUnit XML.
- [ ] unresolved-values mode.
- [ ] resolved-values mode.
- [ ] secret confirmation.
- [ ] deterministic export tests.

---

# 24. Scripting Runtime

## 24.1 Process Architecture

- [ ] script worker executable.
- [ ] versioned IPC.
- [ ] process launch.
- [ ] process health.
- [ ] automatic restart.
- [ ] kill hung process.
- [ ] clean shutdown.
- [ ] crash isolation test.

## 24.2 JavaScript Engine

- [ ] benchmark QuickJS.
- [ ] benchmark V8/deno_core if needed.
- [ ] select engine.
- [ ] basic eval.
- [ ] async/promise support.
- [ ] timers.
- [ ] JSON.
- [ ] crypto API subset.
- [ ] URL API.
- [ ] package loading strategy.

## 24.3 Sandbox

- [ ] CPU timeout.
- [ ] wall timeout.
- [ ] memory limit.
- [ ] no filesystem by default.
- [ ] no subprocess by default.
- [ ] controlled network calls.
- [ ] controlled vault access.
- [ ] redacted exceptions.
- [ ] script cancellation.

## 24.4 Lifecycle

- [ ] collection pre-request.
- [ ] folder pre-request.
- [ ] request pre-request.
- [ ] post-response request.
- [ ] post-response folder.
- [ ] post-response collection.
- [ ] execution order tests.

## 24.5 Script API

- [ ] `ps.request`.
- [ ] `ps.response`.
- [ ] `ps.variables`.
- [ ] `ps.environment`.
- [ ] `ps.collectionVariables`.
- [ ] `ps.globals`.
- [ ] `ps.cookies`.
- [ ] `ps.test`.
- [ ] `ps.expect`.
- [ ] `ps.sendRequest`.
- [ ] `ps.execution`.
- [ ] `ps.vault`.
- [ ] console.
- [ ] visualizer API.
- [ ] compatibility namespace strategy.

---

# 25. Tests and Assertions

- [ ] test registration.
- [ ] pass.
- [ ] fail.
- [ ] skip.
- [ ] nested descriptions.
- [ ] status assertions.
- [ ] header assertions.
- [ ] cookie assertions.
- [ ] response time.
- [ ] body contains.
- [ ] JSON property.
- [ ] JSON schema.
- [ ] XML assertion.
- [ ] custom predicate.
- [ ] source location.
- [ ] failure stack.
- [ ] test results UI.
- [ ] summary counts.
- [ ] copy failure.
- [ ] filter failures.

---

# 26. Package Library

- [ ] package model.
- [ ] local package.
- [ ] workspace package.
- [ ] package version.
- [ ] package documentation.
- [ ] imports.
- [ ] package dependency graph.
- [ ] circular dependency detection.
- [ ] npm/JSR feasibility spike.
- [ ] safe external package strategy.
- [ ] package cache.
- [ ] package integrity/checksum.

---

# 27. Collection Runner

## 27.1 Runner UI

- [ ] choose collection.
- [ ] choose folder.
- [ ] environment.
- [ ] iterations.
- [ ] delay.
- [ ] data file.
- [ ] stop-on-error.
- [ ] persist variables option.
- [ ] save response option.
- [ ] start.
- [ ] cancel.
- [ ] live progress.

## 27.2 Runner Engine

- [ ] deterministic request order.
- [ ] script lifecycle.
- [ ] iteration variables.
- [ ] CSV data.
- [ ] JSON data.
- [ ] retry policy.
- [ ] failure handling.
- [ ] cancellation.
- [ ] resource cleanup.

## 27.3 Reports

- [ ] run overview.
- [ ] request durations.
- [ ] tests.
- [ ] failures.
- [ ] logs.
- [ ] JSON export.
- [ ] JUnit XML.
- [ ] comparison with previous run later.

---

# 28. CLI

## 28.1 CLI Foundation

- [ ] CLI crate.
- [ ] `clap`.
- [ ] structured exit codes.
- [ ] JSON output.
- [ ] no-color option.
- [ ] quiet mode.
- [ ] verbose/debug.
- [ ] config discovery.
- [ ] vault access.

## 28.2 Commands

- [ ] `request send`.
- [ ] `collection run`.
- [ ] `test`.
- [ ] `import`.
- [ ] `export`.
- [ ] `env list`.
- [ ] `env use`.
- [ ] `mock start`.
- [ ] `monitor run`.
- [ ] `spec lint`.
- [ ] `version`.

## 28.3 CI

- [ ] GitHub Actions example.
- [ ] GitLab CI example.
- [ ] generic shell example.
- [ ] JUnit integration.
- [ ] non-zero test failure exit.
- [ ] secret-safe logs.

---

# 29. GraphQL

- [ ] dedicated GraphQL request model.
- [ ] endpoint.
- [ ] query editor.
- [ ] variables.
- [ ] headers.
- [ ] auth.
- [ ] send query.
- [ ] operation selector.
- [ ] introspection.
- [ ] schema cache.
- [ ] schema explorer.
- [ ] autocomplete.
- [ ] diagnostics.
- [ ] documentation hover.
- [ ] fragments.
- [ ] subscriptions.
- [ ] saved examples.
- [ ] import SDL.
- [ ] export relevant request.

---

# 30. WebSocket

- [ ] ws.
- [ ] wss.
- [ ] connection headers.
- [ ] query params.
- [ ] cookies.
- [ ] auth.
- [ ] subprotocol.
- [ ] connect.
- [ ] disconnect.
- [ ] reconnect.
- [ ] send text.
- [ ] send binary.
- [ ] receive text.
- [ ] receive binary.
- [ ] ping/pong.
- [ ] message timestamps.
- [ ] message direction.
- [ ] filter.
- [ ] search.
- [ ] save message templates.
- [ ] export messages.

---

# 31. SSE

- [ ] connect through HTTP engine.
- [ ] parse event stream.
- [ ] ID.
- [ ] event type.
- [ ] data.
- [ ] retry metadata.
- [ ] event timeline.
- [ ] filter.
- [ ] search.
- [ ] save events.
- [ ] cancellation/reconnect.

---

# 32. gRPC

## 32.1 Protobuf

- [ ] load `.proto`.
- [ ] load directory.
- [ ] import resolution.
- [ ] well-known types.
- [ ] descriptor generation.
- [ ] dynamic message support.
- [ ] schema diagnostics.

## 32.2 Client

- [ ] server endpoint.
- [ ] TLS.
- [ ] metadata.
- [ ] auth.
- [ ] reflection.
- [ ] service tree.
- [ ] method tree.
- [ ] request editor.
- [ ] message validation.
- [ ] unary.
- [ ] server streaming.
- [ ] client streaming.
- [ ] bidirectional.
- [ ] deadline.
- [ ] cancellation.
- [ ] message timeline.
- [ ] save examples.

---

# 33. Socket.IO

- [ ] connection.
- [ ] protocol versions.
- [ ] namespaces.
- [ ] auth payload.
- [ ] headers.
- [ ] query.
- [ ] emit.
- [ ] listen.
- [ ] acknowledgement.
- [ ] binary.
- [ ] reconnect.
- [ ] event history.
- [ ] saved event templates.
- [ ] filtering.

---

# 34. MQTT

- [ ] MQTT 3.1.1.
- [ ] MQTT 5.
- [ ] TCP.
- [ ] TLS.
- [ ] broker config.
- [ ] client ID.
- [ ] username/password.
- [ ] certs.
- [ ] keepalive.
- [ ] clean session/start.
- [ ] QoS 0.
- [ ] QoS 1.
- [ ] QoS 2.
- [ ] subscribe.
- [ ] unsubscribe.
- [ ] wildcard subscriptions.
- [ ] publish.
- [ ] retain.
- [ ] last will.
- [ ] MQTT 5 properties.
- [ ] message timeline.
- [ ] filter/search.

---

# 35. MCP Client

- [ ] MCP request model.
- [ ] stdio transport.
- [ ] Streamable HTTP transport.
- [ ] initialize.
- [ ] capabilities.
- [ ] tools/list.
- [ ] tools/call.
- [ ] resources/list.
- [ ] resources/read.
- [ ] prompts/list.
- [ ] prompts/get.
- [ ] notifications.
- [ ] server logs.
- [ ] JSON-RPC trace.
- [ ] process lifecycle.
- [ ] local process permission prompt.
- [ ] environment + secret references.
- [ ] saved MCP server configuration.

---

# 36. AI Requests

- [ ] provider abstraction.
- [ ] OpenAI-compatible provider.
- [ ] Anthropic provider.
- [ ] Gemini provider.
- [ ] Ollama.
- [ ] LM Studio.
- [ ] model list.
- [ ] message editor.
- [ ] system prompt.
- [ ] streaming.
- [ ] tools.
- [ ] structured output.
- [ ] temperature.
- [ ] top-p.
- [ ] token limits.
- [ ] multimodal attachments.
- [ ] usage metadata.
- [ ] cost estimate where pricing config exists.
- [ ] raw protocol inspection.
- [ ] BYOK vault integration.
- [ ] MCP attachment later.

---

# 37. SOAP

- [ ] SOAP request preset.
- [ ] XML editor.
- [ ] SOAPAction.
- [ ] namespaces.
- [ ] WSDL import.
- [ ] service explorer.
- [ ] operation explorer.
- [ ] request template generation.
- [ ] response formatting.
- [ ] XSD validation.
- [ ] WS-Security research.
- [ ] basic WS-Security if adopted.

---

# 38. Specs

## 38.1 OpenAPI

- [ ] OpenAPI 3.0 parse.
- [ ] OpenAPI 3.1 parse.
- [ ] Swagger 2 conversion/import.
- [ ] validation.
- [ ] `$ref`.
- [ ] multi-file refs.
- [ ] remote refs.
- [ ] outline.
- [ ] errors panel.
- [ ] format.
- [ ] lint.
- [ ] operation browser.

## 38.2 GraphQL Spec

- [ ] SDL parse.
- [ ] validation.
- [ ] outline.
- [ ] diagnostics.
- [ ] schema browser.

## 38.3 Protobuf Spec

- [ ] proto editor.
- [ ] diagnostics.
- [ ] symbol outline.
- [ ] imports.

## 38.4 AsyncAPI

- [ ] feasibility.
- [ ] parse.
- [ ] validate.
- [ ] editor.
- [ ] generate protocol resources where feasible.

## 38.5 Collection Generation

- [ ] spec → collection.
- [ ] operations.
- [ ] parameters.
- [ ] auth.
- [ ] examples.
- [ ] schemas.
- [ ] server variables.
- [ ] regeneration.
- [ ] preview diff.
- [ ] preserve scripts where possible.

---

# 39. Documentation Generator

- [ ] collection docs model.
- [ ] request docs.
- [ ] auth docs.
- [ ] param tables.
- [ ] headers.
- [ ] bodies.
- [ ] examples.
- [ ] code samples.
- [ ] navigation.
- [ ] search.
- [ ] dark/light.
- [ ] live preview.
- [ ] static export.
- [ ] Markdown export.
- [ ] custom branding later.
- [ ] versioned docs later.

---

# 40. Mock Server

## 40.1 Core

- [ ] local listener.
- [ ] configurable bind address.
- [ ] configurable port.
- [ ] HTTP routing.
- [ ] collection examples.
- [ ] mock definitions.
- [ ] graceful shutdown.

## 40.2 Matchers

- [ ] method.
- [ ] path.
- [ ] query.
- [ ] selected headers.
- [ ] exact body.
- [ ] JSON body partial.
- [ ] regex later.
- [ ] priority.
- [ ] fallback.

## 40.3 Responses

- [ ] status.
- [ ] headers.
- [ ] body.
- [ ] delay.
- [ ] templating.
- [ ] request values in templates.
- [ ] dynamic variables.
- [ ] random failure.
- [ ] script-generated response later.

## 40.4 Logs

- [ ] request log.
- [ ] match result.
- [ ] unmatched diagnostics.
- [ ] response.
- [ ] timing.
- [ ] clear logs.
- [ ] save incoming request.

---

# 41. Daemon

- [ ] daemon binary.
- [ ] local IPC.
- [ ] authenticated local control.
- [ ] lifecycle.
- [ ] auto-start option.
- [ ] OS service integration later.
- [ ] scheduled monitors.
- [ ] persistent mocks.
- [ ] capture ownership.
- [ ] logs.
- [ ] crash recovery.
- [ ] version compatibility with desktop.
- [ ] upgrade behavior.

---

# 42. Monitoring

## 42.1 Definitions

- [ ] monitor model.
- [ ] collection/folder target.
- [ ] environment.
- [ ] timeout.
- [ ] retries.
- [ ] schedule.
- [ ] enabled state.

## 42.2 Scheduling

- [ ] run now.
- [ ] cron.
- [ ] interval.
- [ ] pause.
- [ ] resume.
- [ ] missed-run policy.
- [ ] concurrent-run policy.

## 42.3 Results

- [ ] run history.
- [ ] success/failure.
- [ ] duration.
- [ ] tests.
- [ ] failure streak.
- [ ] uptime.
- [ ] response time chart.

## 42.4 Alerts

- [ ] desktop notifications.
- [ ] generic webhook.
- [ ] Discord webhook.
- [ ] Slack webhook.
- [ ] SMTP email later.
- [ ] alert deduplication.
- [ ] recovery notification.
- [ ] rate limiting.

---

# 43. Performance Testing

## 43.1 Engine

- [ ] virtual user model.
- [ ] fixed VUs.
- [ ] ramp-up.
- [ ] ramp-down.
- [ ] stage configuration.
- [ ] duration.
- [ ] cancellation.
- [ ] connection pooling.
- [ ] backpressure.
- [ ] script/test support.

## 43.2 Metrics

- [ ] requests/sec.
- [ ] iterations/sec.
- [ ] errors/sec.
- [ ] bytes/sec.
- [ ] min.
- [ ] max.
- [ ] average.
- [ ] p50.
- [ ] p75.
- [ ] p90.
- [ ] p95.
- [ ] p99.
- [ ] active VUs.
- [ ] status distribution.
- [ ] assertion failures.

## 43.3 UI

- [ ] configuration screen.
- [ ] live metrics.
- [ ] charts.
- [ ] errors.
- [ ] failed tests.
- [ ] final summary.
- [ ] previous runs.
- [ ] export JSON.
- [ ] export CSV.

## 43.4 Safety

- [ ] VU cap.
- [ ] target warning.
- [ ] public-host high-load confirmation.
- [ ] memory cap.
- [ ] response body discard policy.
- [ ] safe defaults.

---

# 44. Traffic Capture

## 44.1 Proxy

- [ ] local HTTP proxy.
- [ ] HTTPS CONNECT.
- [ ] capture HTTP.
- [ ] forward requests.
- [ ] response capture.
- [ ] timing.
- [ ] streaming.

## 44.2 CA

- [ ] local CA generation.
- [ ] secure private key storage.
- [ ] explicit installation.
- [ ] remove instructions.
- [ ] regenerate.
- [ ] per-host leaf cert generation.
- [ ] never sync CA private key.
- [ ] certificate security documentation.

## 44.3 Sessions

- [ ] start.
- [ ] pause.
- [ ] resume.
- [ ] stop.
- [ ] session history.
- [ ] request list.
- [ ] response details.

## 44.4 Filtering

- [ ] host.
- [ ] path.
- [ ] method.
- [ ] status.
- [ ] content type.
- [ ] time.
- [ ] process if platform allows.

## 44.5 Conversion

- [ ] save request.
- [ ] save request + response example.
- [ ] save selected requests as collection.
- [ ] capture cookies to jar.

---

# 45. Console and Diagnostics

- [ ] global console panel.
- [ ] network events.
- [ ] script logs.
- [ ] test logs.
- [ ] redirects.
- [ ] TLS errors.
- [ ] proxy events.
- [ ] cookies.
- [ ] variable warnings.
- [ ] filter by request.
- [ ] log levels.
- [ ] search.
- [ ] copy.
- [ ] clear.
- [ ] secret redaction tests.

---

# 46. Code Generation

## 46.1 Framework

- [ ] codegen trait.
- [ ] normalized prepared request model.
- [ ] secret policy.
- [ ] plugin extension point.
- [ ] snapshot tests.

## 46.2 Generators

- [ ] cURL.
- [ ] HTTPie.
- [ ] JavaScript fetch.
- [ ] Axios.
- [ ] Node.js https.
- [ ] Rust reqwest.
- [ ] Python requests.
- [ ] Python httpx.
- [ ] Go.
- [ ] Java.
- [ ] Kotlin.
- [ ] C#.
- [ ] PHP.
- [ ] Ruby.
- [ ] Swift.
- [ ] Dart.

---

# 47. Visualizer

## 47.1 Native Visualization Model

- [ ] visualization schema.
- [ ] table.
- [ ] key/value.
- [ ] Markdown.
- [ ] image.
- [ ] bar chart.
- [ ] line chart.
- [ ] scatter.
- [ ] pie/donut.
- [ ] metric cards.

## 47.2 Script Integration

- [ ] `ps.visualizer.set`.
- [ ] pass response data.
- [ ] render errors safely.
- [ ] size limits.
- [ ] no arbitrary native access.

## 47.3 HTML Mode — Optional

- [ ] threat model.
- [ ] sandboxed webview process.
- [ ] CSP.
- [ ] no filesystem.
- [ ] no unrestricted network.
- [ ] explicit feature flag.

---

# 48. Git Integration

## 48.1 Repository Detection

- [ ] detect `.git`.
- [ ] show branch.
- [ ] show clean/dirty.
- [ ] refresh file changes.
- [ ] external Git operation refresh.

## 48.2 Operations

- [ ] diff.
- [ ] stage.
- [ ] unstage.
- [ ] commit.
- [ ] history.
- [ ] checkout branch.
- [ ] pull.
- [ ] push.
- [ ] fetch.
- [ ] conflict detection.

## 48.3 Semantic Diff

- [ ] request method.
- [ ] URL.
- [ ] params.
- [ ] headers.
- [ ] auth.
- [ ] body.
- [ ] scripts.
- [ ] variables.
- [ ] examples.
- [ ] collection metadata.

## 48.4 Secret Safety

- [ ] pre-stage secret scan.
- [ ] warn known vault values.
- [ ] configurable patterns.
- [ ] never stage `.packetsmith` private state by default.

---

# 49. Search

- [ ] SQLite FTS index.
- [ ] resource names.
- [ ] URL.
- [ ] descriptions.
- [ ] headers.
- [ ] bodies.
- [ ] scripts.
- [ ] variable names.
- [ ] specs.
- [ ] incremental updates.
- [ ] fuzzy quick-open.
- [ ] text search.
- [ ] regex.
- [ ] filter syntax.
- [ ] result preview.
- [ ] jump to match.

---

# 50. Collaboration Server

## 50.1 Server Foundation

- [ ] choose server framework.
- [ ] database.
- [ ] migrations.
- [ ] authentication.
- [ ] API versioning.
- [ ] WebSocket/realtime channel.
- [ ] rate limiting.
- [ ] audit log.
- [ ] self-host deployment.

## 50.2 Accounts and Teams

- [ ] user.
- [ ] organization.
- [ ] team.
- [ ] workspace membership.
- [ ] invite.
- [ ] Owner.
- [ ] Admin.
- [ ] Editor.
- [ ] Commenter.
- [ ] Viewer.

## 50.3 Sync

- [ ] resource revision.
- [ ] local op journal.
- [ ] push.
- [ ] pull.
- [ ] conflict detection.
- [ ] conflict UI.
- [ ] offline queue.
- [ ] retry.
- [ ] tombstones.
- [ ] restore.
- [ ] large blob sync.

## 50.4 Real-Time

- [ ] presence.
- [ ] workspace presence.
- [ ] current resource presence.
- [ ] live resource updates.
- [ ] CRDT spike for text editing.
- [ ] adopt/reject CRDT based on spike.

---

# 51. Comments and Activity

- [ ] comments on collection.
- [ ] comments on folder.
- [ ] comments on request.
- [ ] comments on example.
- [ ] comments on spec.
- [ ] replies.
- [ ] resolve.
- [ ] reopen.
- [ ] mentions.
- [ ] activity feed.
- [ ] resource change activity.
- [ ] audit events.

---

# 52. Plugin System

## 52.1 API

- [ ] plugin API version.
- [ ] manifest schema.
- [ ] plugin IDs.
- [ ] capabilities.
- [ ] permission model.
- [ ] error model.
- [ ] compatibility policy.

## 52.2 Runtime

- [ ] choose WASM runtime.
- [ ] load plugin.
- [ ] validate manifest.
- [ ] verify checksum/signature policy.
- [ ] sandbox.
- [ ] timeout.
- [ ] memory limit.
- [ ] unload.
- [ ] crash isolation.

## 52.3 Capabilities

- [ ] command.
- [ ] importer.
- [ ] exporter.
- [ ] code generator.
- [ ] auth provider.
- [ ] response formatter.
- [ ] dynamic variable.
- [ ] visualizer.
- [ ] spec linter.
- [ ] notification adapter.
- [ ] secret provider.

## 52.4 Permissions

- [ ] workspace read.
- [ ] workspace write.
- [ ] network allowlist.
- [ ] secret allowlist.
- [ ] clipboard.
- [ ] notification.
- [ ] user-selected files.
- [ ] deny subprocess.

## 52.5 Plugin UX

- [ ] install local file.
- [ ] install from URL/index later.
- [ ] enable.
- [ ] disable.
- [ ] update.
- [ ] uninstall.
- [ ] permission review.
- [ ] plugin logs.
- [ ] plugin diagnostics.

---

# 53. Flows

## 53.1 Canvas

- [ ] infinite canvas.
- [ ] pan.
- [ ] zoom.
- [ ] select.
- [ ] multi-select.
- [ ] drag nodes.
- [ ] connect ports.
- [ ] delete.
- [ ] duplicate.
- [ ] undo/redo.
- [ ] minimap later.

## 53.2 Nodes

- [ ] request.
- [ ] condition.
- [ ] switch.
- [ ] loop.
- [ ] delay.
- [ ] transform.
- [ ] script.
- [ ] variable.
- [ ] input.
- [ ] output.
- [ ] webhook trigger.
- [ ] schedule trigger.

## 53.3 Runtime

- [ ] graph validation.
- [ ] cycle rules.
- [ ] DAG plan.
- [ ] parallel branches.
- [ ] retries.
- [ ] timeout.
- [ ] cancellation.
- [ ] node logs.
- [ ] persisted run.
- [ ] resume policy.

---

# 54. AI Assistance

## 54.1 Provider Configuration

- [ ] BYOK only default.
- [ ] provider abstraction.
- [ ] model configuration.
- [ ] vault keys.
- [ ] request preview before sending context.

## 54.2 Actions

- [ ] explain response.
- [ ] generate tests.
- [ ] generate docs.
- [ ] create request.
- [ ] create collection.
- [ ] create example.
- [ ] diagnose auth error.
- [ ] compare responses.
- [ ] generate mock.
- [ ] fix OpenAPI diagnostics.

## 54.3 Privacy

- [ ] show context being sent.
- [ ] redact vault secrets.
- [ ] configurable redaction.
- [ ] no hidden uploads.
- [ ] local model option.

---

# 55. Updates and Packaging

## 55.1 macOS

- [ ] Apple Silicon.
- [ ] Intel decision.
- [ ] DMG.
- [ ] application icon.
- [ ] code signing.
- [ ] notarization.
- [ ] update signature.

## 55.2 Windows

- [ ] x64.
- [ ] ARM64 decision.
- [ ] installer.
- [ ] Start menu.
- [ ] uninstall.
- [ ] code signing.
- [ ] protocol/file associations if adopted.

## 55.3 Linux

- [ ] tarball.
- [ ] AppImage.
- [ ] deb.
- [ ] rpm.
- [ ] desktop entry.
- [ ] icons.
- [ ] Wayland validation.
- [ ] X11 validation.
- [ ] Flatpak later.

## 55.4 Updater

- [ ] update manifest.
- [ ] signature.
- [ ] stable channel.
- [ ] beta.
- [ ] nightly.
- [ ] download.
- [ ] verify.
- [ ] restart.
- [ ] release notes.
- [ ] package-manager opt-out.

---

# 56. Security Program

## 56.1 Static and Dependency

- [ ] cargo audit.
- [ ] cargo deny.
- [ ] license checks.
- [ ] dependency update policy.
- [ ] SBOM.
- [ ] secret scan.

## 56.2 Runtime

- [ ] all secrets redacted.
- [ ] TLS verify default.
- [ ] secure temp files.
- [ ] import path traversal tests.
- [ ] zip bomb protection.
- [ ] body size limits where parsing.
- [ ] remote ref timeout.
- [ ] remote ref size limit.
- [ ] script sandbox.
- [ ] plugin sandbox.
- [ ] proxy CA key protection.

## 56.3 Release

- [ ] signed builds.
- [ ] signed update manifests.
- [ ] checksum publication.
- [ ] provenance/attestation later.
- [ ] vulnerability reporting process.
- [ ] security response policy.

---

# 57. Privacy and Telemetry

- [ ] core works with telemetry disabled.
- [ ] telemetry decision documented.
- [ ] opt-in/opt-out UX.
- [ ] list collected fields.
- [ ] never collect request bodies by default.
- [ ] never collect URLs by default.
- [ ] never collect headers.
- [ ] never collect secrets.
- [ ] crash report preview/redaction.
- [ ] disable network analytics entirely via setting/env var.

---

# 58. Crash Recovery and Reliability

- [ ] panic hook.
- [ ] crash-safe logs.
- [ ] autosave draft state.
- [ ] restore tabs.
- [ ] restore splits.
- [ ] restore active workspace.
- [ ] database integrity check.
- [ ] cache rebuild.
- [ ] corrupted resource quarantine.
- [ ] migration rollback.
- [ ] script worker restart.
- [ ] daemon reconnect.
- [ ] plugin crash containment.

---

# 59. Performance

## 59.1 Benchmarks

- [ ] cold startup.
- [ ] warm startup.
- [ ] 1k-resource workspace.
- [ ] 10k-resource workspace.
- [ ] 100k search docs stress test.
- [ ] request overhead.
- [ ] 10 MB JSON.
- [ ] 100 MB JSON/raw.
- [ ] 1 GB streaming download.
- [ ] 1k history entries.
- [ ] 100k history entries.
- [ ] collection runner.
- [ ] script startup.
- [ ] mock server.

## 59.2 UI Performance

- [ ] resource tree virtualized.
- [ ] response lines virtualized.
- [ ] history virtualized.
- [ ] no synchronous network.
- [ ] no synchronous large file parse.
- [ ] debounce search.
- [ ] background formatting.
- [ ] frame-time profiling.

---

# 60. Test Infrastructure

## 60.1 HTTP Fixture Server

- [ ] simple JSON.
- [ ] echo.
- [ ] headers.
- [ ] cookies.
- [ ] redirects.
- [ ] slow response.
- [ ] chunked.
- [ ] compressed.
- [ ] upload.
- [ ] multipart.
- [ ] large response.
- [ ] auth.
- [ ] TLS.
- [ ] client cert.

## 60.2 Protocol Fixtures

- [ ] GraphQL.
- [ ] WebSocket.
- [ ] SSE.
- [ ] gRPC.
- [ ] Socket.IO.
- [ ] MQTT.
- [ ] MCP.
- [ ] SOAP.

## 60.3 Compatibility Corpus

- [ ] Postman fixtures.
- [ ] Insomnia fixtures.
- [ ] Bruno fixtures.
- [ ] Hoppscotch fixtures.
- [ ] OpenAPI public specs.
- [ ] GraphQL public schemas.
- [ ] protobuf projects.
- [ ] malformed/hostile fixtures.

---

# 61. Product UX Polish

- [ ] onboarding.
- [ ] first request empty state.
- [ ] import prompt.
- [ ] recent workspaces.
- [ ] keyboard shortcut hints.
- [ ] command palette discoverability.
- [ ] status bar.
- [ ] environment visibility.
- [ ] update indicator.
- [ ] offline indicator for cloud features.
- [ ] errors provide next actions.
- [ ] confirmation only for destructive actions.
- [ ] consistent context menus.
- [ ] drag/drop feedback.
- [ ] loading states.
- [ ] no UI freeze during sends.

---

# 62. Documentation for Users

- [ ] installation.
- [ ] first request.
- [ ] collections.
- [ ] environments.
- [ ] variables.
- [ ] vault.
- [ ] auth.
- [ ] scripts.
- [ ] tests.
- [ ] runner.
- [ ] CLI.
- [ ] GraphQL.
- [ ] gRPC.
- [ ] WebSocket.
- [ ] MQTT.
- [ ] MCP.
- [ ] mocks.
- [ ] monitors.
- [ ] performance.
- [ ] capture.
- [ ] imports.
- [ ] Git.
- [ ] plugins.
- [ ] self-host collaboration.

---

# 63. MVP Exit Gate

Do not publish "MVP" until all required items are true:

- [ ] app launches on macOS.
- [ ] app launches on Windows.
- [ ] app launches on Linux.
- [ ] user can create a workspace.
- [ ] user can create HTTP request.
- [ ] GET/POST/PUT/PATCH/DELETE work.
- [ ] params work.
- [ ] headers work.
- [ ] JSON/raw/form bodies work.
- [ ] response body displays.
- [ ] large response does not freeze UI.
- [ ] cancellation works.
- [ ] request can be saved.
- [ ] collections/folders work.
- [ ] environments work.
- [ ] variables work.
- [ ] basic/bearer/API-key auth work.
- [ ] OAuth 2.0 works.
- [ ] vault works.
- [ ] cookies work.
- [ ] history works.
- [ ] cURL import works.
- [ ] Postman v2.1 import works.
- [ ] OpenAPI import works.
- [ ] workspace files are Git-friendly.
- [ ] secrets are excluded from workspace files.
- [ ] crash recovery works.
- [ ] signed/usable installers exist or documented unsigned developer release policy is explicit.
- [ ] no known critical security issue.

---

# 64. v1.0 Exit Gate

- [ ] all MVP gates remain green.
- [ ] scripting.
- [ ] assertions.
- [ ] runner.
- [ ] CLI.
- [ ] JUnit.
- [ ] GraphQL.
- [ ] WebSocket.
- [ ] gRPC.
- [ ] codegen.
- [ ] mocks.
- [ ] specs.
- [ ] docs.
- [ ] import/export compatibility suite.
- [ ] plugin API foundation.
- [ ] file schema documented.
- [ ] migration strategy proven.
- [ ] security policy public.
- [ ] release process documented.
- [ ] no P0/P1 correctness issue.
- [ ] performance targets measured.
- [ ] accessibility pass on primary workflows.

---

# 65. Post-v1 Platform Gate

- [ ] MQTT.
- [ ] Socket.IO.
- [ ] MCP.
- [ ] AI requests.
- [ ] performance testing.
- [ ] monitors.
- [ ] daemon.
- [ ] traffic capture.
- [ ] semantic Git.
- [ ] collaboration sync.
- [ ] comments.
- [ ] real-time presence.
- [ ] plugin ecosystem.
- [ ] Flows.

---

# 66. Suggested First Milestone Sprint Order

Execute approximately in this dependency order:

- [ ] Foundation repository.
- [ ] GPUI shell.
- [ ] command system.
- [ ] design tokens/components.
- [ ] workspace manifest.
- [ ] request domain model.
- [ ] HTTP engine.
- [ ] request bar.
- [ ] params.
- [ ] headers.
- [ ] body.
- [ ] response metadata.
- [ ] streaming body.
- [ ] cancellation.
- [ ] history.
- [ ] collection tree.
- [ ] save/open request.
- [ ] variables.
- [ ] environments.
- [ ] bearer/basic/API-key auth.
- [ ] vault.
- [ ] Postman/cURL import.
- [ ] internal alpha.

Only after the internal alpha is comfortable for daily HTTP work should the project aggressively branch into scripting and additional protocols.

---

# 67. Final Project Completion Definition

The complete long-term vision is achieved when all of the following workflows are supported end to end:

- [ ] Design an API.
- [ ] Import an API.
- [ ] Create requests.
- [ ] Send HTTP/REST requests.
- [ ] Send GraphQL requests.
- [ ] Send gRPC requests.
- [ ] Use WebSockets.
- [ ] Use Socket.IO.
- [ ] Use MQTT.
- [ ] Use MCP.
- [ ] Test AI model APIs.
- [ ] Work with SOAP/SSE.
- [ ] Authenticate securely.
- [ ] Use variables/environments.
- [ ] Store secrets securely.
- [ ] Script requests.
- [ ] Assert responses.
- [ ] Run collections.
- [ ] Run in CI.
- [ ] Load test.
- [ ] Mock APIs.
- [ ] Monitor APIs.
- [ ] Generate docs.
- [ ] Generate code.
- [ ] Capture traffic.
- [ ] Search everything.
- [ ] Version work in Git.
- [ ] Collaborate optionally.
- [ ] Extend the application through plugins.
- [ ] Keep working with no account and no cloud.
- [ ] Export the user's work in open formats.

When these are true, PacketSmith is no longer a "Postman clone." It is an independent open API development platform.
