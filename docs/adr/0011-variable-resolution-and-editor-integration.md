# ADR-0011: Provenance-Aware Variable Resolution and Editor Integration

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

Template variables appear in URLs, headers, bodies, scripts, and future protocol payloads. A flat string replacement helper cannot safely represent scope precedence, recursion failures, nested object lookup, dynamic values, or secret provenance. The desktop editor also needs the same interpretation as request execution for highlighting, hover previews, navigation, and usage search.

## Decision Outcome

Variable behavior lives in the protocol-independent `ps-variable` crate. It tokenizes references before resolution and returns both an execution value and a display-safe value. Every successful substitution carries its definition scope and source; unresolved, invalid, recursive, and over-depth references remain visible with structured diagnostics.

Scope precedence is:

1. temporary runtime values,
2. iteration data,
3. request,
4. folder,
5. collection,
6. environment,
7. workspace/global.

Vault references use the explicit `{{vault:name}}` namespace. Dynamic references use `$` names and a provider trait so future extensions can add generators without changing the parser.

The application shell owns a display-only controller for token state, hover content, jump targets, create-from-unresolved, and cross-resource usage results. It never exposes raw secret values. Protocol executors receive the same resolver through `ExecutionContext`; event sinks receive only the redacted display representation.

### Consequences

- Desktop, CLI, and every protocol adapter can share deterministic resolution semantics.
- Nested templates and JSON object paths work without recursive-loop risk.
- Secret taint propagates through nested values and remains masked in UI and execution events.
- Editors can provide rich variable actions without depending on networking or storage internals.
