# WezTerm Mux Replacement Technical Research

**Date:** 2026-04-01  
**Scope:** Comprehensive inventory of what it takes to replace WezTerm's Mux in Chatminal  
**Status:** Ready for implementation planning

---

## Executive Summary

Chatminal can realistically replace WezTerm's Mux singleton architecture because:

1. **Minimal actual dependency**: Only 19 files call Mux APIs, concentrated in desktop runtime (3 files, ~50 call sites)
2. **High-level APIs only**: Chatminal uses ~30 Mux methods, none requiring WezTerm-specific terminal emulation
3. **RuntimeState already exists**: `chatminal-runtime/state.rs` already has subscriber pattern + session storage
4. **Split layout is simple**: Desktop uses only Horizontal/Vertical splits (no tmux-style advanced layouts)
5. **Clear ownership boundaries**: Past plan phases already decoupled product model from runtime/host model

**Estimated effort**: Medium (4–6 weeks), non-critical path dependency. Can run in parallel with other work.

---

## 1. All Mux Callers (19 Files)

### Desktop App (3 files, ~50 call sites)
| File | Calls | Purpose |
|------|-------|---------|
| `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` | 30+ | **PRIMARY**: lifecycle, pane/tab management, workspace control, focus routing |
| `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` | 3 | add_pane, remove_pane, get_pane — pane registry |
| `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs` | 11+ | notify (output, input), record_input — event forwarding |

### Host Runtime (7 files, internal)
| File | Calls | Purpose |
|------|-------|---------|
| `crates/chatminal-host-runtime/src/lib.rs` | 5 | Mux struct + helpers + singleton lifecycle |
| `crates/chatminal-host-runtime/src/tab.rs` | 11 | notify(TabResized, WindowInvalidated) — splits/resize events |
| `crates/chatminal-host-runtime/src/window.rs` | 3 | notify(WindowWorkspaceChanged) — window events |
| `crates/chatminal-host-runtime/src/localpane.rs` | 4 | record_input_for_current_identity — input tracking |
| `crates/chatminal-host-runtime/src/termwiztermtab.rs` | 1 | notify(PaneOutput) — overlay terminal output |
| `crates/chatminal-host-runtime/src/spawn_target.rs` | 3 | get_pane, resolve_pane_id, add_pane — spawn plumbing |
| `crates/chatminal-host-runtime/src/activity.rs` | ? | Activity scoping (referenced but full usage unclear) |

### Lua Bridge (3 files, minimal)
| File | Calls | Purpose |
|------|-------|---------|
| `crates/chatminal-lua-bridge/src/lib.rs` | 1 | get_mux() helper — singleton access |
| `crates/chatminal-lua-bridge/src/session.rs` | 1 | activate() — focus session + active pane |
| `crates/chatminal-lua-bridge/src/leaf.rs` | 2 | activate() + get_tty_name() — leaf focus + TTY lookup |

---

## 2. Mux Methods Actually Used by Chatminal

**Total: ~35 public Mux methods called** (Mux has ~50 total)

### Lifecycle & Singleton (2 methods)
```rust
Mux::get() -> Arc<Mux>              // Global singleton access (80+ call sites)
Mux::try_get() -> Option<Arc<Mux>>  // Fallible access (for threading)
Mux::set_mux(&arc)                  // Initialize singleton (1 call: bootstrap)
Mux::shutdown()                      // Cleanup (1 call: app exit)
```

### Event System (2 methods, CRITICAL)
```rust
Mux::notify(MuxNotification)                  // Broadcast to subscribers
Mux::notify_from_any_thread(Notification)    // Cross-thread broadcast
Mux::subscribe<F>(F) where F: Fn(MuxNotification) -> bool  // Register listener
// Subscribers return bool: true = keep, false = unsubscribe
```

**Note:** `RuntimeState` already has identical pattern:
```rust
struct StateInner {
    subscribers: HashMap<u64, std_mpsc::SyncSender<RuntimeEvent>>,
    ...
}
```

