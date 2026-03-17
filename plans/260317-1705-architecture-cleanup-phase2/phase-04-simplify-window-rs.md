# Phase 4: Simplify window.rs (Optional)

## Overview
- **Priority**: P3 (optional)
- **Status**: pending
- **Effort**: 1h
- **Blocked by**: Phase 2

`Window` (268 lines) in `crates/chatminal-host-runtime/src/window.rs` wraps `Vec<Tab>` but Desktop only ever uses 1 Window + 1 Tab.

## Key Insights

### Current Usage
- Desktop creates exactly 1 Window with 1 Tab via Mux
- Window methods used: `push`, `get_active`, `iter`, `prune_dead_tabs`, `window_id`, `get_workspace`, `set_workspace`
- `remove_by_idx`, `remove_by_id`, `insert` — used by tab management but Desktop has 1 tab
- Multi-tab methods (`save_and_then_set_active`, `last_active`) — effectively unused in Desktop

### Why Optional
- Window is a rendering dependency — Mux exposes `get_window()` and rendering iterates windows
- Removing multi-tab support could break daemon compatibility
- 268 lines is not a significant code smell
- Cost/benefit ratio is low

### If we do it
- Mark multi-tab methods with `#[deprecated]`
- Add doc comments clarifying "Desktop uses single Window/Tab; multi-tab retained for daemon compat"
- DO NOT delete methods — daemon may need them

## Implementation Steps

1. Add doc comment block at top of `window.rs` explaining the single-Window/single-Tab model
2. Mark unused-in-Desktop methods with `#[deprecated(note = "Desktop uses single tab")]`
3. Optionally add `debug_assert!(self.tabs.len() <= 1)` in Desktop-only paths (not in lib)
4. Run `cargo check --workspace`

## Success Criteria
- Better documentation of actual usage pattern
- No behavior changes
- Compilation still passes

## Risk Assessment
- **Very low**: This is documentation + deprecation markers only
