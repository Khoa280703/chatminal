# Codebase Summary

Last updated: 2026-04-04 (terminal layer merge cutover)

## Closeout status
- Active terminal domain canonical: `chatminal-terminal-emulator`
- `260404-1532-merge-terminal-core-and-emulator` đang là active closeout wave cho terminal domain convergence.
- Product path đã chốt `chatminal-terminal-emulator` làm canonical terminal layer.
- `chatminal-terminal-emulator` là terminal layer canonical; `chatminal-terminal-core` không còn là target architecture và chỉ còn được phép biến mất khỏi active workspace path.
- `260401-0949-architecture-unification` đã closeout xong.
- Deferred scope đã được chuyển sang:
  - `plans/260403-1800-post-unification-followups/plan.md`
- Follow-up phase 01 config ownership completion đã xong ở product path:
  - `chatminal-window`, `chatminal-terminal-font`, `chatminal-time-funcs`, `chatminal-ratelim`
  - residual `configuration()` reads còn lại chủ yếu ở `chatminal-config` helpers và test/comment seams
- Closeout verify target:
  - `cargo check --workspace`
  - `cargo test --workspace --lib --bins --tests`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - `make window` bounded smoke launch

## Runtime baseline
Chatminal hiện là **single-process desktop** với hai lớp:
- `apps/chatminal-desktop` — desktop app first-party, render/input shell + desktop facade + private engine adapter.
- `crates/chatminal-runtime` — embedded orchestrator (không daemon), persistence facade, session state, startup recipes, persist worker.

Daemon `chatminald` đã hoàn toàn bị xóa; mọi session management hiện nằm trong desktop process.

