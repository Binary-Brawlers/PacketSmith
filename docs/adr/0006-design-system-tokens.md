# ADR-0006: Design System Scales and Token Abstraction

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

A native desktop application requires visual consistency across spacing, typography, border radiuses, elevation, and colors. Hardcoding pixel sizes or RGB values across components creates visual inconsistencies, makes theme switching brittle, and breaks accessibility guarantees.

## Considered Options

1. **Ad-hoc Tailwind-like utilities in GPUI code:** Fast to prototype, but difficult to maintain consistent token math and WCAG AA contrast compliance across light/dark modes.
2. **Centralized Design Token Layer (`ps-ui-components`):** Define standard scale tokens (Spacing, Radius, Typography, Elevation, BorderTokens), semantic theme palettes (ThemePalette), and HTTP method badge colors in a dedicated crate.

## Decision Outcome

Chosen option: **Centralized Design Token Layer (`ps-ui-components`)**.

### Positive Consequences
- Guarantees strict visual consistency across the entire workbench (tabs, split panes, inputs, tree views).
- Programmatic WCAG contrast ratio calculations ensure accessibility standards are met.
- Clean separation between token logic and GPUI rendering elements.

### Negative Consequences
- Changes to token names require updates across UI component callers.