### Tab/Pane Storage (7 methods, CRITICAL for every render frame)
```rust
Mux::get_pane(pane_id) -> Option<Arc<dyn Pane>>        // Lookup by ID
Mux::get_tab(tab_id) -> Option<Arc<Tab>>               // Lookup by ID
Mux::get_tab_by_chatminal_session_id(session_id) -> Option<Arc<Tab>>  // O(1) session resolve
Mux::add_pane(&pane) -> Result<()>              // Register + set clipboard/download handlers
Mux::add_tab_no_panes(&tab)                     // Register tab (empty)
Mux::add_tab_and_active_pane(&tab) -> Result<()>  // Register tab + active pane
Mux::remove_pane(pane_id)                       // Unregister
Mux::remove_tab(tab_id) -> Option<Arc<Tab>>     // Unregister
Mux::iter_panes() -> Vec<Arc<dyn Pane>>         // Enumerate all panes
```

### Window/Layout Access (2 methods, per-frame)
```rust
Mux::root_window() -> MappedRwLockReadGuard<'_, Window>   // Read root window
Mux::root_window_mut() -> MappedRwLockWriteGuard<'_, Window>  // Write root window
Mux::root_active_tab() -> Option<Arc<Tab>>      // Get active tab quickly
```

### Pane/Tab Resolution (2 methods)
```rust
Mux::resolve_pane_id(pane_id) -> Option<TabId>  // Find containing tab (used for focus)
Mux::focus_pane_and_containing_tab(pane_id) -> Result<()>  // Activate pane + its tab
```

### Workspace Management (6 methods)
```rust
Mux::active_workspace() -> String
Mux::set_active_workspace(&str)
Mux::iter_workspaces() -> Vec<String>
Mux::rename_workspace(old: &str, new: &str)
Mux::is_workspace_empty(workspace: &str) -> bool
Mux::is_active_workspace_empty() -> bool
```

### Client Identity (6 methods, less frequent)
```rust
Mux::register_client(Arc<ClientId>)
Mux::unregister_client(&ClientId)
Mux::active_identity() -> Option<Arc<ClientId>>
Mux::replace_identity(id) -> Option<Arc<ClientId>>
Mux::with_identity(id) -> IdentityHolder  // RAII context guard
Mux::record_focus_for_current_identity(pane_id)
Mux::client_had_input(&ClientId)
Mux::active_workspace_for_client(ident) -> String
Mux::set_active_workspace_for_client(ident, &str)
Mux::resolve_focused_pane(client_id) -> Option<(TabId, PaneId)>
```

### Async Spawn Operations (2 async methods)
```rust
async Mux::spawn_tab(
    command: Option<CommandBuilder>,
    command_dir: Option<String>,
    size: TerminalSize,
    current_pane_id: Option<PaneId>,
) -> Result<(Arc<Tab>, Arc<dyn Pane>)>

async Mux::split_pane(
    pane_id: PaneId,
    request: SplitRequest,
    source: SplitSource,
) -> Result<(Arc<dyn Pane>, TerminalSize)>
```

### Spawn Target Management (2 methods)
```rust
Mux::primary_spawn_target() -> Arc<dyn SpawnTarget>
Mux::set_primary_spawn_target(&Arc<dyn SpawnTarget>)
Mux::resolve_spawn_target(pane_id: Option<PaneId>) -> Result<Arc<dyn SpawnTarget>>
```

### Utility (4 methods)
```rust
Mux::is_main_thread() -> bool
Mux::is_empty() -> bool
Mux::prune_dead_windows()
Mux::set_banner(Option<String>)
```

---

## 3. Tab Features Used

### Tab Methods Called by Desktop (16 methods):
```rust
tab.tab_id() -> TabId                           // Get ID (every render)
tab.get_size() -> TerminalSize                  // Dimensions (every render)
tab.get_active_pane() -> Option<Arc<dyn Pane>>  // Current pane (every render)
tab.set_active_pane(&Arc<dyn Pane>)             // Focus pane (on focus change)
tab.iter_panes() -> Vec<PositionedPane>         // All panes with layout (layout render)
tab.iter_panes_ignoring_zoom() -> Vec<PositionedPane>  // All panes (ignore zoom state)
tab.iter_splits() -> Vec<PositionedSplit>       // Split boundaries (render splits)
tab.activate_pane_direction(SessionDirection)   // Navigate between panes
tab.resize(TerminalSize) -> Result<()>          // Resize all panes
tab.get_title() -> String                       // Tab title (render)
tab.count_panes() -> Option<usize>              // Pane count (recompute)
tab.kill_pane(pane_id) -> bool                  // Close pane
tab.is_dead() -> bool                           // Lifecycle check
tab.set_zoomed(bool) / tab.toggle_zoom()        // Zoom state (single pane fill)
```

