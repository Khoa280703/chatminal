# Audit remaining Mux/HostMux get/try_get callsites in desktop app

Work context: `/Users/khoa2807/development/2026/chatminal`
Plan: `plans/260401-0949-architecture-unification/plan.md`
Phase: `phase-03-kill-mux-singleton.md`
Date: 2026-04-01

## Result
Only 2 files remain in `apps/chatminal-desktop/src` with direct `Mux::get|Mux::try_get|HostMux::get|HostMux::try_get` callsites.

No remaining `Mux::get()` or `Mux::try_get()` in desktop app.
No remaining `HostMux::get()` / `HostMux::try_get()` outside the two lower adapter files below.

## Remaining direct callsites
1. `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:52-57`
- `fn host_mux() -> Arc<HostMux> { HostMux::get() }`
- `fn try_host_mux() -> Option<Arc<HostMux>> { HostMux::try_get() }`
- These are now the only direct host singleton entrypoints in this file.
- Everything else in `DesktopSessionHost` already routes through this helper block.

2. `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs:46-53`
- `notify_pane_output()` calls `HostMux::get().notify(...)`
- `record_input_for_current_identity()` calls `HostMux::get().record_input_for_current_identity()`
- All repeated pane-output/input callsites already funnel through these two helpers.

## Interpretation
Phase 03B cutover of `get/try_get` in desktop app is almost done.
The remaining raw singleton access is already localized at the lowest adapter layer.
This means the fastest path is no longer “grep and replace more `Mux::get()`”.
The real critical path has shifted to facade narrowing + ownership migration for 03C.

## Best parallel slices now
### Slice A: session host raw-host helper extraction
Files:
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

Goal:
- Keep ownership isolated to this file.
- Convert the top helper block into a dedicated lower-boundary abstraction or injected host access object.
- Do not touch callers outside this file.

Why parallel-safe:
- Single-file ownership.
- No overlap with facade callers if others avoid `session_host.rs`.

Impact:
- Medium architectural value.
- Low product risk.
- Not the main blocker for user-visible behavior.

### Slice B: session pane raw notification/input boundary
Files:
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`

Goal:
- Replace the 2 direct `HostMux::get()` helpers with a narrower boundary.
- Likely via shared helper import or callback/bus boundary, depending on chosen design.

Why parallel-safe:
- Single-file ownership.
- No overlap with `session_host.rs` if helper location does not move into shared edited file.

Impact:
- Small-medium value.
- Very low blast radius.

### Slice C: facade narrowing around broad re-export
Files:
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/main.rs`
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/desktop_spawn.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`

Goal:
- Shrink dependency on `pub(crate) use crate::desktop_host_runtime::*;`
- Add explicit facade wrappers in `chatminal_runtime/mod.rs`
- Move these callers off implicit re-export access.

Why parallel-safe:
- Safe if one worker owns the entire facade slice.
- Should not run in parallel with another worker editing `chatminal_runtime/mod.rs`.

Impact:
- Highest Phase 03B value right now.
- Clears the path for broader 03C ownership migration.

## Recommended worker ownership
### Worker 1
Ownership:
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/main.rs`
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/desktop_spawn.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`

Task:
- Continue explicit facade cutover.
- Remove more caller reliance on broad re-export surface.

### Worker 2
Ownership:
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

Task:
- Encapsulate the top raw host helper block further.
- Prepare this file for future `HostMux` removal/injection.

### Worker 3
Ownership:
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`

Task:
- Remove the last 2 direct `HostMux::get()` helper callsites behind a narrower boundary.

## What should NOT be split
- Do not split `chatminal_runtime/mod.rs` across multiple workers.
- Do not have one worker edit `session_host.rs` and another move shared helper code into the same file/module simultaneously.
- Do not mix docs updates into the same workers above if speed is the goal; docs can be updated after integration.

## Critical path assessment
1. Facade narrowing in `chatminal_runtime/mod.rs` and its direct callers
2. Then session_host/session_pane raw-host helper elimination or abstraction
3. Then 03C ownership move out of `Mux`

Reason:
- Remaining `get/try_get` callsites are already localized and low count.
- The bigger blocker is architectural dependency on broad re-export/facade leakage, not raw grep count.

## Direct grep outputs used
- `rg -n "\b(Mux::get|Mux::try_get|HostMux::get|HostMux::try_get)\b" apps/chatminal-desktop/src -g '*.rs'`
- result: only `session_host.rs` and `session_pane.rs`

## Unresolved questions
- None for this audit.
