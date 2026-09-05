# ADR 0013: OS vault foundation

Status: accepted for the storage/service slice; desktop workflows pending.

Secrets belong in OS secure storage, outside workspace files and SQLite. The new
`ps-vault` crate uses keyring 3.6.3 with explicit macOS Keychain, Windows native,
and Linux synchronous Secret Service features (Rust encryption for D-Bus).
There is no automatic plaintext or in-memory fallback. Linux requires D-Bus
 development libraries and an available, unlocked Secret Service at runtime.
See https://docs.rs/keyring/3.6.3/keyring/ for backend requirements.

Each stable workspace ID gets a separate service namespace. Each named credential
contains a versioned JSON record with its value, allowed domains, and tags. Values
and serialized buffers use zeroizing wrappers; errors omit backend details and
payloads. The portable record limit is 1200 UTF-16 code units to accommodate the
Windows credential blob limit. An oversized update is rejected before writing.
Create/update is an upsert. There is no credential enumeration/catalog yet.

Policies require HTTPS and exact normalized hosts. Empty policies deny execution;
wildcards, subdomains, URL userinfo, and HTTP do not grant access. Deliberate reveal
is a separate API. Domain policies are host based, so all HTTPS ports on an allowed
host are permitted. Secrets are opaque bytes to the variable template engine.

The service can produce a fresh resolver with only explicitly requested and
authorized secrets. Existing vault definitions are cleared. Callers must use this
resolver only for the authorized request and disable redirects or reauthorize
every redirect target. This API is not yet wired into HTTP sends, and should not
be treated as an end-to-end network enforcement boundary until that work lands.
The existing variable engine still copies strings; zeroization is limited to the
vault wrappers and does not guarantee removal of every execution-time copy.

AppState exposes explicit vault configuration using a stable workspace ID and
clears its vault when switching workspaces. Storage calls are synchronous and
must run on the background runtime. No credential access happens at startup.
The environment screen does not yet expose or automatically configure the vault.

Next: desktop lifecycle/reveal/reference picker; local metadata catalog/search;
clipboard clearing; environment secret persistence; execution and redirect wiring;
authenticated encrypted fallback and migration; OS runtime verification.
