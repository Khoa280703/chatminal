---
title: "Phase 07 — Single Core Boundary: Đảo chiều dependency"
status: completed
priority: P1
effort: 1d
blocked_by: Phase 05, Phase 06
completed: 2026-03-16
---

# Phase 07 — Single Core Boundary

## Goal

Sau phase này: developer chỉ cần nghĩ đến **`chatminal-runtime`** khi làm feature mới. `chatminal-session-runtime` không còn là crate mà `chatminal-runtime` import trực tiếp ở public API path.

## Vấn đề hiện tại (sau Phase 05-06)

`chatminal-runtime/Cargo.toml` depend vào `chatminal-session-runtime`:
```toml
chatminal-session-runtime = { path = "../chatminal-session-runtime" }
```

Vì vậy `DaemonState` leak types từ session-runtime ra ngoài:
```rust
// chatminal-runtime/state.rs
use chatminal_session_runtime::{
    SessionViewId, WorkspaceLayoutState, WorkspaceNodeId, WorkspaceSplitAxis,
};
```

→ Developer không biết "type này ở đâu" — phải nhớ `WorkspaceLayoutState` từ session-runtime dù đang code trong chatminal-runtime.

## Target: 2 options

### Option A (recommended — ít risk hơn)

Move workspace layout types (`WorkspaceLayoutState`, `WorkspaceNodeId`, `SessionViewId`, `WorkspaceSplitAxis`) vào `chatminal-runtime`. `chatminal-session-runtime` chỉ còn là execution engine private của `desktop_host_runtime`.

```
chatminal-runtime
  ├─ Owns: WorkspaceLayoutState, SessionViewId, WorkspaceNodeId  ← MOVED FROM session-runtime
  ├─ Owns: DaemonState, SessionEntry, ProfileState
  └─ Does NOT import chatminal-session-runtime (hoặc chỉ dev-dependency)

chatminal-session-runtime
  ├─ Chỉ biết bởi: desktop_host_runtime
  └─ Execution engine nội bộ: StatefulSessionEngine, SessionCoreState, PTY threading
```

### Option B (aggressive — final crate merge)

Merge `chatminal-session-runtime` vào `chatminal-runtime`. Một crate duy nhất. Phức tạp hơn vì cần resolve circular dependency (session-runtime dùng types từ host-runtime).

**Chọn Option A cho phase này.**

## Workspace layout types cần move

Từ `chatminal-session-runtime` sang `chatminal-runtime`:
- `WorkspaceLayoutState`
- `WorkspaceLayoutNodeKind`
- `WorkspaceLayoutNodeSnapshot`
- `WorkspaceNodeId`
- `WorkspaceSplitAxis`
- `SessionViewId`
- `SessionViewSnapshot`

Các types này là **product model** (bài toán UI/UX), không phải execution engine — logic chỉ ở chatminal-runtime.

## Types giữ lại ở chatminal-session-runtime

- `StatefulSessionEngine`, `SessionEngineShared`
- `SessionCoreState`, `SessionRuntimeRecord`
- `TerminalInstanceId`, `RuntimeId`, `LayoutNodeId`
- `SessionLayoutSnapshot` (internal engine tree)
- `LeafRuntime`, `TerminalInstanceRuntimeRegistry`

## Implementation steps

1. Tạo `crates/chatminal-runtime/src/workspace_layout.rs` — **copy/move code trực tiếp, KHÔNG re-export từ `chatminal-session-runtime`** (re-export sẽ giữ nguyên dependency, không đạt được success criteria)
2. Move types (copy Rust source, update module paths) — không rewrite logic
3. Update `chatminal-runtime/Cargo.toml` — xóa `chatminal-session-runtime` dependency
4. Update tất cả import sites trong `chatminal-runtime/` và `chatminal-desktop/`
5. **Wire `execution_status` sync** (defer từ Phase 05):
   - Sau khi dependency đảo, `chatminal-session-runtime` có thể depend `chatminal-runtime`
   - `ensure_session_runtime_native` set `execution_status = Running { runtime_id }` vào `DaemonState`
   - `close_runtime_native` set `execution_status = Stopped`
6. `cargo check --workspace` gate
7. Đảm bảo `chatminal-session-runtime` không còn trong public API của `chatminal-runtime`

## Success criteria

- `chatminal-runtime/Cargo.toml` KHÔNG còn `chatminal-session-runtime` dependency
- `DaemonState` API không leak bất kỳ type nào từ `chatminal-session-runtime`
- Developer muốn thêm feature mới chỉ cần đọc `crates/chatminal-runtime/` — không cần biết đến `chatminal-session-runtime`
- `grep -rn "chatminal_session_runtime" crates/chatminal-runtime/src/` → 0 results
- `cargo check --workspace` pass
- `cargo test -p chatminal-runtime` pass

## Grep gate

```bash
# Must return 0
grep -rn "chatminal_session_runtime\|chatminal-session-runtime" \
  crates/chatminal-runtime/src/ crates/chatminal-runtime/Cargo.toml
```

## Risk

- `WorkspaceLayoutState` được dùng nhiều chỗ — rename import path là mechanical nhưng nhiều file
- Nếu session-runtime types có `impl` blocks phức tạp gắn với session engine types → cần tách impl ra; dùng `cargo check` liên tục
- `chatminal-desktop` import nhiều types từ `chatminal-session-runtime` gián tiếp qua `chatminal-runtime` re-exports — cần audit import chain

## Resolved

- `SessionRuntimeRecord` **KHÔNG được promote** vào `SessionEntry` — sẽ gây cycle. Phase 05 chỉ thêm `SessionExecutionStatus` (primitive). `SessionRuntimeRecord` ở lại session-runtime.
- `WorkspaceLayoutState` là product model, không có PTY/thread state → move an toàn vào chatminal-runtime
- Sau Phase 07, `chatminal-runtime` chỉ cần biết workspace layout types — không cần biết execution engine internals
