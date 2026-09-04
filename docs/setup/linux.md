# Linux Setup Guide

## Prerequisites

On Debian/Ubuntu-based distributions:

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libvulkan-dev \
    libxkbcommon-dev \
    libfontconfig1-dev \
    libx11-xcb-dev
```

On Fedora:

```bash
sudo dnf install \
    gcc gcc-c++ pkgconfig \
    vulkan-loader-devel \
    libxkbcommon-devel \
    fontconfig-devel \
    libxcb-devel
```

## Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup component add rustfmt clippy
```

## Building PacketSmith

```bash
# Check workspace
cargo check --workspace

# Run tests
cargo test --workspace

# Run desktop app
cargo run -p packetsmith-app --features gpui-ui
```
