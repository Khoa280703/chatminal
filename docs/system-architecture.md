# System Architecture

Last updated: 2026-04-04 (terminal layer merge cutover)

## Latest changes (terminal layer merge cutover, 2026-04-04)
- Active product path đã collapse về một terminal architecture:
  - `chatminal-terminal-emulator` là terminal domain canonical
  - `chatminal-terminal-core` đã bị gỡ khỏi active workspace path
- Desktop session-native runtime không còn dual type contract:
  - `session_engine/*`
  - `session_host.rs`
  - `session_pane.rs`
  - `execution_bridge.rs`
  đều đã dùng `engine_term::TerminalSize`
- Active docs/README đã được sync để không còn mô tả runtime hiện tại như hai terminal layers song song.
- Target verify gate cho wave này:
  - `cargo check --workspace`
  - `cargo test --workspace --lib --bins --tests`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - `make window`

## Latest changes (startup ownership crash fix, 2026-04-04)
- `make window` crash startup trên macOS đã được sửa ở host-runtime ownership seam:
  - `HOST_RUNTIME_ROOT` giờ giữ `Arc<HostRuntimeRoot>` trực tiếp thay vì `Weak<HostRuntimeRoot>`
  - desktop startup không còn phụ thuộc `MuxHandle` being kept alive ở product seam để root tồn tại đủ lâu cho queued GUI startup work
  - panic `host runtime root must exist` trong `SpawnQueue::trigger` không còn tái hiện sau startup smoke
- Desktop fallback sai đã được bỏ:
  - `session_host.rs` không còn tự cài local spawn-target fallback khi host runtime chưa sẵn sàng
  - primary spawn target quay về contract strict: runtime phải được bootstrap đúng trước khi spawn
- Verify cho fix này:
  - `cargo check -p chatminal-host-runtime`
  - `cargo check -p chatminal-desktop`
  - `cargo test -p chatminal-host-runtime root_window_info_is_none_without_runtime -- --nocapture`
  - `cargo test -p chatminal-host-runtime spawn_runtime_entry_returns_runtime_info_without_exposing_tab -- --nocapture`
  - `make window` smoke launch

## Latest changes (post-unification follow-ups completed, 2026-04-03)
- Follow-up plan `260403-1800-post-unification-followups` đã hoàn tất:
  - `Phase 01` config ownership completion: done
  - `Phase 02` terminal crate rename sweep: done
- Terminal crate vocabulary đã được dọn sạch ở package/path/docs active scope:
  - `chatminal-engine-*` package/path names được rename sang `chatminal-terminal-*` / `chatminal-*`
  - `lib.name` và Cargo compatibility aliases `engine-*` vẫn được giữ để tránh churn import Rust hàng loạt
- Plan `260401-0949-architecture-unification` đã được đóng ở trạng thái `done`.
- Done-gate conclusion:
  - `HostRuntimeRoot` là ownership root của active product path; `Mux` chỉ còn là explicit compat facade
  - `with_mux(` / `with_mux_strict(` đã sạch trong code `crates/` + `apps/`
  - product path dùng `host_default()`; `mux_default(` chỉ còn là explicit compat alias/tests
  - public cross-crate boundary mục tiêu đã typed hóa bằng `RuntimeId` / `SessionTerminalHandle`; residual `PaneId` / `TabId` chỉ còn ở crate-local internals hoặc wire compatibility shapes
  - `configuration(` trong closeout scope chỉ còn ở `chatminal-config` foundation helpers và desktop test/comment paths
- Follow-up phase 01 config ownership completion:
  - product-path config reads đã được đẩy sang explicit ownership/snapshot path trong `chatminal-window`, `chatminal-terminal-font`, `chatminal-time-funcs`, và `chatminal-ratelim`
  - `configuration(` product reads now stay trong `chatminal-config` foundation helpers + test/comment seams; window/font/runtime hot paths dùng injected/connection-owned config thay vì singleton polling
- Historical deferred/out-of-scope cho current closeout đã được xử lý qua follow-up plan; phần rename/config ownership không còn pending active.
- Verification used for closeout:
  - `cargo check --workspace`
  - `cargo test --workspace --lib --bins --tests`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - `make window` bounded smoke launch

## Latest changes (HostRuntimeRoot ownership cut, 2026-04-03)
- `chatminal-host-runtime` đã cắt ownership thật ra khỏi `Mux` thêm một nhịp:
  - `HostRuntimeRoot` giờ là owner thật của `tabs`, `panes`, `window`, `control`, và workspace pane-count
  - tại thời điểm cut này global slot `HOST_RUNTIME_ROOT` chỉ giữ `Weak<HostRuntimeRoot>`
  - `Mux` chỉ còn là facade compat mỏng trỏ vào `Arc<HostRuntimeRoot>`
- Desktop bootstrap giữ owner mạnh ở product boundary:
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` giữ `Arc<MuxHandle>` suốt lifecycle bootstrap/shutdown
  - runtime root không còn sống nhờ static owner trong host-runtime
- Helper layer của host-runtime đã chuyển thêm từ facade sang root:
  - root-window/workspace/query/control helpers giờ resolve trực tiếp qua `HostRuntimeRoot`
  - `tab::prune_dead_panes()` kiểm tra pane registry từ root thay vì giữ dependency vào global mux facade
  - queued notify/clipboard/download paths cũng đã đi qua root lookup thay vì `Mux::get()`/`Mux::try_get()`
- Residual debt còn lại:
  - compat helper names `try_global_mux()` / `with_mux()` / `with_mux_strict()` vẫn tồn tại, nhưng chỉ materialize facade tạm từ root
  - mutation/lifecycle paths sâu hơn (`add/remove/focus/spawn/split`) và compat PTY default owner vẫn chưa rời hẳn Mux semantics
- Verify green:
  - `cargo check -p chatminal-host-runtime`
  - `cargo check -p chatminal-lua-bridge`
  - `cargo check -p chatminal-desktop`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `cargo test -p chatminal-lua-bridge`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`

## Latest changes (architecture unification Employee A follow-up, 2026-04-03)
- Host-runtime ownership/control helper layer bị cắt thêm khỏi singleton `Mux`:
- `HostRuntimeRoot` giờ là owner thật của `tabs/panes/window/control-plane`; ở nhịp follow-up này `HOST_RUNTIME_ROOT` từng chỉ giữ `Weak<HostRuntimeRoot>`
  - `Mux` chỉ còn là compat facade mỏng materialize tạm từ root khi caller cũ vẫn cần mutation/lifecycle methods
  - root/window/workspace/query helpers trong `chatminal-host-runtime/src/lib.rs` giờ resolve thẳng qua `HostRuntimeRoot` thay vì lấy global `Mux`
  - `tab.rs::prune_dead_panes()` cũng đã đọc pane registry từ root, không còn phụ thuộc vào global mux facade chỉ để check liveness
  - desktop seam từng giữ strong owner qua `MuxHandle`; startup crash ngày 2026-04-04 cho thấy seam này không đủ bền, nên ownership global đã được đổi lại về strong `Arc`
