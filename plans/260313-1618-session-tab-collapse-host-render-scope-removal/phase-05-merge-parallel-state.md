# Phase 05 — Merge Parallel State

**Status:** completed
**Priority:** P2
**Effort:** 1d
**Blocked by:** Phase 04

## Goal

Chuẩn bị state structure + simplify adapter. Phase này **không wire sync** (vì dependency direction hiện tại là `chatminal-runtime` → `chatminal-session-runtime` — session-runtime KHÔNG THỂ write ngược lên runtime mà không tạo cycle). Sync wiring thật sự sẽ diễn ra ở Phase 07 sau khi dependency đã được đảo.

Scope của Phase 05:
1. Define `SessionExecutionStatus` enum trong `chatminal-runtime` (type preparation, chưa populate).
2. Simplify `DesktopEngineRuntimeAdapter` — xóa render_scope lookups (đã clean sau Phase 03).
3. **Không** modify write path từ session-runtime → runtime ở phase này.

## Context links

- `crates/chatminal-runtime/src/state.rs` — `DaemonState`, `SessionEntry`
- `crates/chatminal-session-runtime/src/session_core_state.rs` — `SessionCoreState`, `SessionRuntimeRecord`
- `crates/chatminal-session-runtime/src/session_engine.rs` — `StatefulSessionEngine`
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs` — `DesktopEngineRuntimeAdapter` + `EngineRuntimeAdapter` impl
- `crates/chatminal-session-runtime/src/engine_runtime_adapter.rs` — trait definition

## Current dual state

```
Layer 1 — chatminal-runtime:
  DaemonState.sessions: HashMap<String, SessionEntry>
    SessionEntry { metadata, workspace_layout_state, ... }

Layer 2 — chatminal-session-runtime:
  SessionCoreState.runtimes: HashMap<RuntimeId, SessionRuntimeRecord>
    SessionRuntimeRecord { session_id, layout, terminal_instances, ... }
  session_to_runtime: HashMap<String, RuntimeId>
```

**Overhead**: session activate → update Layer 1 → call through `EngineRuntimeAdapter` → update Layer 2. Close session → hai nơi phải đồng bộ.

## Target state

```
DaemonState.sessions: HashMap<String, SessionEntry>
  SessionEntry {
    metadata: StoredSession,
    // Thêm:
    execution_status: SessionExecutionStatus,  // owned by chatminal-runtime
  }

