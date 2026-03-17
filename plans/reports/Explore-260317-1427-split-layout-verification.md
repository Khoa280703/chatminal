# Architecture Verification: "Split Layout Song Song" Report

**Task:** Verify claims in `docs/architecture-analysis.md` about dual split layout systems in Chatminal.

**Date:** 2026-03-17
**Status:** VERIFIED — Claims are accurate and architecturally critical

---

## Executive Summary

**CONFIRMED:** Two completely separate split layout systems **DO exist and are actively used** in parallel with **zero synchronization** between them. This is a serious architectural problem requiring migration away from the WezTerm engine layer.

| System | File | Lines | Status |
|--------|------|-------|--------|
| **WezTerm Engine (OLD)** | `crates/chatminal-host-runtime/src/tab.rs` | **2528** | ✓ Verified |
| **Chatminal Product (NEW)** | `crates/chatminal-runtime/src/workspace_layout.rs` | **560** | ✓ Verified |

---

## Part 1: File Verification

### 1.1 WezTerm Engine: tab.rs

**Location:** `/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/tab.rs`

**Line count:** 2528 lines ✓

**Key types:**
```rust
pub type Tree = bintree::Tree<Arc<dyn Pane>, SplitDirectionAndSize>;
pub type Cursor = bintree::Cursor<Arc<dyn Pane>, SplitDirectionAndSize>;
```

**Functions found:**
- `Tab::split_and_insert(pane_index, split_request, pane)` (line 740)
- `Tab::remove_pane(pane_id)` (line 680)
- `Tab::resize_split_by(split_index, delta)` (line 639)
- `Tab::set_active_pane(pane)` (line 701)

**Data types:**
- `SplitDirection::Horizontal | Vertical`
- `SplitDirectionAndSize` enum

---

### 1.2 Chatminal Product: workspace_layout.rs

**Location:** `/Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/workspace_layout.rs`

**Line count:** 560 lines ✓

**Key types:**
```rust
pub struct WorkspaceLayoutState {
    pub root_node_id: WorkspaceNodeId,
    pub active_view_id: SessionViewId,
    pub nodes: Vec<WorkspaceLayoutNodeSnapshot>,
    pub views: Vec<SessionViewSnapshot>,
    // ...
}

pub enum WorkspaceLayoutNodeKind {
    View { view_id: SessionViewId },
    Split { axis: WorkspaceSplitAxis, first: WorkspaceNodeId, second: WorkspaceNodeId, ratio: u16 }
}
```

**Functions found:**
- `WorkspaceLayoutState::split_view(view_id, axis, session_id)` (line 78)
- `WorkspaceLayoutState::close_view(view_id)` (line 137)
- `WorkspaceLayoutState::resize_split(node_id, ratio)` (line 159)
- `WorkspaceLayoutState::focus_view(view_id)` (line 128)

**Data types:**
- `WorkspaceSplitAxis::Horizontal | Vertical`
- `ratio: u16` (500-1000 range, with clamping)

---

## Part 2: Call Graph Analysis

### 2.1 Tab.split_and_insert Usage (WezTerm Engine)

**Callers:**
```
tab.split_and_insert()
├── Domain::split_pane() [crates/chatminal-host-runtime/src/domain.rs:140]
│   └── Mux::split_pane() [crates/chatminal-host-runtime/src/lib.rs]
│       └── split_terminal_handle() [apps/chatminal-desktop/src/desktop_host_runtime/mod.rs:868]
│           └── split_terminal_handle_by_public_id() [line 880]
│               └── desktop_spawn.rs:114 [FALLBACK PATH]
│
└── tmux_commands.rs (legacy tmux integration)
└── engine_client (remote client)
```

**Flow path in desktop_spawn.rs (lines 85-127):**
```
SpawnWhere::SplitSession(direction)
  → if can_use_session_view_split && has_active_session:
      [PREFERRED PATH] → desktop_create_split_session_view()
                      → workspace_layout_split_view() (NEW SYSTEM)
  else:
      [FALLBACK PATH] → split_terminal_handle_by_public_id()
                     → Mux.split_pane()
                     → tab.split_and_insert() (OLD SYSTEM)
```

