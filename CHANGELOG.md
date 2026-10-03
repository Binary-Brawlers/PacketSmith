# Changelog

All notable changes to PacketSmith will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- macOS universal `PacketSmith.app` and DMG packaging plus a per-user Windows
  NSIS installer and portable zip, produced by `cargo xtask package`.
- Automated release workflow with artifact checksums, generated release notes,
  and optional Apple Developer ID signing/notarization and Windows code signing.
- Placeholder PacketSmith application icon assets (PNG, ICO, ICNS) and a
  dependency-free generator under `packaging/icons/`.
- Native request save/open workflow with collection selection, Cmd/Ctrl+S,
  dirty tracking, and Save/Discard/Cancel when closing changed tabs.
- Backward-compatible HTTP workspace fields for headers, query parameters,
  and bodies; saves preserve existing request identity and known metadata.
- Credential-reference validation, external edit detection, and background,
  atomic replacement of existing request files.

### Fixed
- Shared HTTP preparation now sends persisted headers and bodies through both
  the desktop sender and protocol executor; Basic auth resolves variables before encoding.
- Query synchronization retains variable templates, repeated keys, and URL fragments.
- Applied authentication header Debug output always masks credential values.
- Contrast test tolerates f32 rounding at the maximum 21:1 ratio.
- Installed builds persist window state in the OS application data directory
  instead of the process working directory and open workspace pickers in the
  user home directory.

### Foundation
- Phase 0 Technical Foundation:
  - Root Cargo workspace with modular crates (`packetsmith-app`, `ps-domain`, `ps-workspace`, `ps-storage`, `ps-request-engine`, `ps-http`, `ps-ui-components`, `ps-settings`, `ps-test-support`, `xtask`).
  - Pinned Rust toolchain (`1.94.0`) and workspace linter configuration.
  - Apache-2.0 licensing and GPL clean-room policy.
  - Multi-platform CI pipeline for macOS, Ubuntu, and Windows.
  - Initial Architecture Decision Records (ADRs 0001–0004).