Desktop product path là `single-flow local-first`:
- startup không còn public legacy flags cho attach/spawn selection
- desktop host mux luôn boot theo default local flow
- active keyassignment/config path không còn các action legacy kiểu spawn/attach/detach theo execution target
- host-runtime không còn `SpawnTarget.attach/detach/state` compat flow
- active host engine đã đổi naming sang `spawn target`; legacy vocabulary cũ không còn nằm trong active product path
- desktop app entry mặc định luôn boot unified shell path; sidebar không còn gate bởi env runtime flag
- desktop boundary config hot path đang được siết lại:
  - `frontend.rs` giữ config snapshot lúc init rồi refresh on reload
  - `main.rs` thread `ConfigHandle` qua desktop bootstrap thay vì re-read dọc startup flow
  - `stats.rs` cache `periodic_stat_logging` bằng atomic + config subscription
  - `selection.rs` và `overlay/copy.rs` consume config từ desktop-held UI state thay vì đọc singleton trong hot UI path
  - `customglyph.rs` snap AA policy 1 lần từ glyph cache config snapshot cho block draw path
  - `chatminal-host-runtime` đã thêm guard cho re-init split-brain và cleanup after shutdown
  - `chatminal-host-runtime/src/lib.rs` đã tách control plane khỏi `Mux` bằng `HostRuntimeControlPlane` cho spawn target / subscribers / clients / identity / workspace / focus metadata
  - `HostRuntimeRoot` giờ là owner thật của `tabs/panes/window/control`; global slot chỉ giữ `Weak<HostRuntimeRoot>`, còn `Mux` chỉ là facade compat mỏng
  - helper layer ở host-runtime giờ ưu tiên `HostRuntimeRoot` / `with_control_plane(...)` cho root-window/workspace/query/control paths thay vì route qua facade dày hơn
  - `HostRuntimeControlPlane` giờ cũng giữ focus metadata theo `SessionTerminalHandle`; `ClientInfo` vẫn serialize field cũ `focused_pane_id` để boundary mixed-version không gãy
  - desktop facade cũng đang siết dần raw numeric ids:
    - `FrontendResolvedPane` / `FrontendFocusedPane` giờ mang `RuntimeId` / `SessionTerminalHandle`
    - `frontend_resolve_pane(...)` / `focus_terminal_handle_by_id(...)` đã đổi qua typed boundary thay vì `u64`
    - `chatminal_runtime/mod.rs` đã thu hẹp thêm compat re-export nội bộ xuống `pub(crate)`
    - pane-centric `RuntimeNotification` payloads ở desktop boundary cũng đã chuyển sang `SessionTerminalHandle`
  - root-window control path cũng đang được siết:
    - `chatminal-host-runtime/src/window.rs` không còn chạm `Mux` inline cho title/workspace notify
    - `chatminal-lua-bridge/src/window.rs` không còn resolve raw root-window guards cho common window/session queries
    - `window.rs` ở host/lua sides đều ưu tiên snapshot/local helper hơn raw singleton/root-window access
    - `WindowRef.active_session()` và `WindowRef.active_session_id()` cùng đi qua `LuaBridgeHost::root_active_tab_info()`, nên contract đọc session hoạt động đã thống nhất ở một root-entry path
    - `TerminalRef` không còn lộ raw tuple field; bridge boundary giữ `SessionTerminalHandle` nội bộ, desktop Lua trigger path dùng `pane_id()`, và `LuaBridgeHost` gom lookup qua `with_*_tab` helpers cùng seams `ensure_runtime_available()` / `session_tab_info()`
    - focused-pane host boundary không còn trả raw tuple; `FocusedPaneBinding` là DTO hiện tại giữa host-runtime và desktop boundary
    - `ClientInfo.focused_pane_id` không còn public field
  - session-pane direct key forwarding không còn rewrite `Backspace` sang `Char(...)`; leaf runtime encoder giữ contract encode
  - startup/config bootstrap boundary cũng gọn hơn:
    - `main.rs` dùng `current_config_handle()`
    - `stats.rs` dùng `periodic_stat_logging_secs()`
  - `chatminal-config` giờ có thêm foundation entry points nhỏ để phase config propagation bám contract hẹp hơn:
    - `current_config_handle()`
    - `default_workspace_name_or(...)`
    - `current_initial_terminal_size()`
    - `current_output_parser_config()`
    - `current_exit_behavior()`
    - `TermConfig::enq_answerback()` cũng đã đi qua snapshot/injected config path thay vì gọi global singleton riêng lẻ
  - local host spawn env path cũng đã được siết lại thêm một nhát:
    - flatpak host path không còn re-apply `CHATMINAL_UNIX_SOCKET` / `SSH_AUTH_SOCK` từ sandbox sau `fixup_command()`
    - local spawn vẫn re-apply `CHATMINAL_PANE`
    - `SSH_AUTH_SOCK` ưu tiên env hiện tại trước snapshot identity cũ
  - `chatminal-codec` đã tách compile-time raw id aliases khỏi host-runtime, nên private hóa `PaneId` / `TabId` không còn làm vỡ desktop build qua codec path
  - `chatminal-host-runtime` cũng đã có thêm capability helpers mới để future desktop/Lua cutover bớt phải kéo `Arc<Tab>`:
    - `runtime_entry_exists(...)`
    - `set_runtime_entry_title(...)`
    - `runtime_entry_terminal_handles(...)`
    - `runtime_entry_terminal_handle_in_direction(...)`
    - `set_runtime_entry_zoomed(...)`
    - `rotate_runtime_entry_counter_clockwise(...)`
    - `rotate_runtime_entry_clockwise(...)`
    - `set_runtime_entry_active_terminal(...)`
    - `runtime_entry_terminal_infos(...)`
    - cùng một lớp helper này cũng đã có mirror theo `session_id` để future Lua cutover không phải tiếp tục giữ `tab_by_session_id(...)`
  - root-window/runtime-entry foundation vừa được kéo thêm một nhịp:
    - `RootWindowInfo` + `root_window_info()` / `root_last_active_runtime_id()` gom root-window read metadata vào DTO thay vì bắt caller giữ `with_root_window(...)`
    - `create_attached_runtime_entry_for_terminal(...)` cho phép dựng/attach runtime entry trực tiếp từ `Pane`, nên các phase sau có thể bớt tự dựng `Arc<Tab>` cho flow shim/simple attach
    - `spawn_target.rs` đã dùng lại builder path này để tránh duplicated root-entry creation logic
    - thêm tiếp helper cho nhóm callsite thường còn phải hydrate concrete `Tab`:
      - close check
      - runtime entry resize
      - split layout snapshot
      - split resize
      - zoom toggle
      - active terminal getter
      - activate terminal theo index/direction
      - active terminal swap
      - active terminal size adjust
      - tất cả đều có mirror theo `session_id`
    - root-window read slice cũng có thêm DTO/helper để bớt dùng `with_root_window(...)` ở phase desktop cutover sau:
      - `RootWindowInfo.initial_position`
      - `root_window_initial_position()`
      - `root_active_runtime_entry_info()`
      - `root_runtime_entry_summaries()`
      - `root_runtime_entry_count()`
      - `root_active_runtime_entry_index()`
      - `root_last_active_runtime_entry_index()`
      - `root_runtime_id_at_index()`
      - `root_runtime_entry_info_at_index()`
      - `focus_root_runtime_entry_index()`
      - `focus_root_last_runtime_entry()`
      - `focus_root_runtime_entry_relative()`
      - `move_root_active_runtime_entry_to_index()`
    - async/session-id slice cũng có thêm helper để bớt nhận concrete tab:
      - `runtime_id_for_session_id()`
      - `focus_root_runtime_entry_by_session_id()`
      - `spawn_runtime_entry()` trả `RuntimeEntryInfo + Pane`
    - terminal metadata/public-id slice cũng được gom xuống host-runtime:
      - `terminal_instance_id_for_pane()`
      - `terminal_by_terminal_instance_id()`
      - `terminal_by_public_id()`
      - `resolve_runtime_id_for_terminal_instance_id()`
    - host-runtime tests giờ có shared lock cho các case đụng global runtime root/runtime env để tránh race giữa foundation tests

