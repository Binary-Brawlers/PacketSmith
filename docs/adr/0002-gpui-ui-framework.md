# ADR-0002: Selection and Pinning Strategy for GPUI

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

PacketSmith requires a high-performance, GPU-accelerated desktop UI that can handle streaming large responses (hundreds of megabytes), responsive code editors, tabs, and complex trees with sub-millisecond frame rendering times. Electron and web wrappers introduce significant memory overhead and sluggish response visualization.

## Considered Options

1. **GPUI (Zed Industries):** High-performance, GPU-accelerated Rust UI framework.
2. **Tauri / Webview:** Web frontend with Rust backend. High memory usage with large text payloads.
3. **Iced / Slint:** Alternative Rust native UI toolkits with less mature layout and text editing features.

## Decision Outcome

Chosen option: **GPUI**, pinned to an exact Git commit and isolated behind a feature flag (`gpui-ui`).

### Positive Consequences
- True GPU-accelerated 120fps UI performance with minimal memory overhead.
- Clean text rendering and flexible layout primitives.
- Pure Rust codebase across the entire platform.

### Negative Consequences
- GPUI is under active evolution; API breaking changes require structured, branch-isolated upgrade cycles.
- Feature gating is maintained to preserve headless CLI and core domain testing without compiling the graphics pipeline.
