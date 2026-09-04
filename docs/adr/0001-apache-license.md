# ADR-0001: Selection of Apache-2.0 License and GPL Boundary Policy

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

PacketSmith is an open-source, local-first API development platform built on Zed's GPUI framework. Zed's application and editor code is predominantly licensed under GPL-3.0-or-later, while GPUI itself is released under Apache-2.0. We need a clear, permissive licensing strategy for PacketSmith that fosters widespread developer and enterprise adoption while avoiding GPL obligations for users and downstream contributors.

## Considered Options

1. **GPL-3.0-or-later:** Mirror Zed's top-level license.
2. **MIT License:** Standard permissive license.
3. **Apache License 2.0:** Standard permissive license with explicit patent and trademark grants, directly compatible with GPUI.

## Decision Outcome

Chosen option: **Apache License 2.0** with a strict clean-room policy regarding Zed GPL code.

### Positive Consequences
- Downstream users, teams, and enterprises can freely adopt, extend, and integrate PacketSmith into their environments without copyleft viral obligations.
- Perfectly aligns with GPUI's Apache-2.0 licensing.
- Explicit patent grant protections.

### Negative Consequences
- Developers cannot copy or vendor code from Zed's GPL editor crates. All editor widgets, UI primitives, and features must be authored independently against the Apache-2.0 GPUI public API.
