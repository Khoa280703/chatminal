# Phase 03C prep audit: Mux ownership + extraction order

Context:
- Work context: `/Users/khoa2807/development/2026/chatminal`
- Scope chỉ audit:
  - [crates/chatminal-host-runtime/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
  - [apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs)
  - [apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs)

## Executive summary
- `DesktopSessionHost` đã kéo một phần ownership ra khỏi `Mux`: session-native pane maps, session->pane, session->tab shim, runtime render snapshot, runtime->terminal instances, runtime size cache.
- `Mux` vẫn còn giữ 6 nhóm ownership lớn:
  1. engine pane registry
  2. engine tab registry
  3. root window
  4. notification fanout + clipboard/download dispatch
  5. client identity/workspace/focus bookkeeping
  6. spawn target + PTY ingestion pipeline
- 03C nên tách theo thứ tự: notification/clipboard/download trước, registry/session index kế tiếp, root window/tab ownership sau, workspace/client/spawn cuối.
- Nếu làm ngược, diff sẽ chồng chéo và dễ gãy behavior ở session-native render path.

## Ownership đã ở ngoài Mux
Trong [session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs#L431):
- `panes`: `terminal_instance_id -> Arc<ChatminalSessionPane>`
- `session_pane`: `session_id -> pane`
- `session_tab_shim`: `session_id -> mux tab_id`
- `runtime_render_state`: `runtime_id -> ChatminalRenderState`
- `runtime_terminal_instances`: `runtime_id -> set<terminal_instance_id>`
- `runtime_terminal_size`: `runtime_id -> TerminalSize`

Các maps này được dùng trong:
- runtime sync/render hydration: [session_host.rs:735](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:735)
- stale-session reconcile: [session_host.rs:694](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:694)
- tab shim attach/replace: [session_host.rs:899](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:899)
- runtime resource cleanup: [session_host.rs:992](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:992)

## Ownership còn nằm trong Mux
Trong [lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs#L94):

### 1. Pane storage
- `panes: RwLock<HashMap<PaneId, Arc<dyn Pane>>>` [lib.rs:95](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:95)
- API liên quan:
  - `get_pane()` [lib.rs:672](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:672)
  - `add_pane()` [lib.rs:688](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:688)
  - `remove_pane()` [lib.rs:794](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:794)
  - `iter_panes()` [lib.rs:897](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:897)

### 2. Tab storage
- `tabs: RwLock<HashMap<TabId, Arc<Tab>>>` [lib.rs:94](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:94)
- API liên quan:
  - `get_tab()` [lib.rs:676](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:676)
  - `add_tab_no_panes()` [lib.rs:725](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:725)
  - `add_tab_and_active_pane()` [lib.rs:730](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:730)
  - `remove_tab()` [lib.rs:799](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:799)
  - `resolve_pane_id()` [lib.rs:903](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:903)

### 3. Root window ownership
- `window: RwLock<Window>` [lib.rs:96](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:96)
- API liên quan:
  - `root_window()` [lib.rs:854](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:854)
  - `root_window_mut()` [lib.rs:858](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:858)
  - `root_active_tab()` [lib.rs:862](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:862)
  - `attach_tab()` [lib.rs:867](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:867)
  - `prune_dead_windows()` [lib.rs:805](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:805)

### 4. Session reverse index
- `chatminal_session_id_index: RwLock<HashMap<String, PaneId>>` [lib.rs:111](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:111)
- Dùng ở:
  - `get_tab_by_chatminal_session_id()` [lib.rs:682](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:682)
  - insert/remove trong `add_pane()` và `remove_pane_internal()` [lib.rs:702](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:702), [lib.rs:742](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:742)
- Ghi chú trong source đã nói đây là deprecated global index, chỉ giữ cho compat bridge.

### 5. Notification + clipboard/download ownership
- `subscribers` map [lib.rs:99](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:99)
- `notify()` / `notify_from_any_thread()` [lib.rs:624](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:624)
- `MuxClipboard` gửi `AssignClipboard` qua Mux [lib.rs:1116](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:1116)
- `MuxDownloader` gửi `SaveToDownloads` qua Mux [lib.rs:1132](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:1132)
- PTY ingestion `send_actions_to_mux()` cũng đang publish `PaneOutput` qua Mux [lib.rs:118](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:118)

### 6. Client/workspace/focus ownership
- `clients`, `identity`, `num_panes_by_workspace` [lib.rs:100](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:100)
- API liên quan:
  - `record_input_for_current_identity()` [lib.rs:425](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:425)
  - `record_focus_for_current_identity()` [lib.rs:431](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:431)
  - `resolve_focused_pane()` [lib.rs:437](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:437)
  - `active_workspace*()` / `set_active_workspace*()` [lib.rs:529](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:529)
  - `subscribe()` [lib.rs:613](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:613)
  - `recompute_pane_count()` [lib.rs:403](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:403)

### 7. Spawn target ownership
- `primary_spawn_target` [lib.rs:98](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:98)
- API liên quan:
  - `primary_spawn_target()` [lib.rs:644](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:644)
  - `set_primary_spawn_target()` [lib.rs:652](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:652)
  - `resolve_spawn_target()` [lib.rs:925](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:925)
  - `spawn_tab()` [lib.rs:1026](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:1026)
  - `split_pane()` [lib.rs:951](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs:951)

## session_host.rs đang còn phụ thuộc Mux ở đâu
Trong [session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs):
- toàn bộ raw host access đã bị dồn vào helper layer đầu file [session_host.rs:48](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:48)
- phần còn thực sự phụ thuộc Mux semantics:
  - pane registration/unregistration cho render compat: `host_add_pane`, `host_remove_pane` [session_host.rs:61](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:61), [session_host.rs:57](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:57)
  - tab shim lookup/attach/replace/remove: `host_get_tab`, `host_get_tab_by_session_id`, `host_add_tab_and_active_pane`, `host_attach_tab`, `host_remove_tab` [session_host.rs:69](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:69) đến [session_host.rs:88](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:88)
  - root window activation/path query: `with_host_window_ref`, `with_host_window_mut_ref`, `host_focus_root_window_tab` [session_host.rs:90](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:90), [session_host.rs:177](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:177)
  - client/workspace/focus/notification wrappers: [session_host.rs:110](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:110) đến [session_host.rs:160](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:160)
  - spawn/bootstrap wrappers: [session_host.rs:169](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:169) đến [session_host.rs:205](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs:205)

## session_pane.rs đang còn phụ thuộc Mux ở đâu
Trong [session_pane.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs):
- chỉ còn 1 helper `with_live_host_mux()` dùng `HostMux::try_get()` [session_pane.rs:43](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs:43)
- `notify_pane_output()` đã đổi sang `HostMux::notify_from_any_thread()` [session_pane.rs:47](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs:47)
- `record_input_for_current_identity()` vẫn còn bám client-input bookkeeping của Mux [session_pane.rs:53](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs:53)

## Extraction order thực dụng nhất cho 03C

### Phase 03C-1: notification/clipboard/download tách khỏi Mux trước
Làm trước vì:
- ít đụng root window/tab layout
- giảm coupling cho `session_pane.rs`
- mở đường cho PTY pipeline 03G

Move ra khỏi Mux:
- `subscribers`
- `notify()` / `notify_from_any_thread()`
- `MuxClipboard`
- `MuxDownloader`
- `send_actions_to_mux()` publish path

Target mới:
- notification hub owned bởi desktop runtime/session host
- clipboard/download dispatch đi qua runtime host boundary thay vì Mux singleton

### Phase 03C-2: dẹp session reverse index global
Làm tiếp vì:
- `session_tab_shim` ở `DesktopSessionHost` đã thay thế phần desktop path
- global `chatminal_session_id_index` chỉ còn là compat debt

Move/replace:
- `get_tab_by_chatminal_session_id()` không nên lookup từ `Mux`
- desktop path dùng thẳng `session_tab_shim`
- nếu còn compat consumer khác, chuyển sang host adapter cục bộ hoặc runtime-host query hẹp

### Phase 03C-3: tách pane registry ownership
Làm sau notification/index vì:
- pane registry hiện còn kéo theo clipboard/download setup + PTY reader thread spawn
- đây là lớp có coupling cao nhất với output pipeline

Move ra khỏi Mux:
- `panes`
- `get_pane()` / `iter_panes()` / `add_pane()` / `remove_pane()`
- reader-thread hookup trong `add_pane()`

Target mới:
- `DesktopSessionHost` hoặc runtime-host impl giữ pane registry cho session-native panes
- host-runtime giữ minimal adapter cho legacy/overlay path nếu cần

### Phase 03C-4: tách tab registry + root window ownership
Làm sau pane registry vì:
- `tabs` và `window` phụ thuộc pane registry, active pane, prune logic
- đây là bước phá coupling lớn nhất

Move ra khỏi Mux:
- `tabs`
- `window`
- `root_window*()`
- `attach_tab()`
- `remove_tab()`
- `prune_dead_windows()`
- `resolve_pane_id()`

Target mới:
- runtime-host impl giữ session-native render target registry và root window attachment state
- `Mux` nếu còn tồn tại chỉ là compat shell, không giữ source of truth

### Phase 03C-5: tách client/workspace/spawn target cuối cùng
Làm cuối vì:
- ít block session-native rendering hơn
- nhưng đụng nhiều public surface và UX state

Move ra khỏi Mux:
- `clients`
- `identity`
- `num_panes_by_workspace`
- `primary_spawn_target`
- active workspace/focus bookkeeping

Target mới:
- runtime-host / frontend host context giữ identity + workspace + focus
- spawn target config nằm ở desktop runtime facade hoặc host bootstrap state riêng

## Write scopes có thể parallel hóa về sau

### Scope A: notification + clipboard/download
Files chính:
- [crates/chatminal-host-runtime/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
- [apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs)
- có thể thêm [apps/chatminal-desktop/src/chatminal_runtime/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs)

Ít conflict với pane/tab/window extraction nếu interface hẹp.

### Scope B: session index compat cleanup
Files chính:
- [crates/chatminal-host-runtime/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
- [apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs)
- khả năng đụng compat consumer khác ngoài scope này

Có thể chạy song song với Scope A nếu không cùng sửa notification path.

### Scope C: pane registry extraction
Files chính:
- [crates/chatminal-host-runtime/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
- [apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs)
- [apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs)

Không nên chạy song song với Scope D vì chồng mạnh vào lifecycle tab/window.

### Scope D: tab + root window extraction
Files chính:
- [crates/chatminal-host-runtime/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
- [apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs)
- có thể đụng `desktop_host_runtime/mod.rs` và caller window helpers ngoài scope hiện tại

Đây là critical scope. Không nên parallel write với Scope C.

### Scope E: client/workspace/spawn target extraction
Files chính:
- [crates/chatminal-host-runtime/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
- [apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs)
- [apps/chatminal-desktop/src/chatminal_runtime/mod.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_runtime/mod.rs)
- frontend caller files nếu API đổi

Có thể làm song song một phần với Scope A/B, nhưng không nên song song với Scope D nếu cùng chạm root window/workspace semantics.

## Kết luận thực dụng
- 03C nên mở bằng notification hub + clipboard/download trước. Đây là slice có payoff cao nhất và risk thấp nhất.
- Sau đó dẹp session reverse index global.
- Pane registry và tab/window ownership phải đi tuần tự, không nên tách 2 worker cùng sửa.
- Client/workspace/spawn target để cuối; đụng rộng nhưng ít chặn render path hơn.

## Unresolved
- Compat consumer cuối cùng của `chatminal_session_id_index` ngoài desktop path là chỗ nào chính xác ngoài ghi chú Lua bridge.
- Nếu kéo `notify_from_any_thread()` ra khỏi `Mux`, sẽ đặt main-thread dispatch primitive mới ở `chatminal-runtime` hay ở desktop host adapter.
