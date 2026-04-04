# Remaining Mux Singleton Coupling in Chatminal

**Date:** 2026-04-01 | **Focus:** Global Mux exposure points and PTY I/O pipeline

---

## 1. Global Mux Wrapper Functions (Exposed to Desktop)

**In `/crates/chatminal-host-runtime/src/lib.rs` lines 48–240:**

Desktop calls **32 public wrapper functions** that access the global Mux:

### Core Mux Access
- `global_mux() -> Arc<Mux>` (line 48) — direct Mux ref, rarely used
- `try_global_mux() -> Option<Arc<Mux>>` (line 52) — optional access
- `create_global_mux()`, `install_global_mux()`, `shutdown_global_mux()` (lines 64–74) — lifecycle

### Pane/Tab Management
- `terminal_by_id()`, `tab_by_id()`, `remove_pane_by_id()`, `remove_tab_by_id()`
- `add_pane_without_default_side_effects_to_global_mux()`
- `add_tab_and_active_pane_to_global_mux()`, `attach_tab_to_global_mux_window()`

### Window & Focus
- `with_root_window()`, `with_root_window_mut()` — locked window accessors
- `focus_pane_and_containing_tab_global()`, `root_active_tab_id()`

### Workspace & Identity
- `active_workspace_name()`, `set_active_workspace_name()`
- `iter_workspaces_global()`, `is_workspace_empty_global()`
- `active_workspace_for_client_global()`, `set_active_workspace_for_client_global()`
- `resolve_pane_id_global()`, `resolve_focused_pane_global()`
- `record_focus_for_current_identity_global()`, `record_input_for_current_identity_global()`
- `active_identity_global()`

### Spawn & Configuration
- `spawn_tab_in_global_mux()`, `split_pane_in_global_mux()`
- `set_primary_spawn_target_global()`, `primary_spawn_target_global()`
- `tab_by_chatminal_session_id_global()`

---

## 2. Mux Dependency in Parse/Buffer Pipeline

### `parse_buffered_data()` (line 333)
**Does NOT directly access Mux.** Instead:
- Accepts a `Weak<dyn Pane>` reference
- Calls **`send_actions_to_mux()`** (line 315) to apply parsed terminal actions
- `send_actions_to_mux()` is **internal (non-public)** and:
  - Upgrades the weak pane reference
  - Calls `pane.perform_actions(actions)` on the pane
  - Calls `Mux::notify_from_any_thread(MuxNotification::PaneOutput(pane_id))` — **the only Mux singleton access in the pipeline**

**Implications:** The parse pipeline is Mux-aware but doesn't need the full Mux; only needs to notify it via the static notify method.

### `read_from_pane_pty()` (line 472)
**Purely I/O-based, zero Mux coupling:**
- Spawns a thread to parse buffered data via `parse_buffered_data()`
- Reads PTY data into a socketpair
- Dies gracefully when pane is dropped or read fails
- Uses `AtomicBool` for thread coordination, no Mux calls

---

## 3. Desktop Callsites in Host-Runtime Wrapper Functions

**File:** `/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

Desktop calls **27 unique `host_runtime::*` functions** (uniq count):
```
3x host_runtime::terminal_by_id
2x host_runtime::iter_panes_global
2x host_runtime::focus_pane_and_containing_tab_global
1x each for: {terminal_by_id, tab_by_id, remove_pane_by_id, remove_tab_by_id,
             add_tab_and_active_pane_to_global_mux, attach_tab_to_global_mux_window,
             with_root_window, with_root_window_mut, spawn_tab_in_global_mux,
             set_primary_spawn_target_global, primary_spawn_target_global,
             active_workspace_name, set_active_workspace_name,
             active_workspace_for_client_global, set_active_workspace_for_client_global,
             is_workspace_empty_global, iter_workspaces_global,
             resolve_pane_id_global, resolve_focused_pane_global,
             record_focus_for_current_identity_global, record_input_for_current_identity_global,
             active_identity_global, root_active_tab_id, create_global_mux,
             install_global_mux, shutdown_global_mux}
