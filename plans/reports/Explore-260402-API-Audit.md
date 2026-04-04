# Audit Report: Public API in chatminal-host-runtime/src/lib.rs

## Summary

This audit identifies 31 public methods on `impl Mux` and categorizes them by cross-crate usage. The Mux struct itself is `pub(crate)`, but many of its methods are `pub`. Most public methods are only called by internal free function wrappers or initialization code, making them candidates for narrowing to `pub(crate)`.

**Key finding**: The architectural pattern is sound—public cross-crate access goes through safe free function wrappers. Most `pub` methods on Mux could be narrowed to `pub(crate)` without affecting cross-crate callers.

---

## 1. PUB METHODS ON impl Mux (31 total)

### Recommendations by Category

#### A. Constructor & Static Accessors — NARROW to pub(crate)
These are never called from cross-crate code:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| new() | pub | No | NARROW – only called in initialize_host_runtime() |
| get() | pub | No | NARROW – static accessor, internal only |
| try_get() | pub | No | NARROW – static accessor, internal only |
| set_mux() | pub | No | NARROW – initialization only |
| shutdown() | pub | No | NARROW – shutdown only |
| is_main_thread() | pub | No | NARROW – internal thread check |

#### B. Client/Identity Management — MIXED
These are wrapped by MuxHandle or free functions:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| register_client() | pub | No (via MuxHandle) | KEEP – exposed via safe MuxHandle wrapper |
| replace_identity() | pub | No (via MuxHandle) | NARROW – expose only through MuxHandle |

#### C. Notification & Subscription — NARROW to pub(crate)
Only used internally:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| notify() | pub | No | NARROW – only internal calls via notify_mux() |
| notify_from_any_thread() | pub static | No | NARROW – only called via notify_mux_any_thread() wrapper |
| subscribe() | pub | No (via MuxHandle) | NARROW – expose only through MuxHandle |

#### D. Workspace Management — NARROW to pub(crate)
All wrapped by free functions:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| iter_workspaces() | pub | No | NARROW – wrapped by free function |
| set_active_workspace() | pub | No | NARROW – wrapped by free function |
| set_active_workspace_for_client() | pub | No | NARROW – wrapped by free function |
| is_workspace_empty() | pub | No | NARROW – wrapped by free function |

#### E. Pane Management — KEEP or NARROW selectively
Used by internal wrappers:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| add_pane() | pub | No (internal only) | NARROW – wrapped by register_pane_with_default_side_effects() |
| add_pane_without_default_side_effects() | pub | No (internal only) | NARROW – wrapped by register_pane() |
| add_pane_with_io_hooks() | pub | No (internal only) | NARROW – wrapped by register_pane_with_io_hooks() |
| add_pane_with_default_side_effects_and_io_hooks() | pub | No (internal only) | NARROW – wrapped by register_pane_with_default_side_effects_and_io_hooks() |
| iter_panes() | pub | No (internal only) | NARROW – wrapped by free function iter_panes() |
| get_pane() | pub(crate) | No | CORRECT – already private |

#### F. Tab Management — MIXED
Some exposed via wrappers, some internal:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| add_tab_and_active_pane() | pub | No (internal only) | NARROW – wrapped by register_tab() |
| get_tab() | pub(crate) | No | CORRECT – already private |
| get_tab_by_chatminal_session_id() | pub | No (internal only) | NARROW – wrapped by runtime_entry_by_session_id() |
| root_active_tab() | pub | No (internal only) | NARROW – wrapped by root_active_runtime_id() |
| attach_tab() | pub | No (internal only) | NARROW – wrapped by attach_tab_to_window() |

#### G. Window Management — NARROW to pub(crate)
Only crate-internal usage:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| root_window() | pub | No | NARROW – only used in with_root_window() |
| root_window_mut() | pub | No | NARROW – only used in with_root_window_mut() |
| prune_dead_windows() | pub | No | NARROW – internal cleanup only |

#### H. Utility Methods — NARROW to pub(crate)
Internal helpers:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| is_empty() | pub | No | NARROW – internal check only |
| resolve_spawn_target() | pub | No | NARROW – internal to split_pane/spawn_tab |
| primary_spawn_target() | pub | No | NARROW – internal (accessed via primary_spawn_target() free fn) |

#### I. Main Public Operations — KEEP pub
These are core async operations called by free function wrappers:

