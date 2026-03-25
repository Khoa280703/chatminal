# System Architecture

Last updated: 2026-03-25 (single-flow desktop/spawn-target cleanup complete)

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

## Latest changes (Phase 2 complete, 2026-03-17)
- **Phase 2.1 - Type alias consolidation**: 17 Runtime* types (RuntimeSession, RuntimeProfile, etc.) are now **direct type aliases** to `chatminal-protocol` types. Deleted `api/protocol.rs` (431 LOC) conversion boilerplate; moved 5 Store→Protocol From impls to `chatminal-store`.
- **Phase 2.2 - Engine split fallback removal**: Deleted `split_terminal_handle` + `split_terminal_handle_by_public_id` from `desktop_host_runtime/mod.rs`; removed `HostSplitSource`, `HostRuntimeEntryId`, `HostLayoutNode`, `HostSplitDirection` type aliases; replaced desktop_spawn.rs split fallback (lines 111-131) with `anyhow::bail!` error.
- **Phase 2.3 - Dead code cleanup**: Removed 4 functions (`active_host_target_name`, `set_default_host_target`, `new_headless_connection_ui`, `host_client_targets`); removed 3 type aliases; ~33 LOC cleaned. Note: tab.rs split functions cannot be removed (lua-bridge dependency).
- **Phase 2.4 - Workspace layout persistence**: Already implemented via `set_string_state`/`get_string_state` with key prefix `workspace_layout:`; mutations auto-save to app_state table.
- **Phase 2.5 - Documentation**: Added doc comment to window.rs explaining single-Window/single-Tab desktop model; this file updated.

## Topology

### Desktop app
```text
chatminal-desktop
  -> chatminal_runtime facade
    -> chatminal-runtime
      -> chatminal-session-runtime
        -> workspace_layout + session engine + terminal runtime registry
    -> desktop_host_runtime (private engine adapter only)
      -> chatminal-host-runtime
        -> Mux/Tab/Pane private engine primitives
  -> termwindow render/input shell
  -> chatminal_sidebar + session bar UI
```

### Persistence and compatibility
```text
chatminal-runtime
  -> chatminal-store (SQLite)
  -> profiles / sessions / scrollback / workspace layout state
  -> native_api + runtime_bridge

chatminald / chatminal-app
  -> compatibility boundary only
  -> reuse protocol/store/runtime contracts
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
- `chatminal-session-runtime` giữ live execution model: session engine, runtime registry, focus manager, workspace layout registry.
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
- `cargo test -p chatminal-session-runtime -- --test-threads=1`: pass
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`: pass
- `cargo test --manifest-path apps/chatminald/Cargo.toml -- --test-threads=1`: pass

## Workspace layout persistence
- Layout state persisted via `native_api.rs` `set_string_state`/`get_string_state` with key prefix `workspace_layout:`.
- All mutations (split, close, resize, focus) auto-save to `app_state` table as JSON blob.
- No separate schema migration needed; key-value store handles layout versioning.

## Remaining intentional compatibility
- Engine internals vẫn có `Mux/Tab/Pane` trong `chatminal-host-runtime` và private adapter desktop.
- Command/config compatibility vẫn giữ upstream `KeyAssignment::*Tab*` translation trong `desktop_commands.rs` để không gãy config cũ.
- `OverlayRenderScope` dùng cho launcher/confirm/prompt overlays nhưng không còn coupled với render scope; boundary fully isolated.
- `SessionExecutionStatus` enum thêm vào `chatminal-runtime/state.rs` để track running status.
- Các phần trên là intentional private/compatibility zones, không còn là product-facing architecture.