- Host-runtime control-plane metadata giữ typed handle sâu hơn:
  - `ClientInfo` giờ giữ focused terminal dưới `SessionTerminalHandle`
  - wire field vẫn là `focused_pane_id` để không gãy codec/protocol readers cũ
  - `FocusedPaneBinding` cũng giữ `RuntimeId` + `SessionTerminalHandle` nội bộ thay vì raw `TabId`/`PaneId`
- Config foundation có helper entry points hẹp hơn:
  - `chatminal-config` thêm `current_config_handle()`
  - `default_workspace_name_or(...)`
  - `current_initial_terminal_size()`
  - `current_output_parser_config()`
  - `current_exit_behavior()`
  - host-runtime foundation helpers giờ lấy config qua các entry point này thay vì mỗi chỗ tự kéo singleton reads
  - `TermConfig::enq_answerback()` cũng đã đi qua `self.configuration()` để injected config path không còn rơi về global singleton ở slice này
- Local spawn env reapply an toàn hơn ở host-runtime:
  - flatpak host path chỉ giữ `CHATMINAL_PANE`; không re-apply lại `CHATMINAL_UNIX_SOCKET` / `SSH_AUTH_SOCK` từ sandbox sau `fixup_command()`
  - local spawn path cũng ưu tiên `SSH_AUTH_SOCK` từ env hiện tại trước identity snapshot cũ
- Compile-fallout shim được cô lập khỏi host-runtime public raw ids:
  - `chatminal-codec` dùng local `usize` aliases cho `PaneId` / `TabId`, nên desktop compile không còn phụ thuộc public raw host ids
- Host-runtime đã có thêm capability surface hẹp để consumer cutover dần khỏi concrete `Tab`:
  - `runtime_entry_exists(...)`
  - `set_runtime_entry_title(...)`
  - `runtime_entry_terminal_handles(...)`
  - `runtime_entry_terminal_handle_in_direction(...)`
  - `set_runtime_entry_zoomed(...)`
  - nhóm helper này cũng có mirror theo `session_id` cho Lua/runtime consumer path
  - `rotate_runtime_entry_counter_clockwise(...)`
  - `rotate_runtime_entry_clockwise(...)`
  - `set_runtime_entry_active_terminal(...)`
  - `runtime_entry_terminal_infos(...)`
- Root-window/runtime-entry foundation được siết thêm một nhịp:
  - `RootWindowInfo` + `root_window_info()` / `root_last_active_runtime_id()` gom read-only metadata của root window vào DTO hẹp hơn
  - `create_attached_runtime_entry_for_terminal(...)` cho phép dựng/attach runtime entry từ `Pane` mà caller ngoài crate không phải tự cầm `Arc<Tab>` để làm bước khởi tạo đơn giản
  - `spawn_target.rs` đã dùng lại cùng builder path này để giữ semantics tạo root runtime entry ở một chỗ
  - thêm tiếp capability surface cho nhóm behavior thường khiến caller phải kéo concrete `Tab`:
    - zoom toggle
    - resize runtime entry
    - active-pane size adjust
    - active terminal handle getter
    - active terminal activation theo index/direction
    - swap active terminal
    - close-without-prompting check
    - split layout snapshot + split resize result
  - các helper này đều có path theo `RuntimeId` và mirror theo `session_id`
  - root-window read contract cũng được gom thêm:
    - `RootWindowInfo` giờ mang cả `initial_position`
    - có thêm `root_window_initial_position()`
    - `root_active_runtime_entry_info()`
    - `root_runtime_entry_summaries()`
    - root-window navigation/order slice giờ cũng có helper riêng:
      - `root_runtime_entry_count()`
      - `root_active_runtime_entry_index()`
      - `root_last_active_runtime_entry_index()`
      - `root_runtime_id_at_index()`
      - `root_runtime_entry_info_at_index()`
      - `focus_root_runtime_entry_index()`
      - `focus_root_last_runtime_entry()`
      - `focus_root_runtime_entry_relative()`
      - `move_root_active_runtime_entry_to_index()`
  - mục tiêu của lớp này là để phase desktop wiring sau có snapshot đủ giàu mà không cần giữ raw window guard cho các read paths phổ biến
  - thêm nhát order helper này để merge step sau có thể bóc dần các window-index mutation closures ở desktop fallback path mà chưa phải bỏ compat APIs cũ ngay
  - session-id/spawn foundation cũng hẹp hơn:
    - `runtime_id_for_session_id(...)` cho caller resolve runtime id mà không cần hydrate concrete tab
    - `focus_root_runtime_entry_by_session_id(...)` cho root activation path theo session id
    - `spawn_runtime_entry(...)` trả `RuntimeEntryInfo` cùng pane handle, giúp future consumer bớt nhận `Arc<Tab>` ở async spawn path
  - metadata lookup cũng được chuẩn hóa ở host-runtime:
    - `terminal_instance_id_for_pane(...)`
    - `terminal_by_terminal_instance_id(...)`
    - `terminal_by_public_id(...)`
    - `resolve_runtime_id_for_terminal_instance_id(...)`
  - lớp helper này nhắm vào việc bỏ các vòng scan pane metadata trùng lặp ở desktop/Lua compat paths
  - unit tests đụng global mux giờ dùng shared test lock để không race giữa root-window/control-plane cases

## Latest changes (architecture unification Employee D follow-up, 2026-04-03)
- Host-runtime local spawn path giữ runtime env explicit hơn:
  - `spawn_target.rs` re-apply `CHATMINAL_UNIX_SOCKET`, `CHATMINAL_PANE`, và `SSH_AUTH_SOCK` sau `fixup_command()`, nên exec-target fixup/env-clear paths không còn vô tình làm rơi startup env cần cho local PTY/session spawn
  - `SSH_AUTH_SOCK` trên local spawn path ưu tiên active-identity snapshot trước env hiện tại; flatpak host path chỉ giữ `CHATMINAL_PANE` để không kéo lại sandbox-only socket env
- PTY compat path dùng snapshot/callback rõ owner hơn:
  - `pty_io.rs` snapshot parser config + default exit behavior lúc start reader qua config helper foundation thay vì đọc singleton sâu hơn trong read/cleanup flow
  - `pty_io.rs` cũng giữ `output / cleanup / inline error` dưới dạng callback concrete, nên compat dispatcher không còn `unwrap_or_else` fallback ngầm về Mux ở read-loop layer
  - `PtyIoHooks::noop()` đã xuất hiện cho output-only registration path; custom output callback không còn vô tình kéo theo hidden inline-error/cleanup Mux semantics
  - `localpane.rs` + `localpane_hooks.rs` route child-exit cleanup qua explicit exit-behavior callback, nên compat localpane không còn chỉ nudge prune mù khi child waiter hoàn tất
  - `LocalPaneHooks::default()` giờ là no-op; Mux-backed compat semantics được dồn lên `LocalSpawnHooks::mux_default()` / `PtyIoHooks::mux_default()` ở spawn boundary
  - `LocalPane::new(...)` cũng không còn hard-code `mux_default()`, và `LocalSpawnHooks::noop()` cho phép chuẩn bị bundle non-Mux rõ ràng hơn ở host-runtime layer
  - `localpane.rs` constructor cũng đã dùng `current_config_handle()` cho snapshot khởi tạo