## Current cleanup status
- Architecture unification Phase 01 complete:
  - deleted dead config surface for SSH/TLS/WSL
  - deleted auto-update config stubs
  - removed WSL branch from `crates/chatminal-host-runtime/src/spawn_target.rs`
  - removed root workspace dependency `libssh-rs`
- Intentional keep:
  - `crates/chatminal-config/src/ssh.rs` now only contains minimal active helpers `SshParameters` and `username_from_env()`
  - `engine-gui-subcommands` and `engine-toast-notification` remain active

## High-signal modules

### Desktop facade and shell
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - desktop-facing bindings/query/action cho session/view/render-target/terminal handle/window snapshot.
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
  - desktop runtime client dùng để resolve/dispatch actions qua runtime boundary.
- `apps/chatminal-desktop/src/termwindow/mod.rs`
  - coordinator cho render/input/overlay shell.
- `apps/chatminal-desktop/src/tabbar.rs`
  - product-facing session bar state.
- `apps/chatminal-desktop/src/chatminal_layout/*`
  - layout helpers cho workspace/session views.
- `apps/chatminal-desktop/src/chatminal_render/*`
  - render DTO/adapters dùng boundary types mới.

### Desktop private adapter
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - private adapter boundary duy nhất còn biết host runtime.
  - Phase 2 cleanup: deleted `split_terminal_handle*`, `HostSplitSource`, `HostRuntimeEntryId`, `HostLayoutNode`, `HostSplitDirection` type aliases; removed 4 dead functions.
  - Single-flow cleanup: desktop mux init luôn default `local`; product path không còn resolve startup target từ CLI/config public surface, cũng không còn wrapper attach-by-target.
  - Cleanup mới nhất: internal naming đổi sang `HostSpawnTarget*`, `spawn_target.rs`, `spawn_target_id()`.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - private session host: builds `ChatminalRenderState` trực tiếp từ `session_pane` map; `HostRenderScope` chỉ còn cho overlay compat.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
  - terminal pane bridge cho output/input/runtime metadata.
- **DELETED** `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs` — Phase 2 cleanup (was only 250 LOC, subsumed by session_engine)
- **DELETED** `apps/chatminal-desktop/src/desktop_host_runtime/pane.rs` — Phase 2 cleanup (528 LOC test-only wrapper, tests refactored)

### Runtime and execution core (single-process embedded)
- `crates/chatminal-runtime/src/state/native_api.rs`
  - desktop/native API cho workspace/session lifecycle.
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
  - mapping giữa persisted/runtime-facing state.
- `crates/chatminal-runtime/src/state/persist_worker.rs`
  - background thread, coalescing persist jobs, zero lock contention.
- `crates/chatminal-runtime/src/state/session_event_processor.rs`
  - in-memory event processor, 5 SQLite writes → 0 under lock.
- `crates/chatminal-runtime/src/state/startup_recipes.rs`
  - per-session startup command registry (run/type/enter/wait/wait-for).
- `crates/chatminal-runtime/src/api/mod.rs`
  - app-facing boundary ids/snapshots.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/mod.rs`
  - session engine facade.
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/leaf_runtime.rs`
  - PTY instance runtime (CoreTerminal + IoTerminal, 3K scrollback lines).
- `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/output_history.rs`
  - output history bounded to 512KB per session.
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - desktop facade bindings for session/view/render-target/terminal handle/window snapshot.

