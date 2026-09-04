# macOS Setup Guide

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
