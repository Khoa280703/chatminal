---
phase: 03
status: pending
priority: critical
effort: large (5-7 weeks)
risk: medium
---

# Phase 03: Kill Mux → RuntimeHost

## Overview
Thay thế WezTerm's global Mux singleton bằng Chatminal-owned `RuntimeHost` trait. Eliminate dual session model (RuntimeSession vs Tab/Pane).

## Key Insights (từ research)
- **19 files** gọi Mux APIs (~50 callsites tập trung ở `desktop_host_runtime/mod.rs`)
- Chatminal dùng ~35/50 Mux methods
- `RuntimeState` đã có subscriber pattern, session storage — foundation sẵn
- Split layout dùng bintree, chỉ Horizontal/Vertical — không phức tạp
- Lua bridge chỉ có 4 Mux callsites — scope nhỏ

## Architecture Target

### Before (hiện tại)
```
TermWindow → DesktopSessionHost → Mux (global singleton)
                                    ├── Window (RwLock)
                                    ├── tabs: HashMap<TabId, Tab>
                                    ├── panes: HashMap<PaneId, Pane>
                                    └── chatminal_session_id_index (deprecated)

RuntimeState (parallel, no connection to Mux)
```

### After (target)
```
TermWindow → RuntimeHost (Arc, passed explicitly)
               ├── sessions: HashMap<SessionViewId, SessionHost>
               ├── layouts: SplitLayoutManager (bintree)
               ├── notifications: broadcast channel
               └── clipboard/download handlers

RuntimeState integrated into RuntimeHost (owns persistence + execution)
```

## Sub-phases

### 3A. Define RuntimeHost trait (1 week)
1. Create `crates/chatminal-runtime/src/runtime_host.rs`
2. Define trait with methods matching actual Mux usage:
   - `get_session(&self, id: SessionViewId) -> Option<Arc<dyn SessionPane>>`
   - `spawn_session(&self, config: SpawnConfig) -> Result<SessionViewId>`
   - `split_session(&self, source: SessionViewId, direction: SplitDirection) -> Result<SessionViewId>`
   - `close_session(&self, id: SessionViewId)`
   - `active_layout(&self) -> &SplitLayout`
   - `subscribe(&self) -> Receiver<RuntimeNotification>`
3. Implement RuntimeHost on existing DesktopSessionHost (adapter wrapping Mux internally)
4. **Verification gate**: `cargo check --workspace`

### 3B. Migrate TermWindow → RuntimeHost (2 weeks)
1. Replace all `Mux::get()` calls in desktop app with `self.runtime_host.method()`
2. Files to change (19 files):
   - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` — heaviest (~30 callsites)
   - `apps/chatminal-desktop/src/termwindow/mod.rs`
   - `apps/chatminal-desktop/src/desktop_termwindow_*.rs` files
3. Replace PaneId/TabId with SessionViewId/RuntimeId at all UI boundaries
4. **Verification gate**: grep for `Mux::get\|Mux::try_get` → 0 results outside host_runtime

### 3C. Move storage out of Mux (1 week)
1. Move pane/tab HashMap storage into RuntimeHost implementation
2. Mux becomes empty shell (or removed)
3. Notification system: replace `MuxNotification` with `RuntimeNotification`
4. **Verification gate**: `cargo check --workspace`

### 3D. Eliminate Mux singleton (1 week)
1. Remove `Mux::get()` global accessor
2. Remove `static MUX: OnceCell<Arc<Mux>>`
3. Pass `Arc<RuntimeHost>` explicitly through call chain
4. Update `chatminal-host-runtime/src/lib.rs` — Mux struct → minimal or deleted
5. **Verification gate**: grep `Mux` in desktop app → 0 results

### 3E. Migrate Lua bridge (3 days)
1. Lua bridge receives `Arc<RuntimeHost>` instead of calling `Mux::try_get()`
2. Update 4 callsites in `chatminal-lua-bridge/src/lib.rs`
3. Remove `chatminal_session_id_index` from host-runtime
4. **Verification gate**: Lua config loading still works

### 3F. Eliminate dual ID system (3 days)
1. Remove PaneId, TabId from public API
2. All external interfaces use SessionViewId, RuntimeId, TerminalInstanceId only
3. Internal engine can still use numeric IDs, but wrapped
4. Remove type aliases in `desktop_host_runtime/mod.rs` (15+ aliases)

## Architectural Decisions (pre-resolved)
- **Singleton vs passed context**: Pass `Arc<RuntimeHost>` explicitly (no global state)
- **Keep bintree splits**: Works well, no reason to change layout algorithm
- **Clipboard handlers**: Set at RuntimeHost construction time
- **Pane trait**: Keep as-is internally, but expose via SessionPane at boundary
- **overlay_compat**: Eliminate within sub-phase 3D/3E when RuntimeHost replaces Mux — not before
- **promise::spawn replacement**: Use `std::sync::mpsc::SyncSender` for Mux notification replacement (proven pattern from chatminal-runtime persist worker). No tokio — project has no async runtime.

### 3G. Migrate PTY I/O Pipeline (1 week)
1. Move `read_from_pane_pty` + `parse_buffered_data` (~200 LOC) from `chatminal-host-runtime/src/lib.rs` into `session_engine`
2. Replace `send_actions_to_mux()` → `Mux::notify_from_any_thread()` with direct channel send to RuntimeHost
3. Replace `promise::spawn::spawn_into_main_thread` in exit handler (L344-358) with explicit notification dispatch
4. Preserve `BUFSIZE=256KB` socketpair architecture (proven, performant)
5. Preserve `parse_buffered_data` coalescing logic (SynchronizedOutput handling)
6. **Verification gate**: PTY output still renders, session exit still triggers cleanup

## Risk Mitigation
- Each sub-phase is independently compilable
- Adapter pattern (3A) means old code works while migrating
- Can rollback any sub-phase without affecting others
- Feature flag: `cfg(feature = "legacy-mux")` fallback during transition

## Verification
```bash
cargo check --workspace
cargo test --workspace --lib --bins --tests
grep -r "Mux::" apps/ --include="*.rs"  # should be 0
grep -r "PaneId\|TabId" apps/ --include="*.rs" | grep -v "// "  # should be 0 in public API
```

## Success Criteria
- [ ] Zero `Mux::get()` calls in desktop app
- [ ] Zero PaneId/TabId in public API
- [ ] RuntimeHost owns session lifecycle
- [ ] Single ID system (SessionViewId/RuntimeId/TerminalInstanceId)
- [ ] Lua bridge works through RuntimeHost API
- [ ] All tests pass
