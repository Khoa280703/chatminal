# Phase 3.1: Clean ghost crate references

**Context:** [plan.md](./plan.md) | Tier 3 Low | Independent, anytime

## Overview

- **Priority:** P3
- **Status:** completed
- **Effort:** 15min
- **Description:** Replace 12 stale `chatminal-session-runtime` references in comments across 8 .rs files. The crate was deleted (inlined into `desktop_host_runtime/session_engine/`); comments still reference it.

## Key Insights

- `chatminal-session-runtime` was inlined during Phase 08 of a previous refactor
- All execution engine code now lives in `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/`
- Comments reference the deleted crate name, confusing for new contributors

## Related Code Files

**Modify (comment-only):**

| File | Line | Current Reference |
|------|------|-------------------|
| `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs` | 42 | `chatminal-session-runtime` |
| `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/mod.rs` | 1-2 | `chatminal-session-runtime (Phase 08)` |
| `crates/chatminal-runtime/src/state.rs` | 73 | `chatminal-session-runtime` |
| `crates/chatminal-runtime/src/state/test_bridge.rs` | 2 | `chatminal-session-runtime` |
| `crates/chatminal-runtime/src/state/runtime_bridge.rs` | 2, 5, 7 | `chatminal-session-runtime` (3 refs) |
| `crates/chatminal-runtime/src/lib.rs` | 6 | `chatminal-session-runtime` |
| `crates/chatminal-runtime/src/workspace_layout.rs` | 1, 5 | `chatminal-session-runtime` (2 refs) |
| `crates/chatminal-runtime/src/workspace_ids.rs` | 6 | `chatminal-session-runtime` |

## Implementation Steps

1. **For each file**, replace `chatminal-session-runtime` with the correct current module path:
   - In `desktop_host_runtime/` files: use `desktop_host_runtime::session_engine`
   - In `chatminal-runtime/` files: use `desktop_host_runtime::session_engine` (external crate reference)

2. **Specific replacements:**
   - `execution_bridge.rs:42`: `chatminal-session-runtime` -> `desktop_host_runtime::session_engine`
   - `session_engine/mod.rs:1-2`: Update to say "Session execution engine (inlined from former chatminal-session-runtime crate)"
   - `runtime_bridge.rs:2,5,7`: Replace with `desktop_host_runtime::session_engine`
   - `lib.rs:6`: Replace with `desktop_host_runtime::session_engine`
   - `state.rs:73`: Replace reference
   - `test_bridge.rs:2`: Replace reference
   - `workspace_layout.rs:1,5`: Replace references
   - `workspace_ids.rs:6`: Replace reference

## Todo List

- [x] Update execution_bridge.rs comment
- [x] Update session_engine/mod.rs comments
- [x] Update runtime_bridge.rs comments (3 refs)
- [x] Update lib.rs comment
- [x] Update state.rs comment
- [x] Update test_bridge.rs comment
- [x] Update workspace_layout.rs comments (2 refs)
- [x] Update workspace_ids.rs comment
- [x] Run verification

## Success Criteria

- Zero occurrences of `chatminal-session-runtime` in `.rs` files
- All replacement comments are accurate

## Risk Assessment

- **Zero risk:** Comment-only changes

## Verification

```bash
grep -rn "chatminal-session-runtime" --include="*.rs" .
# Should return 0 results
cargo check --workspace
```
