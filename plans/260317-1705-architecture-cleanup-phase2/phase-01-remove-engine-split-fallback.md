# Phase 1: Remove Engine Split Fallback

## Overview
- **Priority**: P1 (blocks Phase 2)
- **Status**: pending
- **Effort**: 30min

The engine split fallback in `desktop_spawn.rs:111-131` still calls `split_terminal_handle_by_public_id` when `session_id = None`. Phase 1 of the previous plan added `log::warn!` + deprecated `split_pane` but left the fallback code alive. Replace with `anyhow::bail!`.

## Key Insights
- Line 94: `if let Some(_session_id)` check — when this is `None`, we fall through to the engine split path
- This fallback should never fire in normal Desktop use (session always exists)
- `split_terminal_handle_by_public_id` defined in `desktop_host_runtime/mod.rs:880`
- After removing the caller, the function + `SpawnTarget::split_pane` become dead code

## Related Code Files

### Modify
- `apps/chatminal-desktop/src/desktop_spawn.rs` — replace lines 111-131 with `anyhow::bail!`
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` — delete `split_terminal_handle_by_public_id` fn

### Evaluate for removal
- `crates/chatminal-host-runtime/src/spawn_target.rs` — `SpawnTarget::split_pane` trait method (deprecated)
  - Check if daemon still calls it; if not, delete
- `crates/chatminal-host-runtime/src/lib.rs` — re-export of `split_pane` related types

## Implementation Steps

1. In `desktop_spawn.rs`, replace the `log::warn!` block (lines 111-131) with:
   ```rust
   anyhow::bail!(
       "engine split fallback reached — session_id is None; this is a bug"
   );
   ```
   Remove the `activity`, `pane_id` check, and `split_terminal_handle_by_public_id` call.

2. Delete `split_terminal_handle_by_public_id` from `desktop_host_runtime/mod.rs`.

3. Check if `SpawnTarget::split_pane` has any remaining callers outside `spawn_target.rs:140` (the trait default impl):
   - If no external callers: delete the default impl body, keep deprecated empty trait method for daemon compat
   - `crates/chatminal-lua-bridge/src/leaf.rs` — check for `split_pane` usage

4. Remove unused imports after deletions.

5. Run `cargo check --workspace` to verify compilation.

## Success Criteria
- No code path calls `split_terminal_handle_by_public_id`
- `cargo check --workspace` passes
- Engine split is fully sealed (only deprecated trait stub remains if daemon needs it)

## Risk Assessment
- **Low risk**: This path was already logging a warning and is unreachable in normal Desktop flow
- If daemon calls `split_pane` we must keep the trait method stub
