# Desktop workbench and text rendering

## Changes

The entry point previously constructed only `EnvironmentView`. It now constructs
`WorkbenchView`, with Requests as the initial screen and Environments as a second
surface. Both share the same environment controller. Request drafts retain their
own inputs, results, and in-flight operation when switching tabs or screens.

The HTTP editor uses `ps_http::desktop::send_desktop_request`, so network I/O runs
on Tokio and complete bounded responses return to GPUI asynchronously. Cancelling
aborts the operation; generation checks prevent an older completion replacing a
newer send. Closing the view aborts its outstanding operations. Responses larger
than 2 MiB stop with a clear error. Display previews are capped at 32,000 characters.

## Blank text investigation & resolution

The reported screenshot showed control backgrounds and containers, but zero text or glyphs.
Root cause identified: `crates/packetsmith-app/Cargo.toml` enabled the `gpui-ui` feature with `gpui_platform/runtime_shaders`, but omitted the `gpui_platform/font-kit` feature flag.

In GPUI:
- `gpui_platform/Cargo.toml` has `default = []` and defines `font-kit = ["gpui_macos/font-kit"]`.
- When `font-kit` is omitted on macOS, `gpui_macos/src/platform.rs` falls back to `gpui::NoopTextSystem`.
- `NoopTextSystem` accepts fonts and computes line layout bounding boxes, but returns empty raster bounds (`Bounds::default()`) and an empty byte buffer for every glyph (`rasterize_glyph`).
- As a result, GPUI's scene graph skipped rendering all text glyphs, leaving all UI boxes and controls completely blank.

Resolution:
1. Added `"gpui_platform/font-kit"` to the `gpui-ui` feature list in `crates/packetsmith-app/Cargo.toml`. This enables `gpui_macos/font-kit` and native CoreText font rasterization (`MacTextSystem`).
2. Configured `UI_FONT` in `crates/packetsmith-app/src/shell/typography.rs` to `.SystemUIFont`, GPUI's built-in cross-platform token that dynamically resolves to `.AppleSystemUIFont` (San Francisco) on macOS, `Segoe UI` on Windows, and desktop font fallbacks on Linux.

After closing the old window, restart from the project root:

```sh
cargo run -p packetsmith-app --features gpui-ui
```

If labels are still missing, retain the terminal's font/rasterization/rendering
errors and a screenshot. Do not mark this rendering issue verified until text is
visible in both Requests and Environments.

## Validation

Passed:

```sh
cargo check --workspace --all-targets --features packetsmith-app/gpui-ui
```

No application, build command, or test suite was run under the user's type-check-only
constraint. Regression targets cover malformed/unresolved URLs, error redaction,
request headers/body/query delivery to a local fixture, response retention and
sensitive header masking, and Unicode-safe preview truncation.

Manual acceptance:

1. Confirm headings, labels, placeholders, and typed text are visible on both screens.
2. Send a request to a local HTTP fixture; check body, headers, status, timing and size.
3. Send JSON and raw bodies with headers and repeated query keys.
4. Switch request tabs during a slow send; verify the result stays with its draft.
5. Cancel and immediately send again; ensure the old completion does not replace it.
6. Open a native workspace under Environments; choose an environment and use a
   `{{variable}}` in a request. Unresolved references must stop before sending.
7. Verify keyboard traversal and Cmd/Ctrl+Enter and Cmd/Ctrl+T.

Current limits: drafts are not saved, workspace requests open as session drafts,
response previews are bounded, and redirects are disabled. Full native integration
of persistence/history, splits, advanced body modes, and vault is separate work.
