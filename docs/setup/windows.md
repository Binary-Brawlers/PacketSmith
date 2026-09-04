# Windows Setup Guide

## Prerequisites

1. **Visual Studio C++ Build Tools:**
   Install Visual Studio 2022 Build Tools with the **Desktop development with C++** workload.

2. **Rust Toolchain:**
   Download and run `rustup-init.exe` from [rustup.rs](https://rustup.rs/).
   Ensure the `x86_64-pc-windows-msvc` target is default:
   ```cmd
   rustup default stable-x86_64-pc-windows-msvc
   rustup component add rustfmt clippy
   ```

## Building PacketSmith

From PowerShell or Command Prompt:

```powershell
# Check workspace
cargo check --workspace

# Run tests
cargo test --workspace

# Run desktop app
cargo run -p packetsmith-app --features gpui-ui
```