**Condition:** `can_use_session_view_split = spawn.args.is_none() && spawn.set_environment_variables.is_empty()`

---

### 2.2 WorkspaceLayoutState.split_view Usage (Chatminal Product)

**Callers:**
```
WorkspaceLayoutState::split_view()
├── WorkspaceLayoutRegistry::split_view() [crates/chatminal-runtime/src/workspace_layout.rs:399]
│   └── ChatminalRuntimeState::workspace_layout_split_view() [crates/chatminal-runtime/src/state.rs]
│       └── desktop_workspace_layout_split_view() [apps/chatminal-desktop/src/chatminal_runtime/mod.rs]
│           └── DesktopWorkspaceLayoutStore::split_view() [apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs:114]
│               └── desktop_spawn.rs:98 [PRIMARY PATH when can_use_session_view_split]
│
└── Tests (workspace_layout.rs:495+)
```

**Key insight:** Called **only when**:
1. `can_use_session_view_split == true` (no args, no env vars)
2. Session is active (has RuntimeSession)

---

## Part 3: The Critical Architectural Problem

### 3.1 Two Completely Separate Code Paths

| Aspect | Path 1: NEW (Chatminal) | Path 2: OLD (WezTerm) |
|--------|---|---|
| **Entry point** | `desktop_spawn.rs:98` | `desktop_spawn.rs:114` |
| **Condition** | `can_use_session_view_split` | Fallback when condition false |
| **Split system** | WorkspaceLayoutState | Tab (bintree) |
| **Identity** | SessionViewId + WorkspaceNodeId | PaneId + TabId |
| **Rendering** | UI layer: workspace_layout.rs | Engine layer: tab.rs + Mux |
| **Syncing** | **NONE** | **NONE** |
| **Risk** | Split recorded in layout but engine pane not created | Split in engine but layout unaware |

### 3.2 Zero Synchronization Between Systems

**Critical finding:** The two systems are **completely decoupled**. Neither system knows about the other:

1. **When workspace_layout_split_view is called:**
   - Creates new SessionViewId, WorkspaceNodeId
   - Updates WorkspaceLayoutState
   - **But DOES NOT:**
     - Create engine Pane
     - Call Tab.split_and_insert
     - Register pane with Mux

2. **When tab.split_and_insert is called:**
   - Creates new PaneId in Mux
   - Modifies bintree in Tab
   - **But DOES NOT:**
     - Update WorkspaceLayoutState
     - Notify UI of layout change

### 3.3 Desynchronization Risk

The architecture-analysis.md warning is **accurate**:

> Hai hệ thống split này phải luôn sync với nhau qua `desktop_host_runtime` adapter. Bất kỳ bug nào ở lớp adapter đều gây desync giữa UI layout và engine layout.

**Current status:** No adapter exists. They're just parallel systems with a runtime choice.

---

## Part 4: Session View Split Implementation Details

### 4.1 SessionLayout vs WorkspaceLayout

Found **third layout system** (partially):

**File:** `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/session_layout_tree.rs`

```rust
pub enum SessionLayoutNodeKind {
    Leaf { terminal_instance_id: TerminalInstanceId }
}

pub struct SessionLayoutSnapshot {
    pub root_layout_node_id: LayoutNodeId,
    pub active_terminal_instance_id: TerminalInstanceId,
    pub nodes: Vec<SessionLayoutNodeSnapshot>,
    pub leaves: Vec<SessionTerminalInstanceSnapshot>,
}
```

This is a **per-session** layout (only leaf nodes, no splits currently). Doesn't implement splits yet.

---

## Part 5: Function Signature Verification