- Session-native PTY path giữ lifecycle owner rõ hơn:
  - `session_engine/leaf_runtime_threads.rs` đóng writer khi reader/waiter phase kết thúc
  - `session_engine_shared.rs` clear leaf process metadata khi nhận `TerminalInstanceExited`
  - session-engine tests hiện đã pass lại trong current tree (`session_engine` + `session_pane`)
- Focus metadata wire compatibility vẫn được giữ:
  - `ClientInfo` dùng `SessionTerminalHandle` nội bộ nhưng vẫn serialize field cũ `focused_pane_id`
  - deserialize cũng chấp nhận alias `focused_terminal_handle` để mixed-version readers không gãy
- Residual status:
  - `07A` chưa closed hoàn toàn: hidden fallback ở compat PTY path đã giảm thêm ở output-only/localpane constructor path, nhưng compat spawn boundary vẫn đang chọn Mux owner mặc định
  - `09B` đã giảm tiếp singleton reads trong scope host-runtime của Employee D, nhưng phase-level goal “đẩy `configuration()` ra khỏi các path còn scope được” vẫn chưa hoàn tất cho toàn lane

## Latest changes (architecture unification phase 01, 2026-04-01)
- `chatminal-config` không còn giữ dead config surface cho SSH/TLS/WSL:
  - removed config fields `wsl_targets`, `ssh_targets`, `ssh_backend`, `tls_servers`, `tls_clients`
  - removed stubs `mux_enable_ssh_agent`, `default_ssh_auth_sock`, `check_for_updates`, `show_update_window`, `check_for_updates_interval_seconds`
  - removed Lua helper `default_wsl_targets`
- `chatminal-host-runtime` không còn WSL spawn-target fixup branch trong active build.
- Root workspace không còn dependency `libssh-rs`.
- Chỉ giữ lại minimal SSH compile-path utilities trong `chatminal-config`:
  - `SshParameters`
  - `username_from_env()`
- Desktop shell startup đã unify:
  - sidebar luôn là một phần của desktop app
  - default startup path của `chatminal-desktop` đi thẳng vào unified desktop shell, không còn terminal-only fallback path ở entry mặc định

## Latest changes (architecture unification phase 04 follow-up, 2026-04-02)
- Desktop config boundary được kéo sát owner hơn:
  - `frontend.rs` nhận config snapshot lúc init và refresh lại khi config reload
  - `selection.rs` lấy `selection_word_boundary` từ config snapshot của desktop boundary, không còn tự kéo singleton trong word-selection hot path
  - `overlay/copy.rs` dùng palette từ `TermWindow.config` thay vì gọi global config trong render path
  - `colorease.rs` nhận `animation_fps` từ caller-owned config, nên animation scheduling không còn phụ thuộc global config singleton
  - `stats.rs` đọc `periodic_stat_logging` từ atomic được refresh bởi config subscription thay vì polling singleton trong background loop
  - `customglyph.rs` snapshot block anti-alias policy một lần cho mỗi draw path thay vì lặp singleton reads trong hot render branches
- Host-runtime config boundary cũng được siết thêm ở các flow low-risk:
  - `spawn_target.rs` lấy một config snapshot cho cả `build_command()` flow
  - `tab.rs` snapshot `unzoom_on_switch_pane` vào `TabInner`
  - `localpane.rs` gom exit/stateful-close checks về local config snapshot
  - `lib.rs` gom thêm `switch_to_last_active_tab_when_closing_tab()` và `unzoom_on_switch_pane()` helper boundaries cho constructor-time behavior flags
  - `window.rs` và `tab.rs` dùng các helper này thay vì đọc `configuration()` trực tiếp ở constructor
- Host-runtime lifecycle được harden thêm:
  - `initialize_host_runtime()` re-use runtime owner hiện có nếu host runtime đã boot, tránh split-brain re-init
  - cleanup chạy trên main thread giờ best-effort nếu mux đã shutdown trước khi task queue được thực thi
  - `HostRuntimeControlPlane` giờ gom `primary_spawn_target`, subscribers, client registry, active identity, workspace metadata, và per-client focus metadata ra khỏi phần registry logic của `Mux`
  - `chatminal-host-runtime/src/lib.rs` thêm helper control-plane/root access hẹp hơn để public/free-function helpers đụng owner gần hơn, thay vì phải đi vòng qua nhiều `Mux` wrapper methods
- Root-window boundary cũng được siết thêm:
  - `chatminal-host-runtime/src/window.rs` snapshot close-behavior config lúc tạo root window và route notifications qua helper wrappers
  - `chatminal-lua-bridge/src/window.rs` dùng closure-based root-window helpers cho workspace/title/session queries, nên không còn leak raw lock guards
  - `TerminalRef` không còn public tuple field ở bridge boundary
  - desktop-side Lua trigger path dùng `TerminalRef::pane_id()` thay vì đọc raw numeric field
- Focus boundary cũng được siết thêm:
  - `chatminal-host-runtime` trả `FocusedPaneBinding` thay vì raw `(TabId, PaneId)` tuple ở host boundary
  - `ClientInfo.focused_pane_id` không còn public field
  - desktop `session_host.rs` chuyển DTO này thành `FrontendFocusedPane` ngay tại desktop boundary