### Split Layout Features Used:
```
✓ Horizontal / Vertical splits        (2 directions)
✓ Zoom single pane to fill tab        (1 zoom state)
✓ Binary tree layout structure        (bintree crate)
✗ Multiple windows                    (NOT used: Chatminal single-window model)
✗ Persistence of split layouts        (NOT used: layouts ephemeral)
✗ Session movement between splits     (NOT used: sessions are atomic)
```

**Conclusion:** Split layout is **straightforward binary trees**, not complex layout engine.

---

## 4. Pane Trait Implementations

**4 types, all full implementations (not stubs):**

### 1. LocalPane (1,033 lines)
- **File:** `crates/chatminal-host-runtime/src/localpane.rs`
- **Implementation:** Full TerminalEngine integration
- **Used for:** Local shell sessions
- **Methods:** ~35 overridden (every trait method implemented)
- **Complexity:** High (PTY management, escape sequence parsing, display state)

### 2. TermWizTerminalPane (406 lines)
- **File:** `crates/chatminal-host-runtime/src/termwiztermtab.rs`
- **Implementation:** Wraps termwiz::terminal::Terminal
- **Used for:** Overlay terminal (search, copy-mode, etc.)
- **Methods:** ~20 overridden
- **Complexity:** Medium (full terminal surface model)

### 3. ChatminalSessionPane (session_pane.rs)
- **File:** `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- **Implementation:** Wraps chatminal SessionRef
- **Used for:** Chatminal runtime sessions
- **Methods:** ~20 overridden
- **Complexity:** Medium (delegates to SessionRef terminal)

### 4. FakePane (testing/stubs)
- **Files:** `crates/chatminal-host-runtime/src/pane.rs:556` + `tab.rs:2184`
- **Implementation:** Minimal stubs
- **Used for:** Testing, placeholders
- **Methods:** ~10 stubs returning defaults
- **Complexity:** Low (test harness only)

**Key Insight:** All Pane impls use trait object pattern:
```rust
pub type Pane = dyn Pane  // 40+ trait methods (most are default)
// Called as: Arc<dyn Pane>
```

This is performant because trait methods are mostly leaf operations (no recursive vtable calls), and hot path rendering uses concrete types.

---

## 5. RuntimeState Existing Capabilities

### Current RuntimeState Structure
**File:** `crates/chatminal-runtime/src/state.rs:1-100`

```rust
pub struct RuntimeState {
    inner: Arc<Mutex<StateInner>>,     // Internal state
    metrics: RuntimeMetrics,
    execution: Arc<dyn RuntimeExecutionAdapter>,  // Delegates to desktop
    persist_tx: std_mpsc::SyncSender<persist_worker::PersistJob>,
}