### Tab.rs Functions
```rust
// Line 740
pub fn split_and_insert(
    &self,
    pane_index: usize,
    split_request: SplitRequest,
    pane: Arc<dyn Pane>
) -> Result<PositionedPane, Error>

// Line 680  
pub fn remove_pane(&self, pane_id: PaneId) -> Option<Arc<dyn Pane>>

// Line 639
pub fn resize_split_by(&self, split_index: usize, delta: isize)

// Line 701
pub fn set_active_pane(&self, pane: &Arc<dyn Pane>)
```

### WorkspaceLayout.rs Functions
```rust
// Line 78
pub fn split_view(
    &mut self,
    view_id: SessionViewId,
    axis: WorkspaceSplitAxis,
    session_id: impl Into<String>,
) -> Option<SessionViewId>

// Line 137
pub fn close_view(&mut self, view_id: SessionViewId) -> bool

// Line 159
pub fn resize_split(&mut self, node_id: WorkspaceNodeId, ratio: u16) -> bool

// Line 128
pub fn focus_view(&mut self, view_id: SessionViewId) -> bool
```

---

## Part 6: Summary Table

| Claim | Status | Evidence |
|-------|--------|----------|
| tab.rs exists at specified path | ✓ YES | File verified at `/crates/chatminal-host-runtime/src/tab.rs` |
| tab.rs has ~2529 lines | ✓ YES | Actual: 2528 lines (off by 1) |
| tab.rs has bintree::Tree | ✓ YES | Line 17: `pub type Tree = bintree::Tree<Arc<dyn Pane>, SplitDirectionAndSize>` |
| tab.rs has split_and_insert | ✓ YES | Line 740 & 1960 (public + impl) |
| tab.rs has remove_pane | ✓ YES | Line 680 & 1596 |
| tab.rs has resize_split_by | ✓ YES | Line 639 & 1261 |
| tab.rs has set_active_pane | ✓ YES | Line 701 & 1750 |
| workspace_layout.rs exists | ✓ YES | File verified at `/crates/chatminal-runtime/src/workspace_layout.rs` |
| workspace_layout.rs ~560 lines | ✓ YES | Actual: 560 lines (exact) |
| workspace_layout.rs has split_view | ✓ YES | Line 78 & 399 |
| workspace_layout.rs has close_view | ✓ YES | Line 137 & 436 |
| workspace_layout.rs has resize_split | ✓ YES | Line 159 & 448 |
| workspace_layout.rs has focus_view | ✓ YES | Line 128 & 424 |
| Both systems actively used | ✓ YES | 88+ references to WorkspaceLayoutState/Registry; split_and_insert in 6+ files |
| Systems are redundant | ✓ YES | Identical operations (split, close, resize, focus) on parallel data structures |
| Zero synchronization | ✓ YES | No shared adapter code, independent code paths in desktop_spawn.rs |

---

## Part 7: Migration Path (Architectural Recommendation)

The new system (WorkspaceLayoutState) is lighter and better architected:

**To consolidate:**
1. Make WorkspaceLayoutState the single source of truth for UI layout
2. Map SessionViewId ↔ PaneId at adapter layer only
3. Remove bintree from Tab, use simple Vec<Arc<Pane>>
4. Move resize/focus logic to adapter layer
5. Keep Mux/Domain for execution engine, not layout

**Current state:** Refactor commits show this is in-progress:
- `ee59dca` removed dead session engine bridge code
- `8dde19a` renamed pane modules
- `cf2dfcd` session-tab collapse
- `ca67ee3` eliminated HostRenderScope

---

## Unresolved Questions

1. **When is tab.split_and_insert still used?** Primarily in tmux command emulation and legacy flows. The `can_use_session_view_split` gate is the transition mechanism.

2. **What triggers fallback to tab.split_and_insert?** When:
   - spawn.args is not empty (custom command args)
   - spawn.set_environment_variables is not empty (env vars set)
   - No active session (create new window instead)

3. **Is SessionLayoutSnapshot ever used for splits?** Currently only for single-leaf views. Split implementation appears incomplete/in-progress.

4. **How are layout changes persisted?** Via `workspace_layout_registry` and runtime store. No tab bintree persistence found.