- Terminal-handle fallback boundary cũng được siết thêm:
  - `chatminal-host-runtime` giờ có typed helpers `terminal_by_handle(...)`, `remove_terminal_handle(...)`, và `record_focus_for_terminal_handle(...)`
  - desktop fallback path và Lua bridge pane/root-tab lookup đã chuyển thêm sang `SessionTerminalHandle` ở slice này
  - `LuaBridgeHost::tab_by_ref(...)` cũng đã chuyển sang `RuntimeId`-based host lookup, nên một cụm raw host helper cũ (`terminal_by_id`, `tab_by_id`, `resolve_pane_id`, `focus_pane_and_tab`) đã được hạ về crate scope thay vì tiếp tục là public cross-crate boundary
  - tiếp theo, một số helper chỉ còn là one-hop compat nội bộ đã bị xóa/inlined hẳn:
    - removed `runtime_entry_by_id(...)`
    - removed `has_tab(...)`
    - `root_active_runtime_id(...)`, `remove_terminal_handle(...)`, và `record_focus_for_terminal_handle(...)` không còn phải đi qua wrapper nội bộ dư thừa
  - thêm một nhát siết public surface:
    - `remove_tab_by_id(TabId)` không còn là public cross-crate helper
    - Lua bridge dùng `runtime_entry_by_session_id(...)` thay cho helper tên `tab_by_chatminal_session_id(...)`
  - execution-path boundary cũng đã đổi sang typed handle thêm một đoạn:
    - `spawn_tab(...)` dùng `Option<SessionTerminalHandle>`
    - `split_pane(...)` dùng `SessionTerminalHandle`
    - desktop host adapter và Lua bridge spawn/split flow không còn kéo `PaneId` qua public helper slice này
  - notification subscriber boundary cũng đã đổi thêm một nấc:
    - `MuxHandle::subscribe(...)` nay phát `HostRuntimeNotification`
    - desktop runtime bridge convert trực tiếp từ typed host notification đó
    - `MuxNotification` không còn là public desktop-facing API
  - PTY output callback boundary cũng đã typed hóa thêm:
    - `register_pane_with_output_callback(...)` và full callback path bên trong `chatminal-host-runtime` giờ mang `SessionTerminalHandle`
    - pane output events ở desktop bridge không còn phải widen raw `usize` trước khi thành `RuntimeNotification::PaneOutput`
  - root-window helper surface cũng được bó hẹp thêm:
    - `root_window_workspace_name(...)`
    - `root_window_title(...)`
    - `set_root_window_workspace_name(...)`
    - `set_root_window_title(...)`
    - `focus_root_runtime_entry(...)`
    - `root_runtime_ids(...)`
    - `runtime_entry_info_by_runtime_id(...)`
    - `root_runtime_entry_infos(...)`
    - `runtime_entry_info_by_session_id(...)`
    - desktop `session_host.rs` và Lua `WindowRef` đã chuyển một slice đầu tiên sang các helper này thay vì direct root-window mutation closures
    - Lua `WindowRef.sessions/sessions_with_info/active_session/active_terminal` và `LuaBridgeHost::root_tabs/root_window_spawn_context` cũng đã chuyển thêm một slice sang root-runtime helper paths
    - `crates/chatminal-lua-bridge` hiện không còn active `with_root_window(...)` / `with_root_window_result(...)` callsites trong product path của slice này
    - thêm một nhát nữa, các root-window query chỉ đọc metadata của Lua bridge giờ dùng `RuntimeEntryInfo` thay vì hydrate `Arc<Tab>` cho `session_id`, title, active terminal, hoặc active terminal instance id
    - các session-id read queries của Lua bridge (`session_active_terminal_instance_id`, `session_title`, `active_terminal_for_session`, `session_size`) cũng đã chuyển sang `RuntimeEntryInfo` thay vì fetch concrete tab
  - một số desktop raw-focus compat wrappers chỉ còn widen về `usize` đã bị xóa sau khi typed helper path đủ coverage
  - root-tab typed boundary cũng được kéo thêm một nấc:
    - `Tab::runtime_id()` giờ là đường typed read-only chính cho caller ngoài crate
    - Lua bridge đổi `RootTabRef` từ `usize` sang `RuntimeId`
    - desktop `main.rs`, `termwindow/mod.rs`, `desktop_termwindow_close_helpers.rs`, và `session_host.rs` không còn phải widen `tab.tab_id() as u64` ở các slice read-only vừa cắt
    - desktop `host_render_scope_size(...)` cũng không còn fetch concrete `Tab` ở fallback slice chỉ để đọc size; nó đi qua `RuntimeEntryInfo`
    - thêm một helper typed-handle hẹp hơn ở host boundary: `terminal_handle_for_pane(&dyn Pane)`, giúp các helper ngoài crate ngừng tự wrap raw `pane_id()` ở những slice đầu tiên đã migrate
- PTY I/O boundary cũng đã bắt đầu được bóc khỏi singleton semantics:
  - `chatminal-host-runtime/src/lib.rs` thêm `PtyIoDispatcher` để gom `output / cleanup / inline error` side effects của PTY pipeline
  - `send_actions_to_mux(...)`, `parse_buffered_data(...)`, và `read_from_pane_pty(...)` không còn hard-code toàn bộ side effects inline; giờ chúng đi qua dispatcher boundary
  - `Mux::add_pane_internal(...)` đã được chia bước đầu thành:
    - register pane
    - start PTY reader
  - socketpair inline-error rendering và `localpane::emit_output_for_pane(...)` giờ dùng chung inline-output dispatch helper, nên path lỗi PTY không còn phụ thuộc trực tiếp vào helper localpane cũ
  - default output fallback và default exit-cleanup fallback cũng đã được kéo vào helper riêng, nên dispatcher setup giờ chỉ còn assemble contract thay vì chứa branch logic trực tiếp cho cả ba loại side effect
  - cụm PTY parser/read loop cũng đã được tách ra module nội bộ `chatminal-host-runtime/src/pty_io.rs`, nên `lib.rs` không còn giữ parser implementation này inline
  - `LocalPane` cũng đã có hook seam riêng qua `chatminal-host-runtime/src/localpane_hooks.rs`; các path `record_input`, `inline output`, `alert`, và `child-exit cleanup` giờ đi qua default hook layer thay vì hard-code singleton-backed helpers trực tiếp trong `localpane.rs`
  - seam của `pty_io.rs` cũng đã được mở rộng thành `PtyIoHooks`, nên PTY reader startup có thể override đồng bộ `output`, `cleanup`, và `inline error` thay vì chỉ tiêm được output callback
  - local fallback spawn path cũng đã có seam riêng:
    - `chatminal-host-runtime/src/spawn_target.rs` thêm `LocalSpawnHooks`
    - `LocalSpawnTarget::{new_with_hooks,new_serial_target_with_hooks}` mở owner seam cho local-pane side effects và PTY callbacks trên fallback path
    - callback public ở seam này dùng `SessionTerminalHandle`, nên cross-crate boundary không cần lộ `PaneId`
    - local fallback pane registration giờ đi qua helper vừa giữ default clipboard/download install vừa nhận custom PTY hooks
  - phía session-native runtime cũng đã được kéo theo cùng hướng:
    - `leaf_runtime.rs` giờ tạo `TerminalInstanceRuntimeHooks`
    - `leaf_runtime_threads.rs` phát `Output / Error / Exited` qua hook contract này thay vì hard-code channel send trong loop
  - default dispatcher vẫn map về Mux-backed cleanup/notification behavior cho compat/non-session paths, nên 03G mới ở trạng thái `partial`
- Desktop typed boundary được siết thêm một notch nữa:
  - `FrontendResolvedPane` và `FrontendFocusedPane` giờ mang `RuntimeId` / `SessionTerminalHandle` thay vì `u64` thô
  - `frontend_resolve_pane(...)` và `focus_terminal_handle_by_id(...)` đã đổi sang typed-handle boundary ở desktop facade
  - `chatminal_runtime/mod.rs` cũng thu hẹp bớt compat re-export nội bộ từ `pub` xuống `pub(crate)`
  - pane-centric desktop notifications cũng đã đổi qua typed handle:
    - `RuntimeNotification::PaneOutput`
    - `RuntimeNotification::PaneAdded`
    - `RuntimeNotification::PaneRemoved`
    - `RuntimeNotification::PaneFocused`
    - `RuntimeNotification::Alert.pane_id`
    - `RuntimeNotification::AssignClipboard.pane_id`
    - các payload này giờ mang `SessionTerminalHandle` thay vì host `usize`
  - tab/render-scope desktop notifications cũng đã đổi qua typed runtime ids:
    - `RuntimeNotification::TabAddedToWindow`
    - `RuntimeNotification::TabResized`
    - `RuntimeNotification::TabTitleChanged.runtime_id`
    - các payload này giờ mang `RuntimeId` thay vì raw host tab ids
  - `desktop_host_runtime/session_host.rs` cũng giữ shim-tab registry ở `RuntimeId` (`session_tab_shim: HashMap<String, RuntimeId>`) thay vì `usize`, nên tab lookup/remove/focus ở desktop boundary không còn widen raw host ids quá sớm
  - split execution boundary cũng đã được siết thêm:
    - `SplitSource` không còn mang `PaneId` ở move-pane path; nó mang `SessionTerminalHandle`
    - public `SpawnTarget::split_pane(...)` cũng dùng `RuntimeId + SessionTerminalHandle`
    - việc convert về raw `TabId` / `PaneId` chỉ còn diễn ra trong host-runtime internals ngay trước khi chạm engine registry
  - thêm một trim nhỏ ở public surface của host-runtime:
    - desktop không còn import trực tiếp pane allocator từ `host_runtime::pane`; nó dùng root helper `alloc_terminal_handle_value()`
    - `window.rs` không còn public các helper chỉ phục vụ nội bộ mux như `idx_by_id`, `remove_by_id`, `prune_dead_tabs`
    - `Mux` cũng không còn public một cụm methods nội bộ lộ raw ids (`get_pane`, `get_tab`, `remove_pane`, `remove_tab`, `resolve_pane_id`, `record_focus_for_client`, `focus_pane_and_containing_tab`)
    - `tab.rs` cũng đã đổi `contains_pane(...)` sang `SessionTerminalHandle`, nên raw `PaneId` không còn lộ ở method công khai này
