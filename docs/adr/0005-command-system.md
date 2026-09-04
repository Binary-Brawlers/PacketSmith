# ADR-0005: Decoupled Command Registry and Keybinding Dispatch Engine

- **Status:** Accepted
- **Date:** 2026-09-04
- **Authors:** PacketSmith Architecture Team

---

## Context and Problem Statement

PacketSmith requires a keyboard-first ergonomics model with a Command Palette, configurable keybindings, application menu integration, and programmatic command execution. If commands are tightly bound to GPUI view elements, actions cannot be invoked from headless CLI scripts, automated macros, or external IPC.

## Considered Options

1. **GPUI Native Actions Only:** Use GPUI's built-in action macros directly in views. Coupes all commands to the desktop GUI crate.
2. **Decoupled Command Registry (`ps-command`):** Abstract command identifiers, descriptors, keybinding metadata, and action dispatchers into a dedicated crate that GPUI views and the CLI can both bind to.

## Decision Outcome

Chosen option: **Decoupled Command Registry (`ps-command`)**.

### Positive Consequences
- Commands are first-class data structures searchable by title, category, and ID in the Command Palette.
- Cross-platform keybinding abstractions (Cmd on macOS vs Ctrl on Windows/Linux) work out of the box.
- Headless testing of command workflows without spinning up GPU contexts.

### Negative Consequences
- Slightly more boilerplate to register actions between GPUI view callbacks and the command registry.
