# ADR-0008: Variable Scoping Precedence and Secret Masking

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

Requests need dynamic substitution of hostnames, auth tokens, path parameters, and query arguments. Variables can be defined at global, environment, collection, folder, or request levels. We need deterministic precedence rules and strict secret redaction to prevent credential leakage.

## Considered Options

1. **Flat global dictionary:** Simple, but prevents multi-environment switching and collection-level variable reuse.
2. **Hierarchical Scoped Precedence with Vault Masking:** Variables resolve from narrowest to widest scope (Request > Folder > Collection > Environment > Global), and sensitive variables/headers are automatically redacted in logs and event sinks.

## Decision Outcome

Chosen option: **Hierarchical Scoped Precedence with Vault Masking**.

### Positive Consequences
- Dynamic variables (`{{$guid}}`, `{{$timestamp}}`, `{{$randomInt}}`) provide automated test ergonomics.
- Clear scoping guarantees predictable variable resolution.
- Headers containing authorization or API keys are masked before passing through `EventSink` to prevent credential exposure in the UI or CLI logs.

### Negative Consequences
- Variable interpolation incurs a lightweight template scanning pass over URLs and headers before dispatch.