- Direct session-pane input path không còn rewrite `KeyCode::Backspace` sang `Char(...)`; leaf runtime/terminal encoder giữ quyền encode backspace, và session-pane forwarding tests quay lại xanh
- Desktop startup/config boundary cũng được gom thêm:
  - `main.rs` dùng `current_config_handle()` cho root startup/bootstrap snapshots
  - `stats.rs` dùng `periodic_stat_logging_secs()` cho các root stats bootstrap/reload reads
- Host runtime root config reads cũng được gom thêm:
  - `chatminal-host-runtime/src/lib.rs` dùng `default_exit_behavior()` và `default_workspace_name()` để giữ host bootstrap/exit fallback reads đi qua helper boundary thay vì rải trực tiếp ở nhiều chỗ
- Phase 04 vẫn partial:
  - `configuration()` vẫn còn callsite trong hot PTY parser path và một số render/bootstrap paths khác
  - config singleton removal full-sweep chưa nên làm trước khi Phase 03 ownership cut xong

## Latest changes (architecture unification phase 03A groundwork, 2026-04-01)
- `chatminal-runtime` now owns a first-class execution boundary trait: `RuntimeHost`
- Boundary DTOs added:
  - `RuntimeTerminalSize`
  - `RuntimeHostSessionState`
- `DesktopSessionHost` is now the concrete adapter implementing `RuntimeHost`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` has started routing execution-facing operations through `Arc<dyn RuntimeHost>` instead of hardcoding the concrete desktop host everywhere
- `RuntimeHost` now also owns session-mode terminal-handle binding/focus boundary:
  - `RuntimeHostTerminalBinding`
  - `terminal_binding_for_handle()`
  - `focus_terminal_handle()`
- Desktop session-mode helpers now resolve/focus terminal handles through `RuntimeHost` first, then fall back to `Mux` only for overlay/legacy paths
- Session-mode close/notification control paths also moved further off raw `Mux`:
  - render-target close now resolves through Chatminal session/runtime semantics before legacy `Mux` tab removal
  - frontend focused-pane/pane-resolution helpers now resolve through Chatminal session bindings first
- Workspace/frontend control-plane wrappers now sit in `DesktopSessionHost`; `desktop_host_runtime/mod.rs` delegates to that host layer instead of owning those `Mux` calls directly
- Spawn/bootstrap wrapper group now also sits one layer deeper in `DesktopSessionHost`; desktop host runtime delegates there first for:
  - local shell runner spawn
  - host runtime entry spawn
  - primary spawn target get/set
  - host mux bootstrap/shutdown
  - serial spawn-target creation
- Another small Phase 03B cut also moved host-window query/activation wrappers one layer deeper into `DesktopSessionHost`:
  - root-window render-scope existence fallback
  - resolved window title
  - active runtime entry size
  - launcher session snapshot
  - root-window runtime-entry activation fallback
- A follow-up Phase 03B cut moved pane/focus/frontend fallback wrappers one layer deeper into `DesktopSessionHost`:
  - active render-scope fallback lookup
  - remove pane / remove runtime-entry tab
  - record focus for current identity
  - fallback pane focus in host window
  - fallback public-pane resolution
  - fallback frontend pane resolution / focused-pane resolution
  - panes-in-workspace query used by startup/spawn decision
- Another Phase 03B cut moved generic window/workspace wrapper closures one layer deeper into `DesktopSessionHost`:
  - `with_host_window`
  - `with_host_window_mut`
  - host-window existence
  - workspace-has-window lookup
  - resize-all-tabs on host window
- Latest Phase 03B cut pushed the remaining raw `Mux` control-plane fallback out of `desktop_host_runtime/mod.rs` entirely:
  - `desktop_host_runtime/mod.rs` now has zero direct `Mux::get()` / `Mux::try_get()` calls
  - raw host primitive access now lives deeper in:
    - `desktop_host_runtime/session_host.rs`
    - `desktop_host_runtime/session_pane.rs`
- Follow-up cleanup on the same boundary:
  - `session_host.rs` control-plane instance methods now thin-wrap the corresponding `legacy_*` helper instead of keeping duplicate inline host logic
  - the dead `set_host_workspace()` instance method was removed after delegation finished
- Additional cleanup in the lower adapter:
  - `session_pane.rs` now funnels repeated pane-output notifications and input-record hooks through small local helpers, reducing scatter of the remaining raw host primitive calls
  - `session_host.rs` now centralizes raw host fallback behind a small host-helper block (`host_mux()/try_host_mux()` + typed host primitive helpers)
  - `DesktopSessionHost` runtime/reconcile/shim/resource-cleanup methods no longer call `HostMux::get()` directly; they go through the shared helper block instead
  - latest Phase 03C pane-registry slice deepened desktop-local ownership:
    - session-native pane lookup/binding helpers now resolve from `DesktopSessionHost.panes` instead of only the `session_pane` alias map
    - session-native public-id resolution now prefers `DesktopSessionHost` local pane registry before legacy mux fallback
    - frontend pane-resolution fallback now prefers `DesktopSessionHost` local terminal binding before legacy pane-id → tab-id lookup
    - direct host-mux access is now gone from `session_pane.rs`
    - direct regression tests now cover:
      - public-id → terminal binding resolution
      - frontend pane fallback resolution without requiring mux-global pane lookup
      - local prune before legacy fallback
      - stale-session prune in `reconcile_visible_sessions()`
  - latest Phase 03D accessor slice removed direct singleton accessor calls from the desktop app:
    - `apps/chatminal-desktop` no longer calls `Mux::get()` / `Mux::try_get()` / `HostMux::get()` / `HostMux::try_get()` directly
    - singleton accessor usage for the desktop path is now funneled through `chatminal-host-runtime` free helpers
  - latest Phase 03E groundwork introduced a narrow bridge host:
    - `chatminal-lua-bridge` now has a `LuaBridgeHost` wrapper that centralizes the current mux-backed runtime access
    - `SessionRef`, `TerminalRef`, and `WindowRef` no longer thread `Arc<Mux>` through their resolve APIs
    - workspace/window/session/terminal Lua methods now call `get_host()` instead of scattering direct mux access throughout the crate
    - `TerminalRef` now stores `usize` directly rather than exposing `PaneId`
    - bridge modules themselves no longer call `get_mux()` / `Mux::get()` / `Mux::try_get()` directly; the remaining mux-backed compatibility is centralized inside `LuaBridgeHost`
    - the first bridge-local follow-up cut removed lock-guard leakage from the window/workspace metadata path:
      - `LuaBridgeHost` now exposes closure/value-based root-window access helpers
      - `WindowRef` no longer returns `MappedRwLock*Guard` handles to callers
      - `SpawnSession::spawn()` snapshots root-window size and active pane through the same closure-based bridge
    - the next 03E cut moved `SessionRef` behavior behind the same host:
      - `LuaBridgeHost` now holds one stable `Arc<Mux>` per Lua call so multi-step bridge operations observe one backend instance
      - `session.rs` no longer resolves `Tab` directly for the common session/title/window/activation surface
      - compat `session.all_terminals()` still reads the mux pane registry instead of shrinking to root-window-only panes
    - the next 03E cut moved common `TerminalRef` behavior behind the same host:
      - `leaf.rs` no longer resolves panes directly from the mux for the common terminal metadata/query surface
      - generic pane lookup now flows through `LuaBridgeHost::with_pane(...)` / `with_pane_result(...)`
      - terminal activation now routes through `LuaBridgeHost::activate_terminal(...)`
      - `TerminalRef` tuple field is now private; callers go through `from_pane_id()` / `pane_id()` helper methods instead of raw tuple access
      - the old clockwise rotation bug in the session bridge was corrected while centralizing this slice
    - the next 03E cut finished hardening the spawn/split surface:
      - spawn-context derivation now lives in `LuaBridgeHost`
      - `SpawnSession::spawn()` now dispatches through host-owned spawn helpers
      - `SplitSession::run()` now dispatches through host-owned split helpers
- `chatminal_runtime/mod.rs` now starts exposing explicit desktop-host lifecycle facade calls for entrypoints:
    - `initialize_desktop_host_runtime()`
    - `shutdown_desktop_host_runtime()`
    - `create_desktop_serial_spawn_target()`
  - `main.rs` now uses those explicit facade APIs instead of depending on broad `desktop_host_runtime::*` re-exports for host bootstrap/shutdown/serial-target setup
  - `chatminal_runtime/mod.rs` no longer uses wildcard host re-export; the desktop host surface is now spelled out explicitly
  - first wrapper families now converted into real facade functions in `chatminal_runtime/mod.rs`:
    - spawn/bootstrap: `set_host_spawn_target`, `primary_host_spawn_target`, `host_has_panes_in_workspace`, `start_host_activity`, `host_activity_count`, `show_host_configuration_error_message`, `host_workspace_name`, `spawn_local_shell_runner`, `spawn_host_runtime_entry`
    - window/shell: `with_host_window`, `with_host_window_mut`, `host_window_exists`, `host_window_contains_render_scope`, `host_window_initial_position`, `resolved_window_title`, `active_host_runtime_entry_size`, `resize_host_window_tabs`
    - pane/render-target compat: `host_active_render_scope_id`, `terminal_handle_arc`, `terminal_handle_arc_by_public_id`, `remove_terminal_handle`, `remove_runtime_entry_scope`, `focus_terminal_handle`, `record_host_focus_for_current_identity`, `resolve_public_pane`, `frontend_resolve_pane`, `frontend_resolve_focused_pane`, `subscribe_runtime_notifications`, `launcher_sessions`
    - frontend/workspace: `primary_host_window_id`, `primary_host_window_exists`, `active_frontend_client`, `subscribe_frontend_notifications`, `active_workspace_for_client`, `set_active_workspace_for_client`, `workspace_is_empty`, `workspace_names`
  - `frontend.rs` and `desktop_spawn.rs` now import these explicit facade APIs directly instead of scattering `crate::chatminal_runtime::...` calls through function bodies
  - caller migration has expanded beyond entrypoints:
    - `desktop_termwindow_host_runtime_helpers.rs`
    - `desktop_termwindow_close_helpers.rs`
    - `desktop_termwindow_positioned_session_helpers.rs`
    - `overlay/confirm_close_pane.rs`
    - `overlay/launcher.rs`
    - `overlay/copy.rs`
    - `termwindow/resize.rs`
    - `termwindow/render/paint.rs`
  - `session_pane.rs` no longer uses `HostMux` directly at all:
    - pane output now publishes through the desktop runtime notification hub
    - input-activity recording now routes through the desktop host wrapper instead of touching mux from the pane implementation
  - root-window bridge/control paths tightened again:
    - `chatminal-host-runtime/src/window.rs` snapshots close-tab behavior config at construction and routes window notifications through helper wrappers
    - `chatminal-lua-bridge/src/window.rs` now uses `LuaBridgeHost` closure helpers for workspace/title/session/root-window queries instead of raw mux guard resolution
- Render-target/window activation wrappers also now prefer session-native render targets before `Mux` fallback:
  - active render target lookup
  - render-target size lookup
  - render-target activation
- Verification quality improved during Phase 03B:
  - canonical scrollback retention bug in `chatminal-store` fixed
  - `session_pane` tests now serialize host-mux setup/teardown to avoid global `Mux` parallel-test flake
- This is an incremental cutover only:
  - `Mux` facade compat vẫn còn tồn tại
  - render/input shell still has lower adapter paths dựa vào compat `Mux` semantics
  - `desktop_host_runtime/mod.rs` is now clean of direct `Mux::get()` / `Mux::try_get()`, and desktop app direct singleton accessor calls are now 0
  - lower adapter behavior is still mux-backed through `chatminal-host-runtime`
  - `chatminal-lua-bridge` no longer scatters direct mux access across every module, but `LuaBridgeHost` is still mux-backed today
  - `session_host.rs` still contains both instance methods and `legacy_*` fallback surface, but the raw host primitive layer is now localized instead of being scattered across runtime methods
  - `chatminal_runtime/mod.rs` is now explicit instead of wildcard-based; only 14 host-side explicit re-exports remain, and the rest of the active desktop host surface already flows through facade functions there

## Latest changes (performance optimization phases 1-4, startup recipes, 2026-03-30/31)
- **Phase 1**: SQLite connection reuse (`Arc<Mutex<Connection>>`) — eliminated ~40 `open_connection()` syscalls per event cycle
- **Phase 2**: Async persist pipeline — background thread with coalescing (UpdateSeq burst N→1, MarkRunning dedup); 5 SQLite writes → 0 under global lock; live_output buffer 1MB→256KB
- **Phase 3**: SQL-based retention — window function `SUM OVER` for memory-bounded eviction; O(N)→O(1) RAM overhead
- **Phase 4**: Resource caps — scrollback 10K→3K lines (~70% RAM reduction); OutputHistory 2MB→512KB; EnforceLimit throttle mỗi 50 chunks (disk I/O -98%); Arc<str> migration for session_id in SessionEvent + TerminalInstanceRuntimeEvent + SessionRuntimeEvent; WebGPU bind group caching
- **Result**: RAM per 5 sessions: ~385MB→~115MB (70% reduction); zero SQLite writes under global lock; verified all tests pass
- **Startup recipes**: Per-session startup commands với multi-step syntax (run/type/enter/wait/wait-for); registry trong `crates/chatminal-runtime/src/state/startup_recipes.rs`

## Canonical scrollback dual-read/canonical-write model (2026-03-27)
- Persisted history không còn coi `scrollback_chunks.chunk_text` là source of truth duy nhất.
- `chatminal-runtime` đã chuyển writer active sang canonical model:
  - `CommittedLine { seq, ord, ts, text }`
  - `OpenFragment { seq, ord, ts, text }`
- Store thêm table `scrollback_records(session_id, seq, ord, kind, record_text, ts)` và index `(session_id, seq DESC, ord ASC)`.
- Active runtime path:
  - `SessionEvent::Output` persist canonical records
  - `session_set_persist()` flush live buffer sang canonical records
  - restore path trong desktop đổi sang `session_restore_snapshot_get()`
- Reader hiện là dual-read:
  - canonical-write
  - canonical-read + legacy-read
  - nếu canonical đã có records cho `seq = N` thì legacy chunk `seq = N` bị bỏ
- Reopen/restore không còn replay hard-wrapped text cũ cho dữ liệu mới; terminal engine tự wrap lại theo width hiện tại khi hydrate snapshot.
- Legacy `scrollback_chunks` vẫn giữ ở chế độ read-only compat cho DB cũ; clear-history / clear-all-history dọn cả legacy lẫn canonical.

## Latest changes (single-flow desktop/spawn-target cleanup complete, 2026-03-25)
- Desktop startup path không còn public routing theo cặp startup flags legacy cho attach/spawn selection; `StartCommand` product surface đã cắt hai cờ này.
- `build_initial_host_mux()` của desktop path luôn khởi tạo default startup flow theo `local` target; config/CLI không còn lái startup sang target khác ở product path.
- `crates/chatminal-lua-bridge` đã cắt các public Lua entry points legacy như `session.get_target`, `session.all_targets`, `session.set_default_target`.
- `gui-attached` Lua event không còn mang payload target-ref; `crates/chatminal-lua-bridge` cũng đã bỏ target-ref helper, `terminal.get_target_name`, và public Lua spawn/split target override.
- Active config/product code path không còn các enum/action legacy kiểu attach/detach theo execution target.
- Host runtime không còn giữ attach/detach/state compat semantics cho `SpawnTarget`; desktop startup path cũng không còn wrapper `attach_host_target()`.
- Dead helpers `spawnable`, `target_label`, `iter_targets`, `target_was_detached` đã bị xóa khỏi active host-runtime path.
- Active host engine đã đổi vocabulary nội bộ ổn định sang `SpawnTarget/SpawnTargetId/spawn_target_id()`.
- `chatminal-config` public model đã đổi từ legacy target-list keys sang `*_targets` / `default_target`; Lua keys `default_wsl_targets` và `exec_target` đã sync theo vocabulary mới.
- Legacy vocabulary cũ hiện không còn nằm trong active desktop/host-runtime/config product path; các chỗ còn lại chỉ là historical docs hoặc từ vựng kỹ thuật khác như Unix socket.

## Latest changes (architecture unification phase 05 final closeout, 2026-04-03)
- Host-runtime product ownership now lives on `HostRuntimeRoot`:
  - `initialize_host_runtime()` installs/returns a root-backed compat handle
  - `shutdown_host_runtime()` clears the installed root
  - root notifications in the product path now route through the host runtime root instead of `Mux` ownership
- PTY/local spawn defaults were moved:
  - product path uses `host_default()` for host-runtime PTY/localpane hooks
  - `mux_default()` remains only as an explicit compat alias, not the default product path
  - desktop local spawn target now opts into `LocalSpawnHooks::host_default()` for local and serial targets
- Scope split is explicit:
  - config sectioning / singleton replacement work is deferred into `plans/260403-1800-post-unification-followups`
  - crate rename / cosmetic sweep is also deferred there
- Verification gates passed for this closeout:
  - `cargo check --workspace`
  - `cargo test --workspace --lib --bins --tests`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `cargo test -p chatminal-lua-bridge`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`

