# Environment verification

Section 18 is implemented. These scenarios are pending execution; this document
is not a record of passing runtime tests. The current task permits type checks
only, so no app, test suite, build, or server was started.

## Type checks

- `cargo check --workspace --all-targets`
- `cargo check --workspace --all-targets --features packetsmith-app/gpui-ui`

## Native interaction scenarios (macOS, Windows, Linux)

1. Open an existing workspace directory. Create Development and Production.
   Switch using the sidebar and Cmd/Ctrl+Shift+E, including No environment.
   Confirm the active header follows selection and Tab/Shift+Tab reaches controls.
2. Add string, number, boolean, JSON, and secret-reference variables. Edit names,
   enabled flags, descriptions, and defaults. Invalid types or duplicate keys
   must report an error without replacing the previous variable.
3. Apply a non-secret current override, an explicit empty override, and Use default.
   Reopen the workspace and verify selection and non-secret overrides restore.
4. Add a secret variable with an empty default. Apply a raw current credential.
   Verify the input/table/diff remain masked and Copy/Cut do not copy it. Confirm
   the credential is absent from native environment files, local snapshots, and
   exports. Restart: the secret override must be absent.
5. Start changing a public variable to secret without saving. Applying a current
   credential must be rejected until classification is saved. Then apply it and
   confirm it remains session-only. Switching with an open row editor requires
   Save or Cancel, so drafts are not silently lost.
6. Set a vault-reference default; edit unrelated metadata while leaving the masked
   default blank. The reference must survive. Clear default must remove it.
7. Rename and inspect the native file: its path and ID must remain stable.
   Duplicate/clone: the new ID differs, defaults match, and current values are
   absent. Rename the clone to the desired name.
8. Export to a new YAML path and import it. The import gets a fresh ID. Import
   malformed YAML: no parser snippet or credential appears in the error. Export
   to an existing file: it remains unchanged and an error is shown.
9. Choose Development as reference while Production is active. Verify added,
   removed, default/current, type, description, enabled, and secret flag
   differences. No raw secret values appear. Missing checks include required
   reference keys absent/disabled in Production and empty enabled target values.
10. Cancel environment deletion, then confirm it. Verify the native file/local
    overrides disappear and the active selection/resolver return to no environment.
11. Rescan with a session secret: compatible session values survive. Externally
    remove a key or change its type/secret flag and rescan: its override is dropped.
12. Test Unicode, IME input, selection, Home/End, clipboard, long-value horizontal
    scrolling, focus visibility, and keyboard activation at supported DPI scales.

A nonempty vault reference or unresolved template is not evidence that a production
credential exists. Section 19 and execution-time resolution provide that validation.
