# ADR 0012: Native environment management

Status: Accepted (core implemented; desktop rendering and runtime validation pending)

## Context

Environment management follows the existing variable resolver and workspace state.
The desktop currently renders a placeholder; controller APIs must not be mistaken
for a rendered environment editor.

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
Missing-value detection reports enabled keys with empty effective values; it does
not validate vault availability or infer production requirements from other files.

Workspace selection replaces only the environment resolver scope, preserving other
scopes and dynamic providers. Workspace save/current/delete methods refresh the
resolver. Callers that mutate the manager directly must refresh before execution.
Workspace scans restore selection and non-secret overrides on initial load. Later
rescans retain compatible session values, including secrets. Failed scans preserve
the previous manager. Selection/current edits persist before applying in-memory
changes; resource saves/deletes refresh the resolver before persisting the local
snapshot. If that latter write fails, the resource edit is already committed and
the error is returned. Secret persistence awaits secure vault storage.

## Validation

Regression tests cover persistence, fresh import identities, duplication, deletion,
validation, export/file secret exclusion, masked display, and scope switching.
Only `cargo check --workspace --all-targets` was run, per the user's instruction.
Runtime tests, native UI wiring, and verification on all supported OSes remain open.