## Latest changes (Phase 2 complete, 2026-03-17)
- **Phase 2.1 - Type alias consolidation**: 17 Runtime* types (RuntimeSession, RuntimeProfile, etc.) are now **direct type aliases** to `chatminal-protocol` types. Deleted `api/protocol.rs` (431 LOC) conversion boilerplate; moved 5 Store→Protocol From impls to `chatminal-store`.
- **Phase 2.2 - Engine split fallback removal**: Deleted `split_terminal_handle` + `split_terminal_handle_by_public_id` from `desktop_host_runtime/mod.rs`; removed `HostSplitSource`, `HostRuntimeEntryId`, `HostLayoutNode`, `HostSplitDirection` type aliases; replaced desktop_spawn.rs split fallback (lines 111-131) with `anyhow::bail!` error.
- **Phase 2.3 - Dead code cleanup**: Removed 4 functions (`active_host_target_name`, `set_default_host_target`, `new_headless_connection_ui`, `host_client_targets`); removed 3 type aliases; ~33 LOC cleaned. Note: tab.rs split functions cannot be removed (lua-bridge dependency).
- **Phase 2.4 - Workspace layout persistence**: Already implemented via `set_string_state`/`get_string_state` with key prefix `workspace_layout:`; mutations auto-save to app_state table.
- **Phase 2.5 - Documentation**: Added doc comment to window.rs explaining single-Window/single-Tab desktop model; this file updated.

