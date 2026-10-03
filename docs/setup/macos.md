# macOS Setup Guide

## Install a Release Build

Download the latest universal DMG from the
[releases page](https://github.com/Binary-Brawlers/PacketSmith/releases), open
it, and drag **PacketSmith** into **Applications**. Developer release builds are
ad-hoc signed; if macOS blocks the first launch, remove the quarantine flag:

```bash
xattr -dr com.apple.quarantine /Applications/PacketSmith.app
```

## Prerequisites

1. **Xcode Command Line Tools:**
   ```bash
   xcode-select --install
   ```

2. **Rust Toolchain:**
   Install Rust via `rustup`:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default stable
   rustup component add rustfmt clippy
   ```

## Building PacketSmith

From the repository root:
```bash
# Check all workspace crates
cargo check --workspace

# Run tests
cargo test --workspace

# Run with GPUI desktop interface (requires Metal support on macOS)
cargo run -p packetsmith-app --features gpui-ui
```

## Build an Installer

From the repository root:

```bash
# Universal DMG written to target/dist/
cargo xtask package --format all --arch universal
```

See [Desktop Packaging and Release Builds](../release/packaging.md) for signing,
notarization, and release workflow details.