```

**None call `Mux::` statics directly** — all go through host-runtime wrappers.

---

## 4. Promise::spawn Async Dispatch

**Total count:** 36 calls to `promise::spawn::spawn_into_main_thread` across codebase.

**In host-runtime crate (3 calls in `/crates/chatminal-host-runtime/src/`):**
- `lib.rs:540, 548` — in `read_from_pane_pty()` exit handler (spawn error notification)
- `lib.rs:828` — in `Mux::split_pane()` (presumably)
- `tab.rs:1670` — in tab implementation
- `activity.rs:30` — activity tracking
- `localpane.rs:783, 821, 869` — local pane operations (3 calls)

**In desktop (7 calls across files):**
- `frontend.rs` — 6 calls (tab/pane event handlers)
- `termwindow/mod.rs` — 1 call
- `overlay/*.rs` — 4 calls (selector, confirm_close_pane, confirm, prompt, debug)
- `desktop_host_runtime/mod.rs:279` — 1 call (notification bridge)

**In window crate (7 calls):** Platform-specific event dispatch (macOS, Wayland, X11, Windows)

**Nature:** All `spawn_into_main_thread` calls are used to bridge from async/thread contexts back to the main event loop. None directly access Mux; they invoke functions that may do so.

---

## 5. What Would Change to Internalize Mux

### Current State
Mux is a public singleton accessed by desktop via 32 wrapper functions in `lib.rs`.

### To Make Mux Purely Internal

1. **Hide Lifecycle Functions** — Move `create_global_mux()`, `install_global_mux()`, `shutdown_global_mux()` to internal host-runtime initialization; desktop never calls them directly (they're in `session_host.rs` which also imports them).

2. **Replace Direct Mux Returns** — Functions returning `Arc<Mux>` (`global_mux()`, `try_global_mux()`) should be removed or made private. Desktop doesn't call them directly anyway.

3. **Keep Accessor Wrappers** — The 25+ query/mutation functions (`terminal_by_id()`, `focus_pane_and_containing_tab_global()`, etc.) are the API layer. These stay public.

4. **Mux Notification Bridge** — `bridge_mux_notifications()` in `mod.rs:285` takes `&Arc<Mux>` as arg. Desktop never calls it; host-runtime initialization does (line 395 in `session_host.rs`). Can stay internal.

5. **Internal Mux::notify_from_any_thread()** — Used only in `send_actions_to_mux()` (line 321), which is internal. No change needed.

6. **Dependency Risk:** If desktop ever imports `Mux` directly or calls `Mux::try_get()`, that breaks. Currently it doesn't (verified: no `Mux::` calls in desktop code).

### Minimal Change Required
- **Move Mux struct definition to private module** (e.g., `mux_impl.rs`)
- **Rename wrapper functions** from `*_global_mux` to `*` (already mostly clean)
- **Ensure no downstream crate imports `chatminal_host_runtime::Mux`** — currently only `session_host.rs` and `mod.rs` do, both internal to desktop
- **Session initialization stays isolated** in `session_host.rs` or moved to a `host_impl.rs` module

**Assessment:** Desktop is already well-decoupled from Mux singleton. The remaining task is **organizational, not functional**. No API changes needed to the 25+ accessor functions.

---

## Key Files

| File | Role |
|------|------|
| `/crates/chatminal-host-runtime/src/lib.rs` lines 1–250 | 32 wrapper functions + lifecycle |
| `/crates/chatminal-host-runtime/src/lib.rs` lines 315–330 | `send_actions_to_mux()` — only Mux::notify_from_any_thread() call |
| `/crates/chatminal-host-runtime/src/lib.rs` lines 333–430 | `parse_buffered_data()` — Mux-agnostic buffer parsing |
| `/crates/chatminal-host-runtime/src/lib.rs` lines 472–550 | `read_from_pane_pty()` — pure I/O, spawns parse thread |
| `/crates/chatminal-host-runtime/src/lib.rs` line 298 | Mux struct definition |
| `/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` lines 60–130, 380–420 | Desktop Mux initialization + callsites |
| `/apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` lines 285–295 | `bridge_mux_notifications()` |

---

## Unresolved Questions

1. **What happens to `tab_by_chatminal_session_id_global()`?** — Appears unused in session_host.rs; should verify if it's dead code or called elsewhere.
2. **Does `split_pane_in_global_mux()` need the full Mux, or just Pane/Tab refs?** — Should inspect its implementation to see if it's another abstraction candidate.
3. **Are the 36 `spawn_into_main_thread` calls the only async dispatch to Mux mutations?** — Verify no other promise/task machinery touches Mux.