## Single-process desktop model (2026-03-31)
- **No daemon**: `chatminald` fully DELETED; runtime embedded in desktop process
- **Single event flow**: PTY output → SessionEngine → SessionEventProcessor (in-memory) → broadcast (UI) + async persist job
- **Persist worker**: Background thread, coalesces database writes, zero operations under global lock
- **Session lifecycle**: Created, resumed, closed all within desktop process; no IPC
- **Startup commands**: Per-session recipes available before first prompt; `startup_recipes` registry persisted in SQLite

## Topology

### Desktop app (single-process)
```text
chatminal-desktop
  -> chatminal_runtime facade
    -> chatminal-runtime (embedded, single-process)
      -> session engine + terminal runtime registry
      -> persist worker (background thread)
    -> desktop_host_runtime (private engine adapter)
      -> chatminal-host-runtime (Mux/Tab/Pane private engine primitives)
  -> termwindow render/input shell
  -> chatminal_sidebar + session bar UI
```

### Persistence and compatibility
```text
chatminal-runtime
  -> chatminal-store (SQLite)
  -> profiles / sessions / canonical scrollback / legacy compat scrollback / workspace layout state
  -> native_api + runtime_bridge
```

## Architecture rules
- Product model: `session -> session_view -> session_group -> workspace_layout -> render_target -> terminal_instance`.
- Desktop startup/public command path là single-flow local-first; không expose public legacy target-selection/attach semantics nữa.
- `apps/chatminal-desktop/src/chatminal_runtime/*` là desktop facade duy nhất cho product state/query/action.
- `apps/chatminal-desktop/src/termwindow/*` và `desktop_termwindow_*` chỉ là render/input shell; không phải source of truth cho business routing.
- `apps/chatminal-desktop/src/desktop_host_runtime/*` là private adapter duy nhất còn chạm host primitives.
- `crates/chatminal-host-runtime/*` được phép giữ `Mux/Tab/Pane`, nhưng chỉ như engine implementation detail.
- `apps/chatminal-desktop/src/desktop_commands.rs` là compatibility translation layer cho `KeyAssignment::*Tab*`; product-facing code không route trực tiếp các symbol đó.