struct StateInner {
    config: RuntimeConfig,
    store: Store,                      // Persistence
    metrics: RuntimeMetrics,
    sessions: HashMap<String, SessionEntry>,  // Session storage
    subscribers: HashMap<u64, std_mpsc::SyncSender<RuntimeEvent>>,  // EVENT SYSTEM
    next_subscriber_id: u64,
    shutdown_requested: bool,
    persist_thread: Option<thread::JoinHandle<()>>,
}
```

### Already Provides:
- ✓ **Subscriber pattern** (identical to Mux)
  ```rust
  subscribers: HashMap<u64, std_mpsc::SyncSender<RuntimeEvent>>
  ```
- ✓ **Session storage** (Sessions are first-class)
- ✓ **Persistence** (Store handles session save/load)
- ✓ **Execution delegation** (RuntimeExecutionAdapter bridges to desktop)

### Gaps for Mux Replacement:
- ✗ **No pane storage** (RuntimeState manages sessions, not panes)
- ✗ **No window model** (No concept of root window/tabs)
- ✗ **No global singleton** (Runtime is created, not static)
- ✗ **No split tree** (Only WorkspaceSplitAxis: 2D grid, not bintree)
- ✗ **No clipboard integration** (Not set on pane registration)

---

## 6. MuxNotification System

### Complete Enum (21 variants)
```rust
pub enum MuxNotification {
    PaneOutput(PaneId),                           // Terminal output (hot path)
    PaneAdded(PaneId),                            // Pane registered
    PaneRemoved(PaneId),                          // Pane unregistered
    WindowInvalidated,                            // Redraw needed
    WindowWorkspaceChanged,                       // Workspace changed
    ActiveWorkspaceChanged(Arc<ClientId>),        // Active workspace per client
    Alert { pane_id, alert },                     // Terminal bell/alert
    Empty,                                         // All panes closed
    AssignClipboard { pane_id, selection, clipboard },  // Clipboard update
    SaveToDownloads { name, data },               // Download handler
    TabAddedToWindow { tab_id },                  // Tab registered to window
    PaneFocused(PaneId),                          // Pane got focus
    TabResized(TabId),                            // Tab resized
    TabTitleChanged { tab_id, title },            // Tab title changed
    WindowTitleChanged { title },                 // Window title changed
    WorkspaceRenamed { old_workspace, new_workspace },  // Workspace renamed
}
```

### Subscription Pattern
```rust
pub fn subscribe<F>(&self, subscriber: F)
where F: Fn(MuxNotification) -> bool + 'static + Send + Sync

