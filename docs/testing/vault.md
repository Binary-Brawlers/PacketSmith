# Vault verification

Type-check service, regression targets, and desktop integration:

```sh
cargo check --workspace --all-targets --features packetsmith-app/gpui-ui
```

This check passed on the development macOS host. Tests were compiled but not run,
per the instruction to use type checks only. No build or dev server was run.

Regression cases in `crates/ps-vault/src/lib.rs` cover create/update/reveal/delete,
masked Debug output, resolver masking, HTTPS exact-host restrictions, empty-policy
denial, host spoofing, malformed records, literal credential interpolation, and
removal of stale vault scope. The memory store is test-only; these cases never
access real OS credentials.

Before completing Section 19, run those tests and independently verify real
create/read/update/delete, locked-store errors, workspace isolation, and restart
persistence on macOS, Windows, and Linux. Verify plaintext absence from workspace,
history, logs, crash reports, and exports after the execution/UI integrations land.
Also test domain changes on redirects. Encrypted migration tests await the fallback.
