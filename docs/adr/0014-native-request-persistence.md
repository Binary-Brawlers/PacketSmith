# ADR 0014: Native request persistence

Status: accepted; native Linux workflow verified, macOS/Windows verification pending.

The native workbench needs to save the complete editable request and reopen it
without losing fields. `HttpRequestPayload` now owns the shared HTTP header,
parameter, and body models in `ps-domain`; `ps-http` re-exports those types to
retain its existing API. New fields have serde defaults and omit empty values,
so existing method/URL-only documents remain compatible with schema 1.0.0.

The editor retains the original `RequestDocument`, workspace path, and a saved
snapshot of editable fields. Content comparison determines dirty state. Updating
a saved request overlays HTTP fields and auth onto the original document,
preserving identity, location, creation time, scripts, settings, tags, description,
and examples. Unchanged auth controls retain the original secret-reference form.
Repeated opens select the existing tab. Unsupported body formats are rejected
on open instead of being silently replaced by the editor.

New requests are named and placed at a collection root; an empty workspace offers
creation of a Requests collection. All save I/O runs on Tokio's blocking pool.
The worker rescans before writing, and existing requests must match the full
document originally opened. A conflict or failure retains the editor contents.
Existing files are replaced from a flushed sibling temporary file. This avoids
truncating the original on a failed write; the conflict check is not a filesystem
lock against another process writing during the check/replace interval.

Raw session auth is permitted for sends, but newly authored persisted credentials
require complete variable references. Known sensitive and explicitly secret
headers receive the same protection, including disabled rows. Shared preparation
resolves credentials before Basic encoding and preserves opaque vault values.
The protocol executor and desktop sender use the same header/body preparation;
persisted query tables are authoritative and never appended twice to their URL.

Dirty tab closing offers Save, Discard, or Cancel. Save closes only if the completed
save still matches the tab's current contents. Unsaved drafts are not restored on
application restart. Persistent history, crash recovery, nested-folder destination
selection, and vault UI/execution enforcement remain separate work.
