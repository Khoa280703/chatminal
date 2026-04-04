---
phase: 03
status: done
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

## Status Audit (2026-04-02)
Snapshot dưới đây là trạng thái lịch sử trước closeout; kết luận cuối cho done gate hiện nằm ở [phase-05-final-closeout.md](./phase-05-final-closeout.md) và [final-closeout-checklist.md](./final-closeout-checklist.md).

## Closeout Completed (2026-04-03)
- `initialize_host_runtime()` và `shutdown_host_runtime()` dùng host-runtime root trực tiếp; product path không còn dựng/shutdown qua `Mux` owner path.
- Product PTY/local spawn path đã chuyển sang `host_default()`:
  - `PtyIoHooks::host_default()`
  - `LocalPaneHooks::host_default()`
  - `LocalSpawnHooks::host_default()`
- `mux_default()` chỉ còn ở explicit compat seam/tests.
- Desktop/lua boundary không còn `PaneId` / `TabId` blocker; raw ids còn lại được giữ crate-local trong `chatminal-host-runtime`.

- `3A Define RuntimeHost trait`: `done`
  - Lý do: trait đã tồn tại trong `chatminal-runtime`, `DesktopSessionHost` đã implement, verify gate compile đã xanh.
- `3B Migrate TermWindow → RuntimeHost`: `partial`
  - Done trong scope:
    - desktop app đã hết direct `Mux::get()` / `Mux::try_get()` outside host layer
    - phần lớn facade execution/window/workspace/pane fallback đã route qua `DesktopSessionHost` / `RuntimeHost` first
  - Chưa done:
    - UI/public boundary vẫn còn compat wrappers và raw host-id leakage
    - `TermWindow → RuntimeHost` chưa thành tuyến sạch hoàn toàn
- `3C Move storage out of Mux`: `partial`
  - `03C Scope A`: `done`
  - `03C Scope B`: `done`
  - `03C Scope C`: `done` cho pane-registry local-first slice đã scope, nhưng `3C` toàn phần chưa done vì `Mux` vẫn còn giữ registry ownership chính.
- `3D Eliminate Mux singleton`: `partial`
  - Done trong scope:
    - desktop app không còn direct singleton accessor calls
    - control-plane đã bắt đầu tách khỏi `Mux` vào `HostRuntimeControlPlane`
    - ownership thật của host runtime đã chuyển sang `HostRuntimeRoot`
    - global slot giờ chỉ giữ `Weak<HostRuntimeRoot>`; `Mux` đã hạ xuống compat facade mỏng
    - root/window/workspace/query/control helper layer giờ resolve trực tiếp qua root; `tab::prune_dead_panes()` cũng không còn bám global mux facade
  - Chưa done:
    - compat helper names (`try_global_mux()` / `with_mux()` / `with_mux_strict()`) vẫn còn tồn tại cho migration path
    - mutation/lifecycle paths (`add/remove/focus/spawn/split`) vẫn còn đi qua compat `Mux` facade
    - chưa pass `Arc<RuntimeHost>` explicit end-to-end cho toàn call chain
- `3E Migrate Lua bridge`: `partial`
  - Scoped done:
    - workspace/window metadata reads-writes: `done`
    - session/tab lookup and activation: `done`
    - pane lookup + pane metadata/query methods: `done`
    - spawn/split operations hardening: `done`
  - Chưa done:
    - bridge vẫn đang bám `LuaBridgeHost` trên nền host-runtime hiện tại, chưa chuyển hẳn sang `RuntimeHost` boundary đúng intent cuối
- `3F Eliminate dual ID system`: `partial`
  - Done trong scope:
    - `TerminalRef` raw tuple boundary đã bị khóa
    - `FocusedPaneBinding` thay raw tuple ở focused-pane path
    - desktop pane bindings đầu tiên đã chuyển sang `RuntimeId` / `SessionTerminalHandle`
    - split boundary cũng đã siết thêm một nấc:
      - `SplitSource::MovePane(PaneId)` đã đổi thành `MoveTerminal(SessionTerminalHandle)`
      - public `SpawnTarget::split_pane(...)` không còn lộ `TabId` / `PaneId`; raw conversion bị đẩy vào host-runtime internals
    - thêm một nhát trim public surface:
      - `pane::alloc_pane_id()` đã bị kéo về crate scope; desktop dùng root helper `alloc_terminal_handle_value()` thay vì chạm module `pane`
      - `Window::{idx_by_id, remove_by_id, prune_dead_tabs}` đã bị kéo về crate scope vì chỉ còn caller nội bộ
    - thêm một nhát trim ở core host object:
      - `Mux::{record_focus_for_client, focus_pane_and_containing_tab, get_pane, get_tab, remove_pane, remove_tab, resolve_pane_id}` đã bị kéo về crate scope
      - raw `PaneId` / `TabId` không còn lộ qua nhóm method này của `Mux`
    - thêm một nhát typed boundary ở `tab.rs`:
      - `Tab::contains_pane(...)` đã đổi từ `PaneId` sang `SessionTerminalHandle`
      - desktop caller duy nhất đã chuyển theo; phần convert raw id chỉ còn nằm trong `tab.rs`
  - Chưa done:
    - `PaneId` / `TabId` vẫn còn ở public API
    - chưa đạt single ID system ở toàn boundary
