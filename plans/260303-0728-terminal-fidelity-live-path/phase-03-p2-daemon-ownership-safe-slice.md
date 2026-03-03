# Phase 03 - P2 Daemon Ownership Safe Slice

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-02-p1-compatibility-checklist-and-regression-gates.md](phase-02-p1-compatibility-checklist-and-regression-gates.md)
- Runtime backend staging: `/home/khoa2807/working-sources/chatminal/src-tauri/src/runtime_backend.rs`

## Overview
- Priority: P3
- Status: pending
- Effort: 6h
- Goal: Ship smallest safe daemon ownership slice without breaking current in-process runtime.

## Key Insights
- Current daemon mode is staging only (`info`/`ping`), no PTY ownership transfer yet.
- Full cutover in one step is high-risk for session lifecycle and live IO reliability.
- Need explicit fallback to in-process when daemon unavailable.

## Requirements
- Zero runtime break when daemon is unreachable.
- Explicit runtime owner visibility in contracts for debugging.
- Keep in-process path as stable default.
- Any daemon pilot must be opt-in and reversible.

## Architecture
- Add runtime ownership decision at spawn boundary:
  - `daemon` requested + reachable + capability OK => daemon pilot path (limited scope).
  - else fallback to in-process automatically.
- Add `runtime_owner` field to session/runtime info for observability.
- Keep `write_input`, `resize_session`, `activate_session` behavior identical from frontend view.

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/runtime_backend.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/models.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/main.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/lib/types.ts`
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`
- Modify `/home/khoa2807/working-sources/chatminal/README.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Implementation Steps
1. Add backend decision helper that returns effective owner (`in_process` or `daemon`).
2. For this slice, daemon path can remain limited/stubbed but must fail closed to in-process.
3. Expose effective owner in session/runtime response models for debugging and support.
4. Update frontend status surface to show owner only in diagnostics text, no behavior fork yet.
5. Validate fallback behavior with `CHATMINAL_RUNTIME_BACKEND=daemon` when daemon endpoint missing/unreachable.

## Todo List
- [ ] Add runtime owner model field and wire through list/load responses.
- [ ] Implement safe owner decision and fallback in service spawn path.
- [ ] Keep existing live IO path untouched for fallback case.
- [ ] Document pilot constraints and rollback instructions.

## Success Criteria
- With daemon requested but unavailable, sessions still spawn and run in-process.
- No regression in `activate_session`, `write_input`, `resize_session`.
- Runtime owner is visible for support/debugging.
- P1 compatibility checklist still green under fallback scenario.

## Risk Assessment
- Risk: ownership metadata drift from actual runtime state.
- Mitigation: owner value set only at spawn/attach boundary from single decision function.
- Risk: hidden partial daemon path causing unstable behavior.
- Mitigation: guard pilot path behind explicit capability check and fast fallback.

## Security Considerations
- Daemon endpoint input remains local IPC only; keep current validation boundaries.
- Never route PTY ownership to daemon when health/capability checks fail.

## Next Steps
- After this safe slice, define full ownership protocol as separate plan (PTY create/attach/io/resize lifecycle).

## Unresolved Questions
1. Daemon pilot first scope: new sessions only, or reconnected sessions too?
