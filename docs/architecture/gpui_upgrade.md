# GPUI Pinning and Upgrade Process

## Background

PacketSmith uses Zed's [GPUI](https://github.com/zed-industries/zed) framework for GPU-accelerated desktop UI rendering.
Because GPUI is rapidly evolving, unpinned dependencies can introduce breaking API shifts or build instability.

## Pinning Strategy

1. **Exact Commit Pinning:**
   GPUI is pinned to a specific Git revision hash in `crates/packetsmith-app/Cargo.toml`.
2. **Feature Isolation:**
   The `gpui` dependency is gated under the `gpui-ui` feature flag. Core libraries, domain models, execution engines, and headless CLI workflows build cleanly without fetching or compiling the GPUI desktop stack.

## Upgrade Procedure

When upgrading to a newer GPUI revision:

1. Create an isolated branch: `git checkout -b chore/gpui-upgrade-<date>`.
2. Update the `rev` hash in `crates/packetsmith-app/Cargo.toml`.
3. Check for any breaking changes in GPUI element APIs or window event hooks.
4. Verify cross-platform compilation on macOS, Linux (X11 & Wayland), and Windows.
5. Merge via PR with full CI test validation.