- `3G Migrate PTY I/O Pipeline`: `partial`
  - Done trong scope:
    - đã có `PtyIoDispatcher` nội bộ để gom side effects `on_output`, `on_cleanup`, `on_inline_error_output`
    - `send_actions_to_mux(...)`, `parse_buffered_data(...)`, `read_from_pane_pty(...)` đã chuyển qua dispatcher boundary này
    - `add_pane_internal(...)` không còn giữ nguyên một khối ownership; registration và PTY-reader startup đã tách bước đầu
    - parser/read loop cluster đã được kéo ra khỏi `chatminal-host-runtime/src/lib.rs` vào module nội bộ `chatminal-host-runtime/src/pty_io.rs`
    - `LocalPane` side effects (`record_input`, `inline output`, `alert`, `child-exit cleanup`) đã được gom qua `LocalPaneHooks` thay vì hard-code trực tiếp ở từng callsite trong `localpane.rs`
    - PTY reader startup hiện đã nhận một contract nội bộ `PtyIoHooks`, nên owner tương lai có thể override đủ `output / cleanup / inline error` thay vì chỉ có `on_output`
    - session-native PTY loop ở `leaf_runtime.rs` / `leaf_runtime_threads.rs` cũng đã được bóc một bước đầu thành `TerminalInstanceRuntimeHooks`, nên reader/waiter thread không còn hard-code `events.send(...)` trực tiếp
    - local fallback spawn path cũng đã có seam riêng:
      - `LocalSpawnTarget` giờ nhận `LocalSpawnHooks`
      - path này không còn hard-code `LocalPane::new(...)` + `register_pane_with_default_side_effects(...)` thành một bundle đóng; desktop/runtime owner có thể inject typed callbacks cho local-pane side effects và PTY output/cleanup mà không phải làm rò `PaneId` ở public boundary
  - Chưa done:
    - default dispatcher vẫn map về Mux-backed cleanup/notification semantics
    - lifecycle owner cuối vẫn chưa chuyển sang `session_engine`

## Audit Reset (2026-04-03)
- Claim `done` trước đó không còn hợp lệ theo source hiện tại.
- Current closeout state của worktree:
  - bootstrap/shutdown product path đã rời `Mux` owner và đi qua installed `HostRuntimeRoot`
  - active product path không còn chọn `mux_default()` làm default; `host_default()` đã thay vào đó
  - raw `PaneId` / `TabId` còn lại chủ yếu là crate-local internals / notification implementation detail
- Gate này đã được đóng trong lượt closeout hiện tại:
  - verify cuối đã xanh
  - final checklist đã chốt raw-id residual scope
  - phase/docs đã sync lại theo evidence cuối

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
- [x] Zero `Mux::get()` calls in desktop app
- [x] Zero PaneId/TabId in public cross-crate API
- [x] RuntimeHost/HostRuntimeRoot owns session lifecycle cho toàn bộ active product path, không còn bootstrap/shutdown/notify blocker
- [x] Compat `Mux` survives only as explicit compat seam, không còn bị materialize như owner mặc định ở runtime product flow
- [x] Lua bridge works through RuntimeHost API
- [x] Raw host ids còn lại đã được internalize hoặc re-scope rõ trong closeout checklist
- [x] All tests pass sau closeout sweep cuối

## Deferred Items
- Full single internal numeric-id system (`PaneId`/`TabId` removed from every crate-local storage/notification detail) is out of scope for this closeout. Done gate for this phase is public/runtime boundary closure, not internal alias eradication.