## Runtime flow

### 1. Product state
- `chatminal-runtime` giữ profile/session persistence, workspace snapshot, native API và desktop-facing runtime bridge.
- `chatminal-runtime::state::canonical_scrollback` là source of truth cho logical scrollback semantics:
  - reducer shell-level tối thiểu: `\r`, `\n`, backspace, erase-in-line
  - mixed-source merge theo `(seq, ord)`
  - restore/preview materialize từ logical snapshot, không từ wrapped text cũ
- live execution model hiện nằm trong desktop private host/session-engine path; không còn crate `chatminal-session-runtime` riêng trong active repo.
- `workspace_layout` là public execution/layout model cho app layer; không expose host split tree ra desktop product path.

### 2. Desktop facade
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` expose desktop bindings như session/view/render-target/terminal handle/window snapshot.
- `client.rs` và facade helpers resolve active session, ordered session entries, focus/close/swap routing qua runtime boundary.
- Desktop shell không còn tự ghép business state từ host tab/pane metadata như source of truth.

### 3. Render/input shell
- `termwindow/*` render terminal, overlay, selection, launcher, mouse/key events.
- `tabbar.rs` đã trở thành session bar state/product UI model.
- `chatminal_layout/*` và `layout_render.rs` map workspace layout sang geometry render thực tế.

### 4. Private engine adapter
- `desktop_host_runtime/*` bridge từ facade/runtime sang engine host thực tế.
- Adapter này giữ host window/session pane/runtime pane/spawn-target internals, nhưng desktop startup không còn nhận public target override hay attach-by-target flow.
- Host vocabulary bị thu xuống `pub(crate)` hoặc private trong desktop app path.
- Render path: `WorkspaceLayout → session_id → DesktopSessionHost.pane(session_id) → GPU draw`
  (không còn đi qua `HostRenderScope` để build pane list).
- `HostRenderScope` fully removed; `OverlayRenderScope` isolated from overlay boundary. Session owns pane directly via `session_id → Arc<ChatminalSessionPane>` lookup.

### 5. Lua/config boundary
- `crates/chatminal-lua-bridge/*` expose Chatminal-facing session/window/terminal queries.
- Public APIs `get_host_tab` và `get_host_leaf` đã bị xóa.
- Public Lua surface dùng `terminal`/`terminal_instance_id` thay cho host-tab/host-leaf ids.
- Public Lua surface không còn các helper target-selection legacy.
- Public Lua surface không còn `terminal.get_target_name`, và `spawn_window` / `spawn_session` / `split` không còn nhận public execution-target override.
- Active keyassignment surface không còn action attach/detach/spawn theo execution target.

## Type alias consolidation (Phase 2.1)
17 Runtime boundary types (`RuntimeSessionStatus`, `RuntimeProfile`, `RuntimeSession`, `RuntimeWorkspace`, `RuntimeCreatedSession`, `RuntimeLifecyclePreferences`, `RuntimeSessionSnapshot`, `RuntimeSessionExplorerState`, `RuntimeSessionExplorerEntry`, `RuntimeSessionExplorerFileContent`, `RuntimePtyOutputEvent`, `RuntimePtyExitedEvent`, `RuntimePtyErrorEvent`, `RuntimeSessionUpdatedEvent`, `RuntimeWorkspaceUpdatedEvent`, `RuntimeDaemonHealthEvent`, `RuntimeEvent`) are now **direct type aliases** in `crates/chatminal-runtime/src/api/mod.rs` to their `chatminal-protocol` counterparts.

**Previous structure (pre-Phase 2.1):**
- Store layer: `StoredSession`, `StoredProfile` (SQLite-specific fields)
- Protocol layer: `SessionInfo`, `ProfileInfo` (network protocol)
- Runtime layer: `RuntimeSession`, `RuntimeProfile` (redundant duplicates)
- Conversion: `api/protocol.rs` (431 LOC) with `From` impls

**Current structure (post-Phase 2.1):**
- Store layer: `StoredSession`, `StoredProfile` (SQLite-specific, unchanged)
- Shared: `chatminal-protocol` types (used directly by Runtime via aliases)
- Conversion: `From` impls for Store→Protocol moved to `chatminal-store` crate
- **Benefit**: No more type redundancy at runtime boundary; desktop/daemon both use protocol types directly.

## Verification freeze
- `cargo check --workspace`: pass
- `cargo check --workspace --all-targets`: pass
- `cargo test -p chatminal-runtime -- --test-threads=1`: pass
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`: pass

## Workspace layout persistence
- Layout state persisted via `native_api.rs` `set_string_state`/`get_string_state` with key prefix `workspace_layout:`.
- All mutations (split, close, resize, focus) auto-save to `app_state` table as JSON blob.
- No separate schema migration needed; key-value store handles layout versioning.

## Remaining intentional compatibility
- Engine internals vẫn có `Mux/Tab/Pane` trong `chatminal-host-runtime` và private adapter desktop.
- Command/config compatibility vẫn giữ upstream `KeyAssignment::*Tab*` translation trong `desktop_commands.rs` để không gãy config cũ.
- `OverlayRenderScope` dùng cho launcher/confirm/prompt overlays nhưng không còn coupled với render scope; boundary fully isolated.
- `SessionExecutionStatus` enum thêm vào `chatminal-runtime/state.rs` để track running status.
- History compatibility hiện còn một vùng intentional:
  - `scrollback_chunks` chỉ read-only compat cho session DB cũ
  - active writer path không còn ghi mới vào legacy table
  - cleanup reader legacy sẽ là phase riêng sau khi đủ confidence rollout
- Các phần trên là intentional private/compatibility zones, không còn là product-facing architecture.

## Phase 05 Final Closeout (2026-04-03)
- Host runtime init/shutdown path now installs and clears `HostRuntimeRoot` directly; `Mux` is no longer the product runtime owner.
- Default PTY/local spawn path uses `host_default()` hooks; `mux_default()` remains only as explicit compat seam.
- Deferred config sectioning / singleton replacement and crate rename work moved to follow-up plan:
  - `plans/260403-1800-post-unification-followups/plan.md`
- Architecture closeout rule:
  - plan `260401-0949-architecture-unification` is only done after the closeout checklist is green against source reality, not plan artifacts.