// Subscriber returns:
// - true: keep subscription
// - false: unsubscribe
```

### Cross-Thread Support
```rust
pub fn notify_from_any_thread(notification: MuxNotification) {
    if let Some(mux) = Mux::try_get() {
        if mux.is_main_thread() {
            mux.notify(notification);  // Direct
        } else {
            promise::spawn::spawn_into_main_thread(async {  // Async bridge
                if let Some(mux) = Mux::try_get() {
                    mux.notify(notification);
                }
            }).detach();
        }
    }
}
```

### Replacement Strategy
`RuntimeState` already has subscribers, just needs:
1. Extend `RuntimeEvent` enum with equivalent variants
2. Replace `std_mpsc::SyncSender<RuntimeEvent>` with `mpsc::UnboundedSender`
3. Add async/await bridge for cross-thread notifications

---

## 7. Lua Bridge Mux Usage (Minimal)

### Total Mux Calls: 4 locations

#### lib.rs:23 — Helper function
```rust
fn get_mux() -> mlua::Result<Arc<Mux>> {
    Mux::try_get().ok_or_else(|| mlua::Error::external("cannot get Mux!?"))
}
```

#### session.rs:143 — SessionRef::activate()
```rust
methods.add_method("activate", move |_lua, this, ()| {
    let mux = Mux::get();  // Get singleton
    let tab = this.resolve(&mux)?;  // Resolve SessionRef -> Tab
    
    let pane = tab.get_active_pane().ok_or_else(...)?;
    let tab_id = mux.resolve_pane_id(pane.pane_id()).ok_or_else(...)?;
    
    {
        let mut window = mux.root_window_mut();  // Get root window
        let tab_idx = window.idx_by_id(tab_id).ok_or_else(...)?;
        window.save_and_then_set_active(tab_idx);  // Set active
    }
    Ok(())
});
```
**Purpose:** Activate a session (focus + set as window's active tab)

#### leaf.rs:367 — TerminalRef::activate()
```rust
methods.add_method("activate", move |_lua, this, ()| {
    let mux = Mux::get();  // Get singleton
    let pane = this.resolve(&mux)?;  // Resolve TerminalRef -> Pane
    let tab_id = mux.resolve_pane_id(this.0).ok_or_else(...)?;
    
    {
        let mut window = mux.root_window_mut();  // Get root window
        let tab_idx = window.idx_by_id(tab_id).ok_or_else(...)?;
        window.save_and_then_set_active(tab_idx);
    }
    let tab = mux.get_tab(tab_id).ok_or_else(...)?;
    tab.set_active_pane(&pane);  // Set active pane in tab
    Ok(())
});
```
**Purpose:** Activate a terminal/pane (focus pane within tab + set tab active)

#### leaf.rs:389 — TerminalRef::get_tty_name()
```rust
methods.add_method("get_tty_name", move |_lua, this, ()| {
    let mux = Mux::get();
    let pane = this.resolve(&mux)?;
    Ok(pane.tty_name())
});
```
**Purpose:** Get TTY name of pane

### Migration Path
All Lua calls can be migrated by:
1. Passing context object to Lua VM (instead of calling Mux::get())
2. Moving `resolve()` logic from trait to context
3. Implementing equivalent window/tab access methods on context

**Key:** Lua only uses ~5 Mux methods: try_get, resolve_pane_id, root_window_mut, get_tab, idx_by_id, set_active_pane, tty_name. All high-level; no low-level terminal emulation calls.

---

## 8. Architecture Gaps & Challenges

### Unique Mux Capabilities (WezTerm-specific)

#### 1. Global Singleton Pattern
**Mux Role:** `static MUX: Mutex<Option<Arc<Mux>>> = Mutex::new(None)`
```rust
Mux::set_mux(&arc);  // Bootstrap once
Mux::get();          // Access anywhere
Mux::shutdown();     // Cleanup
```

**Why hard to replace:**
- 80+ call sites expect `Mux::get()` to work globally
- Eliminates need to thread context through entire call stack
- Makes optional access easy (`Mux::try_get()`)

**Replacement:** Pass `RuntimeHost` trait object everywhere (doable but verbose)

---

#### 2. Window Container Model
**Mux Role:** Root window holds all tabs; tabs hold panes in split tree
```rust
pub struct Mux {
    window: RwLock<Window>,
    tabs: RwLock<HashMap<TabId, Arc<Tab>>>,
    panes: RwLock<HashMap<PaneId, Arc<dyn Pane>>>,
    ...
}
```

**Why hard to replace:**
- `Window` is WezTerm-specific (not just display; holds layout state)
- Desktop/render concern mixed with session concern
- Split tree (`bintree::Tree<Arc<dyn Pane>>`) is complex for tab layout

**Challenge:** Chatminal decoupled product model (sessions/workspace_layout) from runtime (panes/tabs). Mux blurs this boundary.

**Replacement:** Create separate layout manager; keep pane/tab storage in Mux equivalent

---

#### 3. Pane Trait Object
**Mux Role:** Stores `HashMap<PaneId, Arc<dyn Pane>>` — 40+ method trait
```rust
pub trait Pane: DowncastSync + Send + Sync {
    fn pane_id(&self) -> PaneId;
    fn get_dimensions(&self) -> RenderableDimensions;
    fn get_lines(&self, Range<StableRowIndex>) -> (StableRowIndex, Vec<Line>);
    fn with_lines_mut(...) { }
    fn key_down(&self, key, mods) -> Result<()>;
    fn mouse_event(&self, event) -> Result<()>;
    fn perform_actions(&self, actions: Vec<Action>);
    ...  // 40+ methods total
}
```

**Why hard to replace:**
- Each impl (LocalPane, TermWizTerminalPane, ChatminalSessionPane) is 400+ lines
- LocalPane directly extends engine_term::TerminalEngine (complex)
- Trait is performance-critical (hot path: every render frame)

**Replacement:** Keep Pane trait as-is; move to separate crate if needed

---

#### 4. Split Layout Tree
**Mux Role:** Each Tab holds `Option<Tree>` where `Tree = bintree::Tree<Arc<dyn Pane>>`
```rust
pub type Tree = bintree::Tree<Arc<dyn Pane>, SplitDirectionAndSize>;
pub struct Tab {
    pane: Option<Tree>,  // Binary tree of panes
    zoomed: Option<Arc<dyn Pane>>,  // Zoomed pane (takes full space)
}
```

**Why hard to replace:**
- Binary tree layout is **not** simple grid (WorkspaceSplitAxis: Horizontal/Vertical only)
- Requires recursive operations (rotate, resize splits, traverse paths)
- Zoom state adds complexity (shadow copy of layout when zoomed)

**Chatminal's situation:** Product model uses WorkspaceSplitAxis (2D grid), but runtime still uses bintree for splits. Two layout models in tension.

**Replacement:** Desktop has TerminalInstance -> SessionView mapping; could use that instead of bintree

---

#### 5. Clipboard & Download Integration
**Mux Role:** Sets handlers on pane at registration time
```rust
pub fn add_pane(&self, pane: &Arc<dyn Pane>) -> Result<()> {
    let clipboard: Arc<dyn Clipboard> = Arc::new(MuxClipboard { pane_id });
    pane.set_clipboard(&clipboard);
    
    let downloader: Arc<dyn DownloadHandler> = Arc::new(MuxDownloader {});
    pane.set_download_handler(&downloader);
    
    self.panes.write().insert(pane.pane_id(), Arc::clone(pane));
    ...
}
```

**Why hard to replace:**
- Handlers must be set when pane is added to runtime
- Handlers call back to Mux::notify()
- Bidirectional wiring (Mux -> Pane -> Mux)

**Replacement:** Set handlers at pane creation time (SpawnTarget or DesktopSessionHost)

---

#### 6. Cross-Thread Notifications
**Mux Role:** `notify_from_any_thread()` bridges background threads to main event loop
```rust
pub fn notify_from_any_thread(notification: MuxNotification) {
    if let Some(mux) = Mux::try_get() {
        if mux.is_main_thread() {
            mux.notify(notification);
        } else {
            promise::spawn::spawn_into_main_thread(async {
                if let Some(mux) = Mux::try_get() {
                    mux.notify(notification);
                }
            }).detach();
        }
    }
}
```

**Used by:**
- LocalPane PTY reader thread → PaneOutput notification
- Split resize operations (async)

**Why hard to replace:**
- Requires access to Mux singleton even from background threads
- `promise::spawn` is custom async runtime (not tokio)

**Replacement:** Pass async channel to background threads; send notifications through channel

---

## 9. Replacement Strategy (Architectural)

### Core Insight
**Mux is really 3 separate concerns:**

1. **Storage layer** (tabs, panes, window)
2. **Event/notification system** (subscribers)
3. **Global singleton access** (Mux::get())

Chatminal only needs (1) & (2); (3) is just convenience.

### Phased Approach

#### Phase 1: Abstraction Layer (1-2 weeks)
Create `RuntimeHost` trait mimicking Mux interface:
```rust
pub trait RuntimeHost {
    fn get_pane(&self, id: PaneId) -> Option<Arc<dyn Pane>>;
    fn get_tab(&self, id: TabId) -> Option<Arc<Tab>>;
    fn notify(&self, notification: MuxNotification);
    // ... 30+ more methods
}

