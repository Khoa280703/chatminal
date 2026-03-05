# Phase 04 - Session/Profile/History Compatibility and Rollout Guard

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/crates/chatminal-protocol/src/lib.rs](/home/khoa2807/working-sources/chatminal/crates/chatminal-protocol/src/lib.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/request_handler.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/request_handler.rs)
- [/home/khoa2807/working-sources/chatminal/docs/code-standards.md](/home/khoa2807/working-sources/chatminal/docs/code-standards.md)

## Overview
- Priority: P1
- Status: Completed
- Effort: 4d
- Brief: guarantee IPC/domain compatibility so session/profile/history stays stable through backend swap.

## Key Insights
- Existing protocol is already sufficient for lifecycle and history operations.
- Breaking risks are mostly semantic timing/state mismatches, not missing fields.
- Current code standard explicitly enforces daemon-first lifecycle ownership.

## Requirements
- Functional:
1. Maintain old IPC request/response/event behavior for all existing commands.
2. Add backend switch control for staged rollout (`egui` vs `wezterm-gui`).
3. Ensure workspace/profile/session state remains consistent across backend changes.
- Non-functional:
1. Zero data migration in SQLite schema for this migration wave.
2. Backward-compatible protocol evolution only (additive if needed).

## Architecture
- Compatibility layer in app: backend selector + capability check.
- Version guard: handshake via `Ping`/known request probe; fallback to old window backend on mismatch.
- Optional additive fields only, default-safe for old daemon versions.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs`
3. `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
4. `/home/khoa2807/working-sources/chatminal/docs/code-standards.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/ipc-compatibility-matrix.md`
2. `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh`
- Delete:
1. None

## Implementation Steps
1. Define IPC compatibility matrix by request/event type.
2. Implement runtime backend selector env var and command fallback logic.
3. Add compatibility smoke script for session/profile/history operations on both backends.
4. Document rollback procedure.

## Todo List
- [x] Compatibility matrix complete and reviewed.
- [x] Kill-switch script validates backend fallback.
- [x] Session/profile/history invariants verified on both backends. (verified via `phase06-killswitch-verify` + `phase08-wezterm-gui-killswitch-verify` with strict legacy headless path)

## Success Criteria
- No regression in `workspace`, `sessions`, `create`, `activate`, `snapshot`, `history clear` flows.
- Backend swap can be rolled back in one config/env change.

## Risk Assessment
- Risk: hidden dependency on old egui-specific state behavior.
- Mitigation: dual-backend run period with parity checks.

## Security Considerations
- Preserve existing IPC endpoint permission model.
- Keep request size/timeout guards unchanged.

## Next Steps
- Execute fidelity/perf gates (Phase 05).

## Unresolved Questions
1. None.