### Lower engine/private compatibility
- `crates/chatminal-host-runtime/*`
  - lower engine host internals; được phép giữ `Mux/Tab/Pane`.
  - Cleanup mới nhất đã bỏ `SpawnTarget.attach/detach/state`, `spawnable`, `target_label`, `iter_targets`, `target_was_detached`, rồi đổi naming lõi sang `SpawnTarget`.
  - `lib.rs` đang tách dần control-plane khỏi `Mux` qua `HostRuntimeControlPlane`:
    - giữ `primary_spawn_target`
    - giữ subscribers
    - giữ client/identity/focus metadata
    - để registry/tab/pane/PTY path tiếp tục nằm riêng
  - ownership hiện tại trong crate này:
    - `HostRuntimeRoot` là owner thật
    - `Mux` là compat facade để caller cũ migrate dần
    - compat helper names còn tồn tại, nhưng access path singleton đã hạ xuống weak-root lookup thay vì static strong `Arc<Mux>`
  - root config reads mới nhất cũng đã được gom thêm vào helper boundary:
    - `default_exit_behavior()`
    - `default_workspace_name()`
- `crates/chatminal-config/*`
  - config/runtime boundary giờ cũng dùng vocabulary `target`.
  - public fields đã đổi sang `exec_targets`, `wsl_targets`, `ssh_targets`, `unix_targets`, `default_target`, `default_mux_server_target`.
  - Lua helper đổi sang `default_wsl_targets()` và `exec_target(...)`.
- `crates/chatminal-lua-bridge/src/lib.rs`
  - Lua/config bridge theo vocabulary Chatminal.
  - các helper public để query/chọn execution target đã bị cắt khỏi public surface.
  - `terminal.get_target_name` và public execution-target override cho `spawn_window` / `spawn_session` / `split` cũng đã bị cắt.
- `apps/chatminal-desktop/src/desktop_commands.rs`
  - compatibility translation layer cho upstream `KeyAssignment` names.
  - active `SpawnSession` đã thành single-flow action không còn payload target.

## Architectural ownership
- Product source of truth: `chatminal-runtime`.
- Desktop source of truth trong app layer: `apps/chatminal-desktop/src/chatminal_runtime/*`.
- Render/input shell: `termwindow/*`.
- Engine/private host zone: `desktop_host_runtime/*` + `crates/chatminal-host-runtime/*`.

## Verification snapshot
- `cargo check --workspace`: pass
- `cargo check --workspace --all-targets`: pass
- `cargo test --workspace --lib --bins --tests`: pass
- `cargo test -p chatminal-runtime -- --test-threads=1`: pass
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`: pass
- `make window`: smoke pass

## High-signal improvements (Phase 2 summary)
- **Phase 2.1**: 17 Runtime* types unified as chatminal-protocol type aliases; `api/protocol.rs` (431 LOC) deleted; 5 Store→Protocol From impls moved to chatminal-store. Net: -431 LOC conversion boilerplate, zero redundant types at runtime boundary.
- **Phase 2.2-2.3**: Engine split fallback path fully sealed; `split_terminal_handle*` functions deleted from desktop_host_runtime; 4 dead functions removed; ~33 LOC cleaned; type aliases consolidation (HostSplitSource, HostRuntimeEntryId, HostLayoutNode, HostSplitDirection removed).
- **Phase 1 completions**: `OverlayRenderScope` boundary fully isolated; session vocabulary unified; 4 SSH/tmux/remote crates deleted; chatminal-mux binary deleted; ~3GB WezTerm reference snapshot deleted.
- **Phase 2.4**: Workspace layout persistence confirmed via `set_string_state`/`get_string_state` key-value store with auto-save to app_state table.

## Remaining engineering debt (intentional, post-Phase 2)
- **Tab split functions**: `tab.rs` split_and_insert/compute_split_size cannot be removed (lua-bridge calls Mux::split_pane → SpawnTarget::split_pane → tab). Desktop-only uses WorkspaceLayoutState; daemon/lua still need engine split support.
- **Engine split at lower private host layer**: Mux/Tab/Pane vẫn còn trong `chatminal-host-runtime` như implementation detail; lua-bridge compat path vẫn có thể đụng phần này.
- **Command/config compatibility**: `desktop_commands.rs` still translates upstream `KeyAssignment::*Tab*` for config backward-compat (not exercised by desktop product, only preserved for old configs).

## Phase 05 Final Closeout (2026-04-03)
- `crates/chatminal-host-runtime/src/lib.rs` no longer treats `Mux` as the runtime owner in product init/shutdown; ownership now hangs off the installed host runtime root.
- Default PTY/local spawn product path moved from `mux_default()` to `host_default()` hooks in host-runtime and desktop spawn seams.
- Remaining config sectioning / singleton replacement work and crate rename work are explicitly moved to `plans/260403-1800-post-unification-followups/plan.md`.
- Current closeout check: when asked whether `260401-0949-architecture-unification` is done, use the phase-05 checklist plus source grep, not stale phase status.