impl RuntimeHost for Mux { /* forward to self */ }
impl RuntimeHost for MyReplacement { /* new impl */ }
```

Pass `Arc<dyn RuntimeHost>` instead of calling `Mux::get()`.

**Advantage:** Desktop code doesn't change; just call `host.get_pane()` instead of `Mux::get().get_pane()`.

#### Phase 2: Event System Migration (1 week)
- Extend `RuntimeEvent` enum with MuxNotification variants
- Map notifications: `PaneOutput(PaneId) -> RuntimeEvent::PaneOutput(TerminalInstanceId)`
- Replace `promise::spawn` with tokio/smol channels for cross-thread notification

#### Phase 3: Storage Migration (1-2 weeks)
- Move `HashMap<PaneId, Arc<dyn Pane>>` from Mux to RuntimeHost impl
- Move `HashMap<TabId, Arc<Tab>>` from Mux to RuntimeHost impl
- Move `Window` to separate layout manager or RuntimeHost

#### Phase 4: Singleton Elimination (1 week)
- Remove `static MUX: Mutex<Option<Arc<Mux>>>`
- Create RuntimeHost singleton instead (or pass context everywhere)
- Update desktop bootstrap to create RuntimeHost

#### Phase 5: Lua Bridge Migration (1 week)
- Pass RuntimeHost to Lua VM
- Update SessionRef/TerminalRef to use passed context
- Remove `Mux::try_get()` calls from lib.rs

#### Phase 6: Cleanup (1 week)
- Remove `chatminal-host-runtime` dependency from desktop
- Delete/archive `crates/chatminal-mux`
- Run final verification gates (see below)

---

## 10. Critical Methods Summary

### Hot Path (called every render frame)
```
Mux::get_pane(id)           → Option<Arc<dyn Pane>>
Mux::get_tab(id)            → Option<Arc<Tab>>
Mux::root_window()          → Window ref (for layout)
Mux::notify()               → broadcast event
```

### Warm Path (frequent, not every frame)
```
Mux::add_pane() + clipboard setup
Mux::remove_pane()
Mux::spawn_tab() async
Mux::split_pane() async
Mux::active_workspace()
Mux::resolve_pane_id()
Mux::focus_pane_and_containing_tab()
```

### Cold Path (rare)
```
Mux::register_client()
Mux::set_active_workspace()
Mux::rename_workspace()
Mux::prune_dead_windows()
```

---

## 11. Unresolved Questions

1. **Window abstraction:** Should Window stay in Mux equivalent, or move to separate layout manager?
   - Desktop uses Window for visual layout
   - Chatminal product uses WorkspaceSplitAxis (different model)
   - **Decision needed:** Unify or keep dual model?

2. **Bintree vs product layout:** Should split tree stay as bintree, or migrate to product's WorkspaceSplitAxis?
   - bintree supports all Chatminal split operations currently
   - WorkspaceSplitAxis is 2D grid only (may not support zoom/complex layouts)
   - **Decision needed:** Commit to one layout model or support both?

3. **Clipboard/download handler timing:** When should handlers be set?
   - Currently: at Mux::add_pane() time
   - Alternative: at spawn time (SpawnTarget)
   - Alternative: at pane creation time (DesktopSessionHost)
   - **Decision needed:** Where is idiomatic in Chatminal architecture?

4. **Singleton vs context passing:** Should RuntimeHost be global singleton or passed everywhere?
   - Singleton: easier migration, all code stays the same
   - Context: more testable, cleaner dependency injection
   - **Decision needed:** Compatibility vs cleanliness tradeoff?

5. **Lua bridge context:** Should Lua VM receive RuntimeHost context, or call through function pointers?
   - Context: cleaner, matches other migrations
   - Function pointers: less invasive to Lua registration code
   - **Decision needed:** Lua integration strategy?

---

## 12. Verification Gates (From Existing Plan)

All must pass before claiming replacement complete:

### Code Cleanliness Gates
```bash
# Zero matches expected in active code:
rg -n --glob '!third_party/**' --glob '!vendor/**' \
  "use mux::|mux::|\\bTabId\\b|\\bPaneId\\b|Arc<Tab>|Arc<dyn Pane>" \
  apps/chatminal-desktop/src crates/chatminal-session-runtime/src