// Định nghĩa trong crates/chatminal-runtime/src/state.rs:
pub enum SessionExecutionStatus {
    NotStarted,
    Running { runtime_id: u64 },   // u64 primitive — không import chatminal-session-runtime
    Stopped,
}
```

**Tại sao Phase 05 KHÔNG wire sync:**
- Dependency hiện tại: `chatminal-runtime` → `chatminal-session-runtime` (Cargo.toml:9)
- Nếu session-runtime muốn write `execution_status` vào DaemonState → session-runtime phải depend runtime → **cycle**
- Phase 07 đảo dependency: cắt `chatminal-runtime`'s dep vào `chatminal-session-runtime`
- Sau Phase 07: session-runtime (tạm còn tồn tại) có thể depend runtime một chiều → write hợp lệ
- Sync wiring thật sự sẽ implement ở Phase 07

**Layer boundary rules (bất biến):**
- `chatminal-runtime` chỉ được chứa pure data types (primitive, Serialize/Deserialize)
- `SessionRuntimeRecord` ở lại `chatminal-session-runtime` — internal execution bookkeeper
- `Arc<ChatminalSessionPane>` mapping chỉ sống trong `DesktopSessionHost`

`SessionCoreState` vẫn tồn tại như internal execution bookkeeper (giữ terminal thread handles, PTY handles) nhưng **không còn là source of truth** cho layout/focus state — đó là `DaemonState.sessions`.

## What changes in EngineRuntimeAdapter

Sau Phase 02+03, `DesktopEngineRuntimeAdapter` chỉ còn cần thiết cho:
- `spawn_runtime` → gọi `HostMux.spawn_tab_or_window` (bootstrap PTY)
- `close_runtime` → dọn HostMux
- `focus_runtime` / `focus_terminal_instance` → HostMux focus (vẫn cần vì overlay compat)
- `snapshot_runtime` → đã thay bằng `SessionRuntimeState` từ core

Simplification: các method chỉ cần `HostMux` operations, không còn phải dùng `HostRenderScope` làm trung gian (đã xóa ở Phase 03).

## Implementation steps

1. **Thêm `SessionExecutionStatus` enum** vào `crates/chatminal-runtime/src/state.rs` — chỉ primitive types, KHÔNG import `chatminal-session-runtime`.
2. **Thêm `execution_status: SessionExecutionStatus`** vào `SessionEntry`. Default: `NotStarted`. (Chưa populate từ execution layer — sẽ wire ở Phase 07.)
3. **Đơn giản hóa `DesktopEngineRuntimeAdapter`**:
   - Xóa `render_scope_id_for_session`, `render_scope_id_for_runtime` (không còn dùng sau Phase 03)
   - Giữ `spawn_runtime_inner`, `close_runtime`, `focus_*` với HostMux calls đơn giản hơn
4. Confirm `StatefulSessionEngine<DesktopEngineRuntimeAdapter>` (mux_engine) chỉ còn dùng ở legacy flows (`split_terminal_instance`, ...) — tất cả sẽ bị migrate/xóa ở Phase 06.
5. `cargo test -p chatminal-runtime -- --test-threads=1` pass.
6. `cargo test -p chatminal-session-runtime -- --test-threads=1` pass.

## Related code files

**Sửa:**
- `crates/chatminal-runtime/src/state.rs` — thêm `SessionExecutionStatus` enum và field `execution_status` vào `SessionEntry`
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs` — simplify

**Không sửa trong Phase 05 (defer sang Phase 07):**
- `crates/chatminal-session-runtime/src/session_engine_core.rs` — sync write `execution_status` vào DaemonState sẽ làm ở Phase 07 sau khi dependency đảo

**Không sửa:**
- `SessionCoreState` struct (giữ cho PTY/thread handles) — chỉ thay role của nó

## Todo

- [x] Thêm `SessionExecutionStatus` enum vào `crates/chatminal-runtime/src/state.rs`
- [x] Thêm `execution_status` field vào `SessionEntry` (default `NotStarted`)
- [x] Simplify `DesktopEngineRuntimeAdapter` — xóa render_scope lookups
- [x] `cargo test -p chatminal-runtime` pass
- [x] `cargo test -p chatminal-session-runtime` pass

*(Sync wiring `execution_status` từ execution layer → Phase 07)*

## Success criteria

- `SessionExecutionStatus` enum tồn tại trong `chatminal-runtime/src/state.rs` — không import session-runtime types
- `DesktopEngineRuntimeAdapter` không còn reference `HostRenderScope`
- `chatminal-runtime/Cargo.toml` vẫn depend `chatminal-session-runtime` — bình thường, Phase 07 mới cắt
- Cả hai test suites pass

## Risk

- Thay đổi `SessionEntry` là breaking cho tất cả code đọc `DaemonState.sessions` → dùng `cargo check` liên tục.

## Resolved

**`SessionRuntimeRecord` KHÔNG được promote trực tiếp vào `SessionEntry`.** Mặc dù struct thuần data không có circular reference, nhưng nó định nghĩa ở `chatminal-session-runtime`. Nếu `SessionEntry` (chatminal-runtime) import type đó → `chatminal-runtime` depend `chatminal-session-runtime` → cycle với dependency hiện có (session-runtime → runtime).

Giải pháp: Dùng `SessionExecutionStatus` enum primitive (u64 runtime_id) — đủ để track running state mà không cần import session-runtime types. `SessionRuntimeRecord` ở lại nội bộ `chatminal-session-runtime`.
