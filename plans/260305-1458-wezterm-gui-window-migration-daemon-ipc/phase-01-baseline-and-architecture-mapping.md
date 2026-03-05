# Phase 01 - Baseline and Architecture Mapping

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/README.md](/home/khoa2807/working-sources/chatminal/README.md)
- [/home/khoa2807/working-sources/chatminal/docs/system-architecture.md](/home/khoa2807/working-sources/chatminal/docs/system-architecture.md)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_core.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_core.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-gui/src/main.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-gui/src/main.rs)

## Overview
- Priority: P1
- Status: Completed
- Effort: 3d
- Brief: freeze target architecture and migration constraints before touching runtime code.

## Key Insights
- Current window path is `egui TextEdit` + snapshot text, not native terminal surface.
- WezTerm already exists in `third_party/wezterm`; `wezterm-gui` + `window` stacks are available to reuse.
- Daemon-first invariant is strict in repo standards; cannot move session/runtime ownership client-side.

## Requirements
- Functional:
1. Finalize component mapping current -> target with file-level ownership.
2. Lock Linux/macOS acceptance criteria (fidelity + perf + stability).
3. Define incremental migration commands (`window-wezterm-gui` introduced before cutover).
- Non-functional:
1. No protocol break in this phase.
2. Keep plan incremental; no big-bang rewrite.

## Architecture
- Adopt `wezterm-gui` event/render loop for window runtime.
- Introduce adapter boundary `chatminal_ipc_mux_domain` for IPC -> GUI/mux integration.
- Keep `chatminald` unchanged as authority for session/profile/history/store.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/Cargo.toml`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs`
3. `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
4. `/home/khoa2807/working-sources/chatminal/docs/codebase-summary.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window-wezterm-gui/mod.rs`
2. `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/phase-01-architecture-map.md`
- Delete:
1. None

## Implementation Steps
1. Inventory all window/input/render paths in current `window/native_window_wezterm*` modules.
2. Inventory WezTerm GUI integration points needed (`frontend`, `termwindow`, `window::WindowEvent`).
3. Write contract doc for ownership boundaries: daemon, protocol, client gui adapter.
4. Define first-cut command and feature flag strategy.

## Todo List
- [x] Architecture map doc approved by maintainers.
- [x] Command naming and migration strategy locked.
- [x] Linux/macOS release criteria frozen.

## Success Criteria
- Team can start coding without architecture ambiguity.
- No unknown ownership overlap between daemon and gui runtime.

## Risk Assessment
- Risk: over-coupling to internal WezTerm modules too early.
- Mitigation: keep adapter boundary thin and explicit.

## Security Considerations
- Do not widen daemon IPC surface in this phase.
- Keep endpoint and permission model unchanged.

## Next Steps
- Start Phase 02 and scaffold IPC mux adapter.

## Unresolved Questions
1. Do we vendor selected wezterm-gui modules or depend directly from `third_party/wezterm` workspace crates?