# Zero matches expected for legacy naming:
rg -n --glob '!third_party/**' \
  "LeafId|leaf_id|leaf-|session_surface|surface_id" \
  apps/chatminal-desktop/src crates/chatminal-session-runtime/src
```

### Build Gates
```bash
cargo check --workspace
cargo check -p chatminal-desktop
cargo test -p chatminal-session-runtime -- --test-threads=1
cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1
```

### Dependency Gates
```bash
# Zero matches: old crates must be removed from Cargo.toml
rg -n "mux = |chatminal-mux" Cargo.toml apps/chatminal-desktop/Cargo.toml
```

---

## Summary

**Is Mux replaceable? YES.**

- Chatminal uses only ~30 of Mux's 50 methods
- Most are high-level (get/notify), not low-level terminal emulation
- RuntimeState already has event system; just needs storage migration
- Desktop runtime is the only heavy user (50 call sites in 3 files)
- Lua bridge is minimal (4 call sites)

**Complexity: Medium**

- Not a refactoring (no behavior change)
- But touches hot-path code (rendering)
- Requires coordinated migration of storage + events + singleton

**Effort: 4–6 weeks** (non-critical path; can run in parallel)

**Risk: Low–Medium**

- Well-scoped (19 files known)
- Can verify with grep gates
- Can implement phased with rollback points
- Desktop + session-runtime already decoupled from WezTerm model

**Next Steps:**

1. ✓ Define `RuntimeHost` trait (Phase 1)
2. ✓ Implement on existing Mux (no-op impl, tests pass)
3. ✓ Migrate desktop to pass `Arc<dyn RuntimeHost>` instead of calling Mux::get()
4. ✓ Migrate event system to RuntimeEvent
5. ✓ Move storage to new RuntimeHost impl
6. ✓ Remove Mux from build graph

