# ADR 0012: Native environment management

Status: Accepted (core and native UI implemented; runtime validation pending)

## Context

Environment management follows the existing variable resolver and workspace state.
The desktop entry point renders an environment workspace backed by the same
workspace state and resolver used by other shell controllers.

## Decision

`ps-workspace::EnvironmentManager` owns native YAML environment lifecycle operations.
New files use stable UUID names in `environments/`; rename preserves the ID and path.
Imports and duplicates get fresh IDs. Duplicate IDs on scan and duplicate variable
keys on edits are errors. Updates use a sibling temporary file followed by rename.

`VariableEntry.value` remains the shared default for backward compatibility. Type
and description are optional on deserialization. An empty value represents an
unconfigured variable; template-containing values are validated after resolution
by their consumer, rather than being parsed as literal numbers or JSON here.

Secret current values exist only in session memory. Non-secret current values and
active selection persist in `.packetsmith/environments.local.json`, a versioned,
Git-ignored local snapshot written through a sibling temporary file. On Unix new
snapshot files have owner-only permissions. Explicit empty overrides are retained.
Malformed or unsupported snapshots return sanitized errors without mutating
current state. Missing resources, removed keys, changed types, and variables now
classified as secret are ignored when restoring. They override defaults, including an
explicit empty override, and retain the variable's secret classification. Removing
an override restores the default. Changing type or secrecy discards its override.
Duplicates and exports exclude current values. Raw secret defaults are stripped
on save/import/export; vault references are retained when explicitly typed as
`secret_reference`. Existing external YAML is not rewritten merely by scanning.
Users should use current overrides for raw credentials until vault storage exists.

Table models mask secret/default/current values. Diff compares display-safe rows,
so it deliberately does not report whether two masked secret contents differ.
Missing-value detection reports enabled keys with empty effective values. The UI
also accepts an explicit reference environment: its enabled keys are required in
the target, so absent and disabled target keys are reported. Results contain key
names only. Vault availability and unresolved templates are separate checks.

Workspace selection replaces only the environment resolver scope, preserving other
scopes and dynamic providers. Workspace save/current/delete methods refresh the
resolver. Callers that mutate the manager directly must refresh before execution.
Workspace scans restore selection and non-secret overrides on initial load. Later
rescans retain compatible session values, including secrets. Failed scans preserve
the previous manager. Selection/current edits persist before applying in-memory
changes; resource saves/deletes refresh the resolver before persisting the local
snapshot. If that latter write fails, the resource edit is already committed and
the error is returned. Secret persistence awaits secure vault storage.

## Native editor

The screen provides an ordered environment selector, active-environment header,
variable table, named creation/rename, duplicate/clone, confirmed environment
deletion, and a separate variable editor. Tab/Shift+Tab traverse controls;
Enter/Space activate buttons. Cmd+Shift+E (macOS) or Ctrl+Shift+E cycles through
name/ID ordered environments and No environment. Open variable drafts must be
saved or explicitly cancelled before switching or loading another workspace.

Shared variable fields and current overrides have separate commit controls.
Applying a current override requires the persisted key/type/secret classification
to match the editor. This prevents a credential typed into a newly secret draft
from being persisted under the old non-secret classification. Inputs mask secret
content, block copying/cutting secrets, and never preload raw existing secrets.
Blank secret-reference defaults preserve the existing reference; Clear default
explicitly removes it. Raw credentials use session current values.

Import reads a native YAML path and sanitizes parse errors before displaying them.
Export writes shared defaults only, using create-new semantics so an existing file
is never silently overwritten. Duplicate and clone use the same fresh-ID policy;
local overrides are not copied. The workspace path field can open another existing
workspace directory. The input widget is adapted from the Apache-2.0 GPUI example;
see `docs/third-party-notices.md` for attribution.

## Validation

Regression targets cover lifecycle, stable rename, fresh import/clone identities,
resolver refresh, persistence, masked display and diff, production reference keys,
secret file/export exclusion, unsaved secret classification, and session-preserving
initial scans/reloads. Both core and GPUI code are type-checked with:

`cargo check --workspace --all-targets --features packetsmith-app/gpui-ui`

Tests and the application were not executed under the user's type-check-only
instruction. Native interaction and cross-platform validation remain pending;
`docs/testing/environments.md` records the verification scenarios. Section 18
checkboxes remain open for those gates, with implementation status on each item.
