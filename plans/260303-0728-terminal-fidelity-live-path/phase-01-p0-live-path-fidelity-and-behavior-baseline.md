# Phase 01 - P0 Live Path Fidelity and Behavior Baseline

## Context Links
- Plan: [plan.md](plan.md)
- Next: [phase-02-p1-compatibility-checklist-and-regression-gates.md](phase-02-p1-compatibility-checklist-and-regression-gates.md)
- Runtime docs: `/home/khoa2807/working-sources/chatminal/README.md`

## Overview
- Priority: P1
- Status: pending
- Effort: 10h
- Goal: Remove non-terminal-like behavior on active running sessions; keep stream path stable.

## Key Insights
- `frontend/src/App.svelte` resets terminal in multiple hydrate paths.
- `get_session_snapshot` is used even when active session is already running/live.
- `tryHandleLocalSlashCommand` inspects command content (`clear`) and mutates history state.
- Current reconnect flow is good baseline but still snapshot-heavy for live sessions.

## Requirements
- Do not reset/hydrate terminal when active running session can continue live stream.
- Snapshot path should be fallback, not primary, when runtime is running.
- Command-level interception must be removed or optional with explicit opt-in.
- No breaking change for command contracts already used by frontend.

## Architecture
- Introduce frontend render strategy:
  - `live` for running active session.
  - `snapshot` only for disconnected session or hard resync.
- Keep `seq` monotonic guard for dedupe; do not call `terminal.reset()` unless hard resync.
- Add terminal behavior config for interception:
  - `local_command_interception` (`false` default).
  - Frontend branches on this flag before calling local command parser.

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`
- Modify `/home/khoa2807/working-sources/chatminal/frontend/src/lib/types.ts`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/config.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/models.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/main.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`
- Modify `/home/khoa2807/working-sources/chatminal/README.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/code-standards.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Implementation Steps
1. Add terminal behavior setting in backend config model (`UserSettings`) with default `false`.
2. Add command to return terminal behavior settings to frontend at startup.
3. Refactor hydrate flow in `App.svelte`:
   - Skip snapshot fetch when active session is running and rendered `seq` is current.
   - Avoid unconditional `terminal.reset()` in active-live path.
4. Keep snapshot path only for disconnected sessions and hard-miss states.
5. Gate `tryHandleLocalSlashCommand` behind explicit opt-in config; default off.
6. Ensure `clear` behavior comes from shell/TUI, not frontend command parsing.
7. Update docs for behavior and migration notes.

## Todo List
- [ ] Add backend setting and command contract for terminal behavior.
- [ ] Refactor active-session hydrate/reset logic to live-first flow.
- [ ] Make command interception opt-in and default disabled.
- [ ] Verify no regression for reconnect from disconnected sessions.
- [ ] Sync docs and changelog.

## Success Criteria
- Typing in active running session does not trigger visible reset/flicker.
- Switching back to an already running session does not force snapshot rehydrate.
- `clear` command behaves like normal shell command when default config used.
- Build and tests pass:
  - `cargo test --manifest-path src-tauri/Cargo.toml`
  - `npm --prefix frontend run build`

## Risk Assessment
- Risk: stale render if hydrate skip condition too aggressive.
- Mitigation: hard-resync path with explicit condition (`seq mismatch` or missing rendered session).
- Risk: behavior drift across reconnect path.
- Mitigation: keep disconnected snapshot flow unchanged in this phase.

## Security Considerations
- No privilege or auth change in this phase.
- Preserve existing shell validation and input bounds; do not loosen guards.

## Next Steps
- Feed this phase output into P1 compatibility regression checklist.

## Unresolved Questions
1. Should opt-in setting be in `settings.json` only, or also exposed in UI toggle in same phase?
