# Phase 2.2: Document terminal parser split

**Context:** [plan.md](./plan.md) | Tier 2 Medium | Independent, anytime

## Overview

- **Priority:** P2
- **Status:** completed
- **Effort:** 15min
- **Description:** No code change. Add doc comments to clarify why two terminal parser crates exist and their ownership boundaries.

## Results

- Doc comments added to chatminal-terminal-core/src/lib.rs clarifying lightweight core types used by daemon
- Doc comments added to chatminal-engine-term/src/lib.rs clarifying full termwiz-based emulator for desktop
- cargo doc passes with clean docs

## Key Insights

- `chatminal-terminal-core` (210 lines) — lightweight vt100-style types: `TerminalSize`, `CursorPosition`, `ScreenLine`, `TerminalConfiguration` trait. Used by daemon (`chatminal-app`).
- `chatminal-engine-term` (2005 lines) — full termwiz-based terminal emulator. Used by desktop (`chatminal-desktop`).
- Both are needed: daemon uses core types only; desktop needs full rendering.
- No duplication — core provides shared types, engine-term provides implementation.

## Related Code Files

**Modify (doc comments only):**
- `crates/chatminal-terminal-core/src/lib.rs` (line 1 area)
- `crates/chatminal-engine-term/src/lib.rs` (line 1 area)

## Implementation Steps

1. **Add module-level doc comment to `chatminal-terminal-core/src/lib.rs`:**
   ```rust
   //! Lightweight terminal type definitions shared by daemon and desktop.
   //!
   //! This crate provides `TerminalSize`, `CursorPosition`, `ScreenLine`, and
   //! the `TerminalConfiguration` trait. It has zero heavy dependencies (no termwiz).
   //!
   //! Counterpart: `chatminal-engine-term` provides the full termwiz-based terminal
   //! emulator used by the desktop GUI. Daemon code should depend on this crate only.
   ```

2. **Add module-level doc comment to `chatminal-engine-term/src/lib.rs`:**
   ```rust
   //! Full terminal emulator for the desktop GUI (termwiz-based).
   //!
   //! This crate wraps termwiz to provide terminal rendering, input handling,
   //! and escape sequence processing for the desktop app.
   //!
   //! Counterpart: `chatminal-terminal-core` provides lightweight shared types
   //! used by the daemon. Desktop code uses both; daemon uses only terminal-core.
   ```

## Todo List

- [x] Add doc comment to chatminal-terminal-core/src/lib.rs
- [x] Add doc comment to chatminal-engine-term/src/lib.rs
- [x] Run verification

## Success Criteria

- Both crates have clear doc comments explaining their roles
- `cargo doc -p chatminal-terminal-core -p chatminal-engine-term` generates clean docs
- No code changes beyond comments

## Risk Assessment

- **Zero risk:** Doc comments only

## Verification

```bash
cargo doc -p chatminal-terminal-core -p chatminal-engine-term --no-deps 2>&1 | grep -i error
```
