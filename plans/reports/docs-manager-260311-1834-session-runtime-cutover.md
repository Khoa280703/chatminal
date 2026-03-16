# Documentation Update Report: Session Runtime Cutover

**Date:** 2026-03-11
**Scope:** Chatminal Desktop Phase 03+ session-native runtime final cutover
**Status:** Complete

## Summary

Updated project documentation to reflect the Chatminal Desktop session execution core final cutover from mux-adapter pattern to session-native runtime. All architectural changes from commit `a7c7287` (refactor: checkpoint session runtime cutover) now documented.

## Files Updated

### 1. `docs/system-architecture.md` (86 LOC)

**Changes:**
- Rewrote topology section to clearly separate Desktop (first-party native) from Legacy daemon/CLI paths
- Added detailed "Runtime flow (Desktop Path — Phase 03+)" section covering:
  - Session lifecycle: DesktopSessionHost initialization → ensure_surface → sync_panes → focus operations
  - Input/output routing via ChatminalSessionPane ↔ LeafRuntime
  - Pane lifecycle and Tab resource management
  - Active-path function routing through DesktopSessionHost natively
- Expanded "Main components" with subsections:
  - Desktop Runtime (Phase 03+): DesktopSessionHost, ChatminalSessionPane, StatefulSessionEngine<()>
  - Session Engine Core: adapter pattern, StatefulSessionEngine<()> vs ChatminalMuxSessionEngine
  - Input backpressure runtime
  - Transport layer
  - Shared protocol/store crates
- Updated last-modified timestamp to 2026-03-11

**Key Architectural Facts Documented:**
- `DesktopSessionHost` owns panes HashMap (LeafId→ChatminalSessionPane) + surface_tabs HashMap (SurfaceId→Tab) + active_session_id tracking
- `StatefulSessionEngine<()>` is active execution primitive for desktop (no mux overhead)
- `ChatminalMuxSessionEngine = StatefulSessionEngine<ChatminalEngineSurfaceAdapter>` kept only for multi-leaf adapter-compat ops
- Active-path functions route natively through DesktopSessionHost (no adapter translation)

### 2. `docs/codebase-summary.md` (91 LOC)

**Changes:**
- Updated runtime baseline section to reflect Phase 03+ cutover:
  - Added `apps/chatminal-desktop` (~2,500+ LOC, 8+ files) as first-party window
  - Clarified `apps/chatminald` + `apps/chatminal-app` as legacy IPC compat
  - Added `crates/chatminal-session-runtime` (~3,000+ LOC, 18+ files) as shared engine core
- Reorganized high-signal files into two categories: Desktop (Phase 03+) and Daemon/Legacy
- Added new Desktop section covering:
  - `session_host.rs`: lifecycle manager architecture
  - `session_pane.rs`: per-leaf pane I/O routing
  - `chatminal_session_surface.rs`: public native API surface
  - `session_engine.rs`: StatefulSessionEngine generic pattern
  - `session_core_state.rs`: session_surface_map() enumeration method
  - `engine_surface_adapter.rs`: adapter trait for legacy compat
  - `session_engine_core.rs`: native-path methods (ensure/focus/close)
- Added comprehensive "Session Engine Core Architecture" section explaining:
  - StatefulSessionEngine<A> pattern: desktop `StatefulSessionEngine<()>` vs legacy mux adapter
  - Native methods: ensure_session_surface_native, focus_surface_native, focus_leaf_native, close_surface_native
  - Core state enumerations: session_surface_map(), surface(), reconcile_lookup()
- Clarified current risk: session_core_state.rs state access patterns + Phase 07 adapter removal plan
- Updated last-modified timestamp to 2026-03-11

## Architectural Changes Captured

### 1. DesktopSessionHost Lifecycle Management
Documents that DesktopSessionHost is now the session lifecycle manager per window:
- Owns panes HashMap (LeafId → ChatminalSessionPane) for output/input routing
- Owns surface_tabs HashMap (SurfaceId → Tab) for termwindow compat render boundary
- Tracks active_session_id + last_active_session_id for session switching

### 2. Session-Native Execution Primitive
Clarifies that StatefulSessionEngine<()> is the active execution primitive:
- Direct core state operations without adapter overhead
- Used exclusively in desktop path (Phase 03+)
- ChatminalMuxSessionEngine only for legacy multi-leaf adapter ops

### 3. Active-Path Function Routing
Documents that all active-path functions now route natively through DesktopSessionHost:
- `session_native_ensure_surface()` → DesktopSessionHost.ensure_surface()
- `session_native_focus_surface()` → DesktopSessionHost.focus_surface()
- `session_native_focus_leaf()` → DesktopSessionHost.focus_leaf()
- `collect_session_surface_lookup()` → DesktopSessionHost.collect_lookup()

### 4. SessionCoreState.session_surface_map()
Documents new enumeration method for native lookup:
- Returns iterator over (session_id, surface_id) pairs
- Used by DesktopSessionHost.collect_lookup() to build SessionSurfaceLookup
- Enables efficient enumeration without adapter translation

### 5. Pane Lifecycle Synchronization
Documents sync_panes_for_surface() flow:
- Creates ChatminalSessionPane objects for each leaf in layout
- Registers panes with Mux for render compat
- Removes panes when layout no longer contains leaf
- Maintains Tab wrapper with active pane tracking

## Verification

✓ Both files well under 800 LOC limit (86 + 91 LOC total)
✓ All architectural facts verified against source code:
  - DesktopSessionHost (session_host.rs lines 53-350)
  - StatefulSessionEngine<()> usage (session_host.rs line 83)
  - Native methods routing (session_host.rs lines 96-148)
  - session_surface_map() enumeration (session_core_state.rs)
  - Active-path function routing (chatminal_session_surface.rs lines 40-79)

✓ Timestamp updated to current date (2026-03-11)
✓ No broken internal links
✓ Consistent terminology with codebase

## Integration Impact

**Docs dependencies affected:** None (standalone updates)
**Code changes required:** None (documentation only)
**Roadmap/Changelog:** Handled by project-manager agent concurrently

## Notes

- Documentation reflects Phase 03+ final cutover state
- Legacy daemon/IPC path preserved for backward compatibility (daemon still used via `make daemon`)
- Multi-leaf adapter ops (swap/move/direction) documented as Phase 07 candidates for removal
- Session engine architecture follows adapter pattern: StatefulSessionEngine<A> generic over adapter type
