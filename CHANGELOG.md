# Changelog

All notable changes to PacketSmith will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Phase 0 Technical Foundation:
  - Root Cargo workspace with modular crates (`packetsmith-app`, `ps-domain`, `ps-workspace`, `ps-storage`, `ps-request-engine`, `ps-http`, `ps-ui-components`, `ps-settings`, `ps-test-support`, `xtask`).
  - Pinned Rust toolchain (`1.94.0`) and workspace linter configuration.
  - Apache-2.0 licensing and GPL clean-room policy.
  - Multi-platform CI pipeline for macOS, Ubuntu, and Windows.
  - Initial Architecture Decision Records (ADRs 0001–0004).
