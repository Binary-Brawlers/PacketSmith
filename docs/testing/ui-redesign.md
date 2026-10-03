# Native desktop UI redesign

The GPUI shell uses neutral charcoal surfaces, a restrained blue accent, labeled
navigation, underlined request tabs, and shared button/input styles. Requests,
environments, history, command palette, and environment quick look use the same
palette and typography. The response viewer includes line numbers and tinted JSON
lines; request errors are shown inline. Shortcut labels follow the host platform.

All screens share the workbench root, so navigation shortcuts and overlays remain
available while managing environments. History uses compact two-line entries,
and response metadata wraps at narrower window sizes.

## Validation

Passed:

```sh
cargo build -p packetsmith-app --features gpui-ui
cargo check --workspace --all-targets --features packetsmith-app/gpui-ui
cargo test -p packetsmith-app --features gpui-ui
```

All 16 application tests passed. Changed Rust files pass rustfmt and the diff
passes `git diff --check`.

The executable was run natively on Linux under Xvfb, using GPUI's OpenGL renderer
with Mesa llvmpipe. Screenshots were captured directly from the desktop window
using `scrot`; they are not browser captures or mockups. A temporary workspace
and local HTTP fixture supplied the example requests and variables.

Manual checks covered saved request loading, environment switching, variable
resolution, a successful HTTP request, authorization tabs, history, an inline
error for an unresolved variable, and layout at 1020 × 640 and 1440 × 900.

## Captured screens

- [Requests and a real HTTP response](screenshots/requests.png)
- [Environment variables](screenshots/environments.png)
- [Request history](screenshots/history.png)
