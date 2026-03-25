# Phase 2: Dead Code Removal in tab.rs

## Overview
- **Priority**: P2
- **Status**: pending
- **Effort**: 1.5h
- **Blocked by**: Phase 1

tab.rs is ~2528 lines. After Phase 1 removes the engine split path, several functions become dead code. Must carefully distinguish rendering-required code from split-mutation code.

## Key Insights

### CAN remove (split mutation — no longer called from Desktop after Phase 1)
- `split_and_insert` (pub(crate), line 740) — only caller was `spawn_target.rs:140` (deprecated `split_pane`)
- `compute_split_size` (pub, line 728) — only caller was `spawn_target.rs:96` + `split_and_insert` internal
- Inner impl counterparts: `TabInner::split_and_insert` (line 1960), `TabInner::compute_split_size` (line 1869)
- Related types: `SplitDirectionAndSize`, `SplitRequest` — check if rendering still uses them
- Tests that exercise split_and_insert (lines ~2340-2502)

### CAN remove if no rendering dependency
- `resize_split_by` (pub, line 639) — called from `desktop_host_runtime/mod.rs:335`
  - **WAIT**: This IS still used by desktop rendering. KEEP it.

### MUST keep (rendering layer depends on these)
- `iter_panes`, `iter_panes_ignoring_zoom` — used extensively by rendering
- `get_active_pane`, `set_active_idx` — used by focus management
- Pane positioning/size calculations — rendering layout
- `resize_split_by` — still called from desktop overlay compat

## Related Code Files

### Modify
- `crates/chatminal-host-runtime/src/tab.rs`:
  - Delete `split_and_insert` (pub + inner)
  - Delete `compute_split_size` (pub + inner)
  - Delete related helper functions used only by these
  - Delete tests that only test removed functions
- `crates/chatminal-host-runtime/src/spawn_target.rs`:
  - If Phase 1 removed `split_pane` body: remove `SplitSource` enum, clean imports

### Do NOT modify
- Anything used by `iter_panes`, `get_active_pane`, `resize_split_by`, positioning

## Implementation Steps

1. After Phase 1 completes, grep for all callers of `split_and_insert` and `compute_split_size`
2. Confirm they are only in deprecated/deleted code + tests
3. Delete `Tab::split_and_insert`, `Tab::compute_split_size` and inner impls
4. Delete `SplitDirectionAndSize` struct if no other callers
5. Delete tests that only exercise removed code
6. Clean up `SplitRequest` — keep if `resize_split_by` or rendering uses it; delete otherwise
7. Remove unused imports
8. Run `cargo check --workspace`
9. Estimate lines removed (target: 500-800 lines)

## Success Criteria
- tab.rs reduced by ~500+ lines
- `cargo check --workspace` passes
- All existing tests pass (only removed tests were for deleted functions)
- `resize_split_by` still works for desktop overlay rendering

## Risk Assessment
- **Medium risk**: Need to carefully trace what rendering uses vs what is pure split-mutation
- Binary tree data structure (`bintree.rs`) is shared between rendering layout and split — DO NOT delete it
- `SplitRequest` type used in `resize_split_by` chain — verify before removing
