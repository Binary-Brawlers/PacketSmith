# Windows Setup Guide

## Install a Release Build

Download the latest x64 setup executable from the
[releases page](https://github.com/Binary-Brawlers/PacketSmith/releases) and run
it. The installer is per-user (no administrator rights required), adds a Start
menu shortcut, and registers an uninstaller in **Settings > Apps**. A portable
zip is published alongside the installer. Developer release builds are
unsigned, so SmartScreen may warn on first launch.

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

## Build an Installer

Install [NSIS](https://nsis.sourceforge.io/) (`choco install nsis`), then from
the repository root:

```powershell
# Installer and portable zip written to target\dist\
cargo xtask package --format all --arch x64
```

See [Desktop Packaging and Release Builds](../release/packaging.md) for signing
and release workflow details.