| Method | Visibility | Used Cross-Crate? | Recommendation |
|--------|------------|-------------------|-----------------|
| split_pane() | pub async | No (internal only) | NARROW – wrapped by free function split_pane() |
| spawn_tab() | pub async | No (internal only) | NARROW – wrapped by free function spawn_tab() |

---

## 2. FREE FUNCTIONS (pub, not pub(crate))

These form the **primary public API** for cross-crate access (chatminal-lua-bridge, chatminal-codec):

All are `pub` and intentionally expose safe subsets of Mux functionality:

- **Initialization**: initialize_host_runtime(), shutdown_host_runtime()
- **Availability**: is_host_runtime_available()
- **Runtime/Tab queries**: root_active_runtime_id(), runtime_entry_by_runtime_id(), runtime_entry_info_by_runtime_id(), etc.
- **Pane queries**: iter_panes(), terminal_by_handle()
- **Workspace ops**: iter_workspaces(), set_active_workspace_name(), rename_workspace()
- **Window ops**: with_root_window(), with_root_window_mut(), root_window_title()
- **Spawn/Split**: spawn_tab(), split_pane()
- **Focus/Input**: focus_terminal_handle(), record_focus_for_terminal_handle()
- **Utility**: terminal_handle_for_pane(), alloc_terminal_handle_value()

**Assessment**: These are well-designed. They provide appropriate encapsulation and should remain `pub`.

---

## 3. PUBLIC TYPES/STRUCTS/ENUMS

| Type | Visibility | Used Cross-Crate? | Recommendation |
|------|------------|-------------------|-----------------|
| MuxHandle | pub | Yes | KEEP – safe initialization wrapper |
| FocusedPaneBinding | pub | No (internal) | Consider pub(crate) – only in resolve_focused_pane() |
| RuntimeEntryInfo | pub | Yes | KEEP – returned by query functions, used in lua-bridge |
| HostRuntimeNotification | pub | Yes | KEEP – exposed for subscription callbacks |
| MuxNotification | pub(crate) | No | CORRECT – internal only |

---

## 4. Summary of Changes

### **10 Methods to NARROW from pub → pub(crate):**

1. Mux::notify()
2. Mux::set_active_workspace()
3. Mux::replace_identity() [or expose only via MuxHandle]
4. Mux::subscribe() [or expose only via MuxHandle]
5. Mux::root_window()
6. Mux::root_window_mut()
7. Mux::prune_dead_windows()
8. Mux::is_empty()
9. Mux::resolve_spawn_target()
10. Mux::get_tab_by_chatminal_session_id()

### **6 Methods to NARROW from pub → pub(crate) (utilities):**

1. Mux::new()
2. Mux::get()
3. Mux::try_get()
4. Mux::set_mux()
5. Mux::shutdown()
6. Mux::is_main_thread()

### **8 Methods to NARROW from pub → pub(crate) (pane operations):**

1. Mux::add_pane()
2. Mux::add_pane_without_default_side_effects()
3. Mux::add_pane_with_io_hooks()
4. Mux::add_pane_with_default_side_effects_and_io_hooks()
5. Mux::iter_panes()
6. Mux::add_tab_and_active_pane()
7. Mux::root_active_tab()
8. Mux::attach_tab()

### **4 Methods to NARROW from pub → pub(crate) (workspace):**

1. Mux::iter_workspaces()
2. Mux::set_active_workspace_for_client()
3. Mux::is_workspace_empty()
4. Mux::notify_from_any_thread()

### **6 Methods to NARROW from pub → pub(crate) (async operations):**

1. Mux::split_pane()
2. Mux::spawn_tab()
3. Mux::primary_spawn_target()

### **2 Types to NARROW from pub → pub(crate):**

1. FocusedPaneBinding (only returned internally)

---

## Impact Analysis

**Affected crates** (if changes made):
- chatminal-lua-bridge: None (uses free functions)
- chatminal-codec: None (uses free functions)
- chatminal-host-runtime (internal): Full scope for narrowing

**Cross-crate callers**: None directly affected because all cross-crate usage goes through free function wrappers or MuxHandle.

**Risk**: LOW – All public methods being narrowed are only called internally. No breaking changes to the public API surface (the free functions).

---

## Conclusion

The current design follows good encapsulation principles. Mux methods are heavily wrapped by safer free function interfaces. **Narrowing 30+ pub methods to pub(crate) would significantly reduce the exposed surface without affecting any cross-crate code.** This is a low-risk refactoring opportunity to improve API clarity.