## Progress
  - 2026-04-02:
  - local fallback PTY ownership seam tiếp tục mở thêm một nhát:
    - `chatminal-host-runtime/src/spawn_target.rs` thêm `LocalSpawnHooks`
    - `LocalSpawnTarget::{new_with_hooks,new_serial_target_with_hooks}` cho phép owner truyền callback typed bằng `SessionTerminalHandle` cho:
      - PTY output
      - PTY cleanup
      - PTY inline error output
      - local pane input / inline output / alert / child-exit cleanup
    - `LocalSpawnTarget::spawn_pane(...)` giờ dùng `LocalPane::new_with_hooks(...)` thay vì khóa cứng `LocalPane::new(...)`
    - pane registration của local fallback path giờ đi qua `register_pane_with_default_side_effects_and_io_hooks(...)`, nên default clipboard/download install vẫn giữ nguyên trong khi PTY hooks đã có thể override riêng
    - `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs` cũng đã chuyển sang explicit `LocalSpawnHooks::default()` ở local/serial fallback constructors để desktop boundary dùng đúng seam mới
    - verify gate xanh:
      - `cargo check -p chatminal-host-runtime`
      - `cargo check -p chatminal-desktop`
      - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - host-runtime queued cleanup paths now use best-effort mux access after shutdown instead of strict `Mux::get()` deref, so shutdown no longer races a panic in the queued main-thread cleanup task
  - `initialize_host_runtime()` is now idempotent when the singleton already exists; repeated init reuses the current `MuxHandle` and only refreshes the primary spawn target instead of split-brain replacing live mux state
  - `chatminal-host-runtime/src/lib.rs` now groups `primary_spawn_target`, subscribers, and client/identity/focus metadata under `HostRuntimeControlPlane`, reducing direct control-plane state scatter inside `Mux`
  - init/bootstrap callers now only receive a narrow `MuxHandle` surface (`register_client`, `replace_identity`, `set_active_workspace`, `subscribe`) instead of broad direct mux access
  - focused-pane host boundary no longer leaks raw tuples:
    - `resolve_focused_pane(...)` now returns `FocusedPaneBinding` instead of `(TabId, PaneId)`
    - `ClientInfo.focused_pane_id` is now private behind crate-local accessor methods
    - desktop `session_host.rs` consumes the DTO and converts to `FrontendFocusedPane` at the desktop boundary
  - control-plane free-function surface in `lib.rs` was tightened one notch further:
    - added `with_control_plane(...)` plus narrower root-access helper layer
    - public helpers for identity/workspace/focus/spawn-target now read or mutate `HostRuntimeControlPlane` trực tiếp hơn thay vì routing qua broader `Mux` wrapper methods
    - dead `impl Mux` wrappers for those paths were removed
  - `window.rs` now snapshots `switch_to_last_active_tab_when_closing_tab` at construction time and publishes root-window notifications through host-runtime helpers instead of inline mux access
  - `WindowRef` no longer resolves raw mux/root-window guards inline for common getters/setters/session listing; the root-window path now stays on `LuaBridgeHost`
  - `TerminalRef` has begun 03F hardening:
    - tuple field is no longer constructed directly outside the bridge
    - desktop callsites now use `TerminalRef::from_pane_id(...)`
  - `tab.rs` boundary surface was trimmed one notch further:
    - dead `kill_pane` API removed
    - `remove_pane` narrowed to crate scope
  - root-window control paths were tightened another notch:
    - `chatminal-host-runtime/src/window.rs` now snapshots `switch_to_last_active_tab_when_closing_tab` at `Window::new(...)` time instead of reading config during tab-removal flow
    - window title/workspace invalidation now publishes through `notify_mux(...)` helper wrappers instead of inline `Mux` access
  - Lua bridge workspace/session error handling was tightened:
    - `chatminal-lua-bridge/src/lib.rs` now propagates lookup failures through `mlua::Result` for `active_workspace`, `workspace_names`, `set_active_workspace`, `rename_workspace`, `root_tabs`, and `panes` instead of silently defaulting to empty values
    - `chatminal-lua-bridge/src/leaf.rs` and `session.rs` now expose helper accessors (`TerminalRef::from_pane_id`, `TerminalRef::pane_id`, `SessionRef::as_str`, `SessionRef::to_owned_id`) so the bridge can stop leaking raw tuple-style host ids at the public boundary
  - Lua bridge `WindowRef` surface no longer resolves the root window through raw mux guards for common operations:
    - workspace/title getters and setters now flow through `LuaBridgeHost::with_root_window(...)` / `with_root_window_mut(...)`
    - `sessions_with_info()` / `active_session_id()` now use owned session-id helpers instead of touching `SessionRef` internals directly
    - `active_terminal()` now constructs `TerminalRef` via `TerminalRef::from_pane_id(...)`
  - Lua bridge terminal-handle boundary was tightened one notch further:
    - `TerminalRef` tuple field is now private
    - desktop-side Lua entrypoints now use `TerminalRef::pane_id()` instead of reading the raw tuple field
  - desktop typed-id boundary was tightened further without touching core numeric internals:
    - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` narrowed more compat re-exports from `pub` to `pub(crate)`
    - `FrontendResolvedPane` now carries `RuntimeId` instead of `u64`
    - `FrontendFocusedPane` now carries `RuntimeId` + `SessionTerminalHandle` instead of raw `u64` fields
    - `frontend_resolve_pane(...)` and `focus_terminal_handle_by_id(...)` now use `SessionTerminalHandle` at the desktop boundary
    - `frontend.rs` and `overlay/copy.rs` now consume those typed values directly instead of widening them back to raw ids at call boundaries
    - pane-centric `RuntimeNotification` payloads now also use `SessionTerminalHandle` at the desktop boundary for:
      - `PaneOutput`
      - `PaneAdded`
      - `PaneRemoved`
      - `PaneFocused`
      - `Alert.pane_id`
      - `AssignClipboard.pane_id`
    - tab/render-scope desktop notification payloads now also use `RuntimeId` for:
      - `TabAddedToWindow`
      - `TabResized`
      - `TabTitleChanged.runtime_id`
    - `session_host.rs` now stores `session_tab_shim` as `HashMap<String, RuntimeId>` and keeps host-tab lookup/remove/focus helper flow on `RuntimeId` until the final host-runtime conversion point
  - host/runtime typed boundary tightened one notch further on terminal-handle helpers:
    - `chatminal-host-runtime/src/lib.rs` now exposes `terminal_by_handle(...)`, `remove_terminal_handle(...)`, and `record_focus_for_terminal_handle(...)`
    - desktop `session_host.rs` / `desktop_host_runtime/mod.rs` now consume those typed helpers for pane lookup/remove/focus-record fallback instead of raw pane-id helpers on the same slice
    - `chatminal-lua-bridge/src/lib.rs` now resolves pane lookup and root-tab lookup through `SessionTerminalHandle`-based helpers
    - `chatminal-lua-bridge/src/leaf.rs` now exposes `TerminalRef::terminal_handle()` for that bridge boundary
    - once those callsites moved, `terminal_by_id(...)`, `tab_by_id(...)`, `resolve_pane_id(...)`, and `focus_pane_and_tab(...)` were narrowed back to crate scope instead of staying public cross-crate helpers on this slice
    - another dead/raw helper slice was then collapsed:
      - deleted `runtime_entry_by_id(...)`
      - deleted `has_tab(...)`
      - inlined old one-hop internal wrappers behind `root_active_runtime_id(...)`, `remove_terminal_handle(...)`, and `record_focus_for_terminal_handle(...)`
    - one more raw/public edge was trimmed:
      - `remove_tab_by_id(TabId)` is now crate scope only
      - Lua bridge session lookup now consumes `runtime_entry_by_session_id(...)` instead of the older `tab_by_chatminal_session_id(...)` helper name
    - execution-path public helpers were tightened too:
      - `spawn_tab(...)` now accepts `Option<SessionTerminalHandle>` instead of `Option<PaneId>`
      - `split_pane(...)` now accepts `SessionTerminalHandle` instead of `PaneId`
      - desktop `session_host.rs` and Lua bridge spawn/split paths were migrated to the typed handle boundary
  - root-tab/runtime-id boundary tightened one notch further:
    - `chatminal-host-runtime/src/tab.rs` now exposes `Tab::runtime_id()`
    - `chatminal-host-runtime/src/lib.rs` now exposes `root_runtime_ids()`
    - `chatminal-host-runtime/src/lib.rs` also now exposes a read-only `RuntimeEntryInfo` DTO plus `runtime_entry_info_by_runtime_id(...)` / `root_runtime_entry_infos(...)`
    - `chatminal-host-runtime/src/lib.rs` now also exposes `runtime_entry_info_by_session_id(...)` for the same read-only cut on session-id keyed queries
    - `chatminal-host-runtime/src/lib.rs` now exposes `terminal_handle_for_pane(&dyn Pane)` so boundary callers can stop re-wrapping `pane_id()` manually at the first typed-handle slices
    - `chatminal-lua-bridge/src/lib.rs` changed `RootTabRef` from raw `usize` to `RuntimeId`, so root-tab activation/lookup no longer round-trips through `usize`
    - `chatminal-lua-bridge/src/window.rs` and the session-module root listing helpers now read root-window session metadata through `RuntimeEntryInfo` instead of iterating concrete `Arc<Tab>` values for these read-only queries
    - Lua read-only session queries (`session_active_terminal_instance_id`, `session_title`, `active_terminal_for_session`, `session_size`) now also read through `RuntimeEntryInfo` instead of hydrating a concrete `Arc<Tab>` first
    - desktop callsites that were only reading/rendering runtime ids now use `tab.runtime_id()` (`main.rs`, `termwindow/mod.rs`, `desktop_termwindow_close_helpers.rs`, `session_host.rs`) instead of widening `tab.tab_id() as u64`
    - desktop root-window tab iteration that only needed ids/sizing now starts from `root_runtime_ids()` and only rehydrates concrete tabs where behavior still needs `Arc<Tab>`
    - desktop `host_render_scope_size(...)` also switched to the DTO path on its legacy-host fallback, so one more read-only desktop helper no longer needs `Arc<Tab>` on that slice
    - first desktop/Lua helper slices that only needed typed pane handles now call `terminal_handle_for_pane(...)` instead of constructing `SessionTerminalHandle::new(pane.pane_id() as u64)` inline
  - verify gate stayed green after this slice too:
    - `cargo check -p chatminal-host-runtime`
    - `cargo check -p chatminal-lua-bridge`
    - `cargo check -p chatminal-desktop`
    - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
    - notification/control subscriber boundary was tightened:
      - `MuxHandle::subscribe(...)` now delivers `HostRuntimeNotification`
      - desktop bridge converts from that typed host notification instead of raw `MuxNotification`
      - raw `MuxNotification` is now internal to `chatminal-host-runtime` on this slice
    - dead desktop raw-focus compat wrappers on that slice were removed once the typed path went green
  - another safe boundary cut landed after that:
    - `register_pane_with_output_callback(...)` and the full PTY output callback path now use `SessionTerminalHandle` instead of raw `usize`
    - root-window common reads-writes now have explicit helpers in `chatminal-host-runtime`:
      - `root_window_workspace_name(...)`
      - `root_window_title(...)`
      - `set_root_window_workspace_name(...)`
      - `set_root_window_title(...)`
      - `focus_root_runtime_entry(...)`
    - first desktop/Lua callsites were migrated to those helpers, reducing a slice of direct `with_root_window_mut(...)` use
    - then one more Lua slice moved:
      - `WindowRef.sessions`
      - `WindowRef.sessions_with_info`
      - `WindowRef.active_session`
      - `WindowRef.active_session_id`
      - `WindowRef.active_terminal`
      - `LuaBridgeHost::root_tabs`
      - `LuaBridgeHost::root_window_spawn_context`
      now route through root-runtime helper paths instead of direct root-window iteration closures
    - Lua bridge product code no longer has active `with_root_window(...)` / `with_root_window_result(...)` callsites on this slice
  - 03G started for real after audit:
    - introduced internal `PtyIoDispatcher` so PTY pipeline side effects are no longer hard-coded inline at every output/error/cleanup site
    - `send_actions_to_mux(...)`, `parse_buffered_data(...)`, and `read_from_pane_pty(...)` now consume that dispatcher
    - split pane registration from PTY reader startup with `register_pane_internal(...)` and `start_pane_pty_reader(...)`
    - socketpair inline-error path no longer calls `localpane::emit_output_for_pane(...)` directly from `read_from_pane_pty(...)`; both paths now share one inline-output helper
    - default output fallback and default exit-cleanup fallback were then pulled into dedicated helpers too, so dispatcher setup no longer hard-codes those branches inline
    - PTY parser/read loop cluster was then extracted out of `chatminal-host-runtime/src/lib.rs` into `chatminal-host-runtime/src/pty_io.rs`, leaving `lib.rs` with registry/lifecycle ownership but not parser implementation details
    - `localpane.rs` then received a first hook seam via `LocalPaneHooks`, so `record_input`, `alert`, `inline output`, and child-exit cleanup no longer call singleton-backed helpers inline from each `LocalPane` path
    - `pty_io.rs` then widened its internal seam from output-only override to a full `PtyIoHooks` contract, so future runtime owners can inject cleanup and inline-error handling too without reopening the parser module
    - in parallel, the session-native PTY reader/waiter loop also received a first explicit owner seam: `TerminalInstanceRuntime::spawn(...)` now delegates through `TerminalInstanceRuntimeHooks`, and `leaf_runtime_threads.rs` no longer hard-codes direct channel sends for output/error/exit inside the loop body
    - default behavior remains intentionally Mux-backed for compat/non-session paths, so this is a preparatory cut, not final cutover
- 2026-04-01:
  - Added `chatminal-runtime::RuntimeHost`
  - Added boundary DTOs `RuntimeTerminalSize` and `RuntimeHostSessionState`
  - `DesktopSessionHost` now implements `RuntimeHost`
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` execution-path helpers now use `Arc<dyn RuntimeHost>` for:
    - runtime id resolution
    - ensure/focus/hydrate/resize/close runtime operations
    - terminal-instance focus routing
  - Expanded `RuntimeHost` with terminal-handle binding/focus boundary:
    - added `RuntimeHostTerminalBinding`
    - added `terminal_binding_for_handle()`
    - added `focus_terminal_handle()`
  - Session-mode desktop helpers now route through `RuntimeHost` first for:
    - terminal handle focus
    - terminal handle lookup/binding
    - render-scope existence check for session-native runtime ids
    - public pane resolution before falling back to legacy `Mux` lookup
  - Session-mode close/focus notification helpers now route less through raw `Mux`:
    - `remove_runtime_entry_scope()` now closes session render targets through Chatminal runtime semantics before legacy `Mux` fallback
    - `frontend_resolve_pane()` and `frontend_resolve_focused_pane()` now resolve active session-native panes via Chatminal runtime binding first
  - Workspace/frontend control-plane wrappers were moved one layer deeper into `DesktopSessionHost`:
    - host workspace get/set
    - notification subscription
    - active frontend client
    - active workspace per client
    - workspace empty/list queries
  - Spawn/bootstrap lifecycle wrappers were also moved one layer deeper into `DesktopSessionHost`:
    - local shell runner spawn
    - host runtime entry spawn
    - primary spawn target get/set
    - host mux bootstrap/shutdown
    - serial spawn-target creation
  - Host-window query/activation wrappers now also delegate through `DesktopSessionHost` first:
    - root-window render-scope existence fallback
    - resolved window title
    - active runtime entry size
    - launcher session snapshot
    - root-window runtime-entry activation fallback
  - Host fallback wrappers for pane/focus/frontend lookup were moved down into `DesktopSessionHost`:
    - active render-scope fallback lookup
    - remove pane / remove runtime-entry tab
    - record focus for current identity
    - fallback pane focus in host window
    - fallback public-pane resolution
    - fallback frontend pane resolution / focused-pane resolution
    - panes-in-workspace query used by startup/spawn decision
  - Window/workspace wrapper closures now also route through `DesktopSessionHost` first:
    - `with_host_window`
    - `with_host_window_mut`
    - host-window existence
    - workspace-has-window lookup
    - resize-all-tabs on host window
  - `desktop_host_runtime/mod.rs` no longer contains any direct `Mux::get()` / `Mux::try_get()` calls; raw host primitive access for desktop app is now pushed down into:
    - `desktop_host_runtime/session_host.rs`
    - `desktop_host_runtime/session_pane.rs`
  - `desktop_host_runtime/session_host.rs` control-plane layer was cleaned up:
    - instance methods now thin-wrap the corresponding `legacy_*` helper instead of duplicating raw host logic inline
    - removed dead `set_host_workspace()` instance method after the delegation cutover
    - verification is clean again with no warnings from this slice
  - `desktop_host_runtime/session_host.rs` runtime fallback layer was tightened again:
    - direct `HostMux::get()` / `HostMux::try_get()` usage is now localized to a small helper block near the top of the file
    - reconcile/runtime-sync/tab-shim/resource-cleanup methods now consume shared typed host helpers instead of touching `HostMux` directly
  - entrypoint lifecycle facade started to narrow:
    - `chatminal_runtime/mod.rs` now exposes explicit desktop host lifecycle helpers for init/shutdown/serial target creation
    - `main.rs` now uses those explicit helpers instead of implicitly relying on the broad `desktop_host_runtime::*` re-export surface
  - wildcard host re-export is now gone from `chatminal_runtime/mod.rs`:
    - the desktop host bridge surface is now listed explicitly
    - conversion into real facade families has started instead of relying on implicit module leakage
  - first real facade families now live in `chatminal_runtime/mod.rs`:
    - spawn/bootstrap helpers
    - window/shell helpers
    - pane/render-target compat helpers
    - frontend/workspace helpers
  - caller migration has started for those families:
    - `frontend.rs`
    - `desktop_spawn.rs`
    - `desktop_termwindow_host_runtime_helpers.rs`
    - `desktop_termwindow_close_helpers.rs`
    - `desktop_termwindow_positioned_session_helpers.rs`
    - `overlay/confirm_close_pane.rs`
    - `overlay/launcher.rs`
    - `overlay/copy.rs`
    - `termwindow/resize.rs`
    - `termwindow/render/paint.rs`
  - residual host bridge explicit re-exports in `chatminal_runtime/mod.rs` are now down to 14 items
  - `desktop_host_runtime/session_pane.rs` raw host hooks were tightened:
    - repeated `PaneOutput` notifications now go through a single helper
    - repeated `record_input_for_current_identity()` calls now go through a single helper
    - `HostMux::get()` is now gone from that file; only one tiny `try_get()` helper remains
  - residual host bridge surface in `chatminal_runtime/mod.rs` tightened further:
    - broad desktop-host re-export cluster replaced by explicit host type aliases
    - `overlay_compat` is now a curated facade rather than a whole-module bridge leak
  - `termwindow/mod.rs` Cut 2A is now complete:
    - session-native layout refresh now routes through local helper methods
    - render-target close/focus lookup now reuses `desktop_termwindow_host_runtime_helpers.rs` helpers instead of manually composing bridge lookups inline
    - active-session reconcile no longer matches `DesktopSessionBridgeAction` inline in `termwindow/mod.rs`; that bridge detail moved into helper-family code
    - host closeability fallback is now hidden behind `render_scope_can_close_without_prompting_via_host()`
  - verification after Cut 2A:
    - `cargo check -p chatminal-desktop` pass
    - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass (`83/83`)
  - 03C Scope A implementation path is now concrete:
    - keep `Mux` ownership for pane/tab/window registry unchanged for now
    - cut at desktop host boundary only
    - target slice:
      1. desktop-owned `RuntimeNotification` enum + hub
      2. bridge `MuxNotification -> RuntimeNotification`
      3. `add_pane_without_default_side_effects(...)` in host-runtime
      4. desktop-owned clipboard/download bridges for session-native panes
      5. `session_pane.rs` pane-output emit path off `HostMux::notify_from_any_thread(...)`
  - 03C Scope A is now implemented:
    - `crates/chatminal-host-runtime/src/lib.rs`:
      - added `add_pane_without_default_side_effects(...)`
      - kept `add_pane(...)` as the legacy/default-side-effect path
    - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`:
      - replaced `RuntimeNotification = MuxNotification` alias with a desktop-owned enum
      - added desktop notification hub + publish/subscribe helpers
      - mux bootstrap now bridges legacy mux notifications into the desktop notification hub
    - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`:
      - session-native panes now install `DesktopClipboardBridge` / `DesktopDownloadBridge`
      - session-native pane registration now uses `add_pane_without_default_side_effects(...)`
      - desktop subscription path now points at the desktop notification hub
    - `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`:
      - `PaneOutput` now publishes through the desktop notification hub
    - scheduler-free/unit-test fallback added for publish-on-main-thread helper so tests do not panic when no GUI scheduler is configured
  - verification after 03C Scope A:
    - `cargo check -p chatminal-desktop` pass
    - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass (`83/83`)
  - 03C Scope B is now implemented:
    - `crates/chatminal-host-runtime/src/lib.rs`:
      - removed global `chatminal_session_id_index`
      - `get_tab_by_chatminal_session_id(...)` now resolves by scanning tab/pane metadata
      - pane add/remove paths no longer maintain a global reverse index
    - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`:
      - `ensure_mux_tab_shim()` no longer falls back to mux-global session-id lookup
      - desktop ownership now relies on `session_tab_shim` only
    - `crates/chatminal-lua-bridge/src/session.rs`:
      - compat comment updated to reflect scan-based resolution semantics
  - verification after 03C Scope B:
    - `cargo check -p chatminal-desktop` pass
    - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass (`83/83`)
  - 03C Scope C has now started with the low-risk read-path slice:
    - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` now exposes host-local pane lookup by terminal handle and by public id
    - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` now serves `terminal_handle_arc(...)` and `terminal_handle_arc_by_public_id(...)` from `DesktopSessionHost` first, before falling back to mux-global pane registry
    - this intentionally avoids touching focus/tab/window semantics yet
  - verification after the first 03C Scope C slice:
    - `cargo check -p chatminal-desktop` pass
    - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass (`83/83`)
  - 03C Scope C read-path slice was tightened again:
    - `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs` no longer carries the dead `with_live_host_mux(...)` helper; direct host-mux access is gone from that file
    - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` now exposes `find_registered_pane(...)`, `terminal_binding_for_public_id(...)`, and related lookup helpers on top of the canonical `DesktopSessionHost.panes` registry
    - `resolve_public_pane_fallback(...)` now reuses `DesktopSessionHost` local pane registry before falling back to legacy non-session/overlay lookup
    - `frontend_resolve_pane_fallback(...)` now reuses `DesktopSessionHost` local terminal binding before falling back to legacy pane-id -> tab-id resolution
    - `frontend_resolve_focused_pane_fallback(...)` now also rebinds legacy focused-pane results back through session-native bindings before returning to desktop callers
    - `desktop_host_runtime/mod.rs` now delegates the session-native read path to `DesktopSessionHost` instead of stitching binding + fallback inline in the module facade
    - `remove_terminal_handle(...)` now strips session-native panes out of desktop-local pane/session/runtime indexes first, tears down the shim tab when applicable, and only then falls back to legacy mux removal for non-session/overlay panes
    - `host_remove_pane(...)` now uses best-effort `try_host_mux()` so local prune paths do not hard-panic when mux bootstrap is absent
    - this slice still intentionally stopped short of `focus/tab/window` ownership cutover
  - verification after the tightened 03C Scope C slice:
    - `cargo check --workspace` pass
    - `cargo check -p chatminal-desktop` pass
    - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass (`87/87`)
    - direct regression coverage now includes:
      - `terminal_binding_for_public_id_resolves_handle_and_terminal_instance`
      - `frontend_resolve_pane_fallback_uses_local_registry_before_mux`
      - `remove_terminal_handle_prunes_local_registry_before_legacy_fallback`
      - `reconcile_visible_sessions_prunes_stale_session_indexes`
  - current residual counts after Scope B + current Scope C slices:
    - desktop app direct `HostMux::get()` / `HostMux::try_get()` / `Mux::get()` / `Mux::try_get()` references: 0
    - lua bridge direct `get_mux()` / `Mux::get()` / `Mux::try_get()` references: 0
    - lua bridge mux-backed compat is now centralized into the `LuaBridgeHost` wrapper in `lib.rs`
  - 03D accessor cut has now started:
    - `chatminal-host-runtime` exposes global accessor helpers such as `global_mux()`, `try_global_mux()`, `root_active_tab_id()`, `remove_pane_by_id()`, `add_pane_without_default_side_effects_to_global_mux()`, `spawn_tab_in_global_mux()`
    - `apps/chatminal-desktop` no longer calls the singleton accessor directly; the remaining mux-backed desktop helper paths now route through `chatminal-host-runtime`
  - 03E has now started with a narrow bridge host:
    - `crates/chatminal-lua-bridge/src/lib.rs` now owns a `LuaBridgeHost` wrapper around the current runtime backend
    - `SessionRef`, `TerminalRef`, and `WindowRef` now resolve via `LuaBridgeHost` instead of taking `Arc<Mux>` directly
    - workspace/session/terminal/window Lua methods now call `get_host()` rather than scattering `get_mux()` / `Mux::get()` throughout the crate
    - `TerminalRef` now stores `usize` directly rather than exposing `PaneId` in its public struct shape
    - `LuaBridgeHost::global()` now acquires the backend via `host_runtime::try_global_mux()`, so bridge modules themselves no longer call the mux singleton accessor directly
  - 03E window/workspace metadata cut is now complete:
    - `LuaBridgeHost` no longer exposes `root_window()` / `root_window_mut()` guard-returning APIs
    - root-window access in the bridge now goes through closure/value-based helpers built on `host_runtime::with_root_window(...)`
    - `WindowRef` no longer exports `resolve()` / `resolve_mut()` guard paths
    - `WindowRef` workspace/title/session listing/active selection methods now evaluate directly inside host-owned closures
    - `SpawnSession::spawn()` now snapshots the root-window size + active pane without holding a leaked lock guard across the bridge call
  - 03E session/tab lookup + activation cut is now complete:
    - `LuaBridgeHost` now retains a stable `Arc<Mux>` for the lifetime of each Lua call instead of probing the global mux and discarding it
    - `session.all_terminals()` keeps compat semantics by reading the mux pane registry, not only root-window tabs
    - `SessionRef` common operations now dispatch through `LuaBridgeHost`:
      - active terminal instance lookup
      - root-window resolution for the session
      - title get/set
      - active terminal + terminal listing + directional lookup
      - zoom + size + activation
    - `session.rs` no longer resolves `Tab` directly for the common `SessionRef` surface; that lookup is centralized back in `LuaBridgeHost`
  - 03E pane lookup + pane metadata/query cut is now complete:
    - `TerminalRef` no longer resolves panes directly from the mux for the common terminal surface
    - `LuaBridgeHost` now owns generic `with_pane(...)` / `with_pane_result(...)` helpers for stable pane lookup inside a single Lua call
    - `leaf.rs` common terminal metadata/query paths now dispatch through those host-owned pane helpers:
      - terminal/session id
      - paste/text send
      - title/progress/cwd/metadata/process info/cursor/dimensions/user vars
      - unseen-output / alt-screen state
      - viewport/logical scrollback text + escape extraction
      - semantic zones / semantic-zone text extraction
      - tty lookup + activation
    - terminal activation is now centralized in `LuaBridgeHost::activate_terminal(...)`
    - latent compatibility bug fixed during this cut: session `rotate_clockwise()` now calls the clockwise host-runtime API instead of rotating counter-clockwise
  - 03E spawn/split hardening cut is now complete:
    - root-window spawn context derivation now lives in `LuaBridgeHost::root_window_spawn_context()`
    - `SpawnSession::spawn()` now delegates runtime spawn fully through `LuaBridgeHost::spawn_session_from_root_window(...)`
    - `SplitSession::run()` now delegates runtime split fully through `LuaBridgeHost::split_terminal(...)`
  - 03C prep audit is now available and points to the practical extraction order:
    1. notification + clipboard/download
    2. deprecated session reverse index
    3. pane registry
    4. tab registry + root window
    5. client/workspace/focus + spawn target
  - updated conclusion for the next safe slices:
    - `03D`: move `tab/root-window/focus` ownership itself, not just accessor calls, further out of mux-backed helper paths
    - `03E`: keep replacing mux-backed bridge clusters behind `LuaBridgeHost` incrementally:
      1. workspace/window metadata reads-writes [done]
      2. session/tab lookup and activation [done]
      3. pane lookup + pane metadata/query methods [done]
      4. spawn/split operations hardening [done]
  - Render-target/window activation wrappers now understand session-native render targets before legacy `Mux` fallback:
    - `host_active_render_scope_id()`
    - `host_render_scope_size()`
    - `activate_host_runtime_entry()`
  - Verification gate quality improved:
    - fixed canonical scrollback retention query in `chatminal-store`
    - serialized `session_pane` host-mux tests to remove global `Mux` parallel flake
  - `cargo check -p chatminal-runtime` pass
  - `cargo check -p chatminal-desktop` pass
  - `cargo test --workspace --lib --bins --tests -- --test-threads=1` pass
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` pass
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml` pass
- Historical open items before 2026-04-03 closeout:
  - Mux singleton still exists
  - `DesktopSessionHost` control-plane wrappers still use `Mux` internally
  - overlay/render-scope host primitives still live behind desktop host adapter types
  - `session_host.rs` still contains both instance methods and `legacy_*` fallback helpers; public/legacy facade consolidation is still open even though raw host access is now localized
  - `chatminal_runtime/mod.rs` no longer re-exports `desktop_host_runtime::*` broadly, but 14 residual host bridge items are still explicit re-export rather than full facade wrappers
  - overlay/non-session compat paths still fall back to `Mux`
