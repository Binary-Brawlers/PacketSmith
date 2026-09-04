# Contributing to PacketSmith

Thank you for contributing to PacketSmith! We welcome contributions to help build a fast, open, and local-first API development platform.

---

## Code of Conduct

Please review and adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## Licensing & GPL-Free Boundary Policy

> [!IMPORTANT]
> PacketSmith is licensed under the **Apache License 2.0**.
> We use Zed's **GPUI** framework (which is licensed under Apache-2.0).
> Most Zed application/editor code is licensed under GPL-3.0-or-later. To preserve PacketSmith's permissive Apache-2.0 license, **do NOT copy, vendor, or link GPL-licensed code from Zed or other GPL repositories into PacketSmith crates.** All UI components, editors, and utilities in PacketSmith are built clean-room against the Apache-2.0 GPUI API.

---

## Development Workflow

1. **Fork and Clone** the repository.
2. Ensure you have the pinned Rust toolchain installed:
   ```bash
   rustup show
   ```
3. Use `cargo xtask` for formatting, linting, and tests:
   ```bash
   cargo xtask fmt
   cargo xtask lint
   cargo xtask test
   ```
4. Keep PRs focused on vertical slices or specific tasks from [`packetsmith_implementation_checklist.md`](packetsmith_implementation_checklist.md).
5. Update tests and documentation alongside functional code.

---

## Architectural Boundaries

- **UI cannot directly perform network operations or access raw disk storage.** Always communicate through service interfaces and domain abstractions.
- **Protocol engines must remain independent of GPUI.** The desktop app, CLI, and background runner share the same core engine logic without GUI dependencies.
- See [`docs/architecture/crate_map.md`](docs/architecture/crate_map.md) for allowed crate dependency paths.
