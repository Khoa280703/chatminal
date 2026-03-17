# Phase 1.2: Seal engine split path

**Context:** [plan.md](./plan.md) | Tier 1 Critical | Depends on Phase 1.1

## Overview

- **Priority:** P1
- **Status:** completed
- **Effort:** 30min
- **Description:** After Phase 1.1, `split_and_insert` has exactly 1 external caller: `domain.rs:140`. Reduce visibility to `pub(crate)`, deprecate the domain wrapper, add tracking for desktop engine-split fallback.

## Key Insights

- After Phase 1.1 deletes SSH/tmux callers, remaining callers:
  - `host-runtime/src/domain.rs:140` — `split_pane()` trait method, calls `tab.split_and_insert()`
  - `host-runtime/src/tab.rs:740` — the definition itself (+ internal `TabInner` at line 1960)
  - `host-runtime/src/tab.rs:2412,2456` — internal test usage within tab module
- `desktop_spawn.rs:107-109` already bails with message about session-native split; line 112-127 uses `split_terminal_handle_by_public_id` (session-native path). Engine split fallback no longer reachable from desktop, but add `log::warn!` for safety.

## Related Code Files

**Modify:**
- `crates/chatminal-host-runtime/src/tab.rs` (line 740)
- `crates/chatminal-host-runtime/src/domain.rs` (line 82-141, `split_pane` method)
- `apps/chatminal-desktop/src/desktop_spawn.rs` (line 111 area)

## Implementation Steps

1. **`tab.rs:740` — reduce visibility:**
   ```rust
   // Before:
   pub fn split_and_insert(
   // After:
   pub(crate) fn split_and_insert(
   ```

2. **`domain.rs` — deprecate `split_pane`:**
   - Add `#[deprecated(note = "Use session-native split; engine split retained for daemon compatibility")]` above the `split_pane` default method (around line 82)
   - Allow the deprecation warning in the module: `#[allow(deprecated)]` on the impl block or call site

3. **`desktop_spawn.rs` — add fallback tracking:**
   - Near line 111 (`let activity = ...`), before the `split_terminal_handle_by_public_id` call at line 114, add:
   ```rust
   log::warn!("engine split fallback triggered for pane_id={:?}", current_pane_id);
   ```
   - This helps track if the deprecated path is ever hit in production

## Todo List

- [x] Change `split_and_insert` to `pub(crate)` in tab.rs
- [x] Add `#[deprecated]` to `split_pane` in domain.rs
- [x] Add `log::warn!` in desktop_spawn.rs engine split path
- [x] Run verification

## Success Criteria

- `cargo check --workspace` passes (with expected deprecation warnings only)
- `split_and_insert` not callable from outside `chatminal-host-runtime` crate
- Deprecation warning visible when `split_pane` is called

## Risk Assessment

- **Very low risk:** Visibility reduction only; if something outside the crate calls it, compiler will catch it
- **Watch:** Ensure no other crate (e.g., `chatminal-lua-bridge`) calls `split_and_insert` directly

## Verification

```bash
cargo check --workspace 2>&1 | grep -i "deprecat"
# Ensure no external callers:
grep -rn "split_and_insert" --include="*.rs" . | grep -v "host-runtime/src/" | grep -v "third_party/"
cargo check --workspace
```
