---
title: "Phase 08 — Final Cleanup: Single Dependency Chain"
status: completed
priority: P1
effort: 0.5d
blocked_by: Phase 07
completed: 2026-03-16
---

# Phase 08 — Final Cleanup

## Goal

Sau phase này: **crate `chatminal-session-runtime` bị xóa hoàn toàn**. Code execution engine được move vào `desktop_host_runtime`. Dependency chain cuối cùng:

```
chatminal-desktop
  └─ chatminal-runtime       (1 core duy nhất — product state + API)
  └─ desktop_host_runtime    (private engine — PTY + render)
       └─ chatminal-host-runtime  (terminal renderer library)

crates/chatminal-session-runtime/ → ĐÃ XÓA
```

## Dead code cần xóa trong `chatminal-session-runtime`

Sau Phase 07 move workspace layout types sang `chatminal-runtime`, các files này thành dead:

| File | Lý do xóa |
|---|---|
| `workspace_layout.rs` | Đã move sang `chatminal-runtime` |
| `workspace_layout_rebuild.rs` | Đi kèm với workspace_layout |
| `workspace_layout_registry.rs` | Đi kèm với workspace_layout |
| `workspace_layout_tests.rs` | Đi kèm với workspace_layout |
| `workspace_host.rs` | SessionWorkspaceHost — thay bằng DaemonState |
| `session_snapshot.rs` | `SessionRuntimeLookup` — thay bằng DaemonState query |

Kiểm tra trước khi xóa: `grep -rn "workspace_layout\|workspace_host\|SessionRuntimeLookup" apps/ crates/chatminal-runtime/ crates/chatminal-desktop/` — nếu còn dùng ở ngoài session-runtime thì phải migrate trước.

## Bước 1 — Move execution code vào `desktop_host_runtime`

Files còn lại trong `chatminal-session-runtime` sau Phase 07 (workspace_layout đã move):

| File | Move đến |
|---|---|
| `engine_runtime_adapter.rs` | `desktop_host_runtime/engine_runtime_adapter.rs` (đã có, merge/update) |
| `leaf_runtime*.rs` (4 files) | `desktop_host_runtime/leaf_runtime/` |
| `session_core_state.rs` | `desktop_host_runtime/session_core_state.rs` |
| `session_engine*.rs` (3 files) | `desktop_host_runtime/session_engine/` |
| `session_event_bus.rs` | `desktop_host_runtime/session_event_bus.rs` |
| `session_focus_manager.rs` | `desktop_host_runtime/session_focus_manager.rs` |
| `session_ids.rs` | `desktop_host_runtime/session_ids.rs` |
| `session_layout_tree.rs` | `desktop_host_runtime/session_layout_tree.rs` |
| `session_spawn_manager.rs` | `desktop_host_runtime/session_spawn_manager.rs` |
| `runtime_bridge.rs` | `desktop_host_runtime/runtime_bridge.rs` |
| `session_runtime_state.rs` | `desktop_host_runtime/session_runtime_state.rs` |
| `session_snapshot.rs` | Xóa (thay bằng DaemonState query từ Phase 05) |
| `workspace_host.rs` | Xóa (thay bằng DaemonState) |
| `session_core_ids.rs` | `desktop_host_runtime/session_core_ids.rs` |

## Bước 2 — Xóa crate và cắt dependencies

```toml
# apps/chatminal-desktop/Cargo.toml — XÓA:
chatminal-session-runtime = { path = "../../crates/chatminal-session-runtime" }

# Cargo.toml workspace members — XÓA:
"crates/chatminal-session-runtime",
```

Sau đó xóa thư mục: `rm -rf crates/chatminal-session-runtime/`

## Bước 3 — Update imports

```rust
// Trước:
use chatminal_session_runtime::WorkspaceLayoutState;

// Sau (layout types từ Phase 07):
use chatminal_runtime::WorkspaceLayoutState;

// Sau (engine types):
use crate::desktop_host_runtime::SessionCoreState;
```

## Grep gates — must all return 0

```bash
# 1. chatminal-runtime không depend session-runtime (từ Phase 07)
grep "chatminal-session-runtime" crates/chatminal-runtime/Cargo.toml

# 2. chatminal-desktop không depend session-runtime trực tiếp
grep "chatminal-session-runtime" apps/chatminal-desktop/Cargo.toml

# 3. Desktop code (ngoài desktop_host_runtime) không import session-runtime trực tiếp
grep -rn "chatminal_session_runtime" apps/chatminal-desktop/src/ \
  --include="*.rs" | grep -v "desktop_host_runtime/"

# 4. workspace_layout files đã xóa
ls crates/chatminal-session-runtime/src/workspace_layout* 2>/dev/null
```

## Build + test gate

```bash
cargo check --workspace
cargo check --workspace --all-targets
cargo test --workspace -- --test-threads=1
```

## Success criteria

- `crates/chatminal-session-runtime/` **không còn tồn tại**
- `Cargo.toml` workspace members không còn `chatminal-session-runtime`
- Developer muốn thêm feature mới → chỉ cần đọc `crates/chatminal-runtime/`
- Không còn file dead trong bất kỳ crate nào
- `cargo check --workspace --all-targets` pass không warning

## Risk

- Desktop code có thể import nhiều types từ session-runtime hơn dự kiến → cần audit kỹ trước khi xóa Cargo.toml dependency
- Một số types cần re-export từ `chatminal-runtime` để desktop vẫn compile sau khi cắt dependency
