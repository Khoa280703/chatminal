# Phase 06 — Dead Code Deletion + Verification

**Status:** completed
**Priority:** P2
**Effort:** 0.5d
**Blocked by:** Phase 03 + 04 + 05

## Goal

Xóa toàn bộ dead code liên quan đến `HostRenderScope` làm trung gian. Chạy full build + test suite để xác nhận baseline mới. Cập nhật docs.

## Context links

- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs` — các methods còn lại dùng `HostRenderScope`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs` — `render_scope_for_runtime`, các `HostRenderScope` usage còn sót
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs` — `desktop_render_scope_id_for_session` (đã deprecated Phase 01)
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` — `remove_runtime_entry_scope`, `activate_host_runtime_entry`, helper functions dùng HostRenderScope
- `docs/system-architecture.md` — update Layer 2/3 description
- `docs/codebase-summary.md` — update high-signal modules

## Deletion checklist

### `engine_runtime_adapter.rs`
- [ ] Xóa `render_scope_id_for_session` method
- [ ] Xóa `render_scope_id_for_runtime` method
- [ ] Xóa `EngineRuntimeAdapter::snapshot_runtime` implementation nếu không còn dùng
- [ ] Đơn giản hóa `attach_runtime` — không còn cần lookup HostRenderScope

### `session_host.rs`
- [ ] Xóa `render_scope_for_runtime` method
- [ ] Xóa `snapshot_runtime_from_host` nếu không còn dùng (đã thay bằng core state)
- [ ] Xóa `Arc<HostRenderScope>` creation trong `sync_render_state_for_runtime` (đã làm Phase 03)
- [ ] Xóa `mux_engine()` method nếu chỉ còn dùng ở các operations đã migrate

### `chatminal_runtime/mod.rs`
- [ ] Xóa `desktop_render_scope_id_for_session` (deprecated Phase 01)
- [ ] Xóa import `HostRenderScope` nếu không còn dùng

### `desktop_host_runtime/mod.rs`
- [ ] Xóa `remove_runtime_entry_scope` nếu không còn gọi
- [ ] Xóa `activate_host_runtime_entry` nếu đã thay bằng session-native focus
- [ ] Xóa `resize_host_window_tabs` loop (thay bằng resize trực tiếp qua session panes)
- [ ] Migrate `host_launcher_tabs` sang enumerate từ `DaemonState.sessions` thay vì `HostMux.get_window().iter()` — **bắt buộc trước khi xóa HostRenderScope**, vì `overlay/launcher.rs:74` vẫn gọi hàm này trực tiếp

### Grep gate — must return 0 results outside `desktop_host_runtime/`
```bash
grep -rn "HostRenderScope\|render_scope_id_for_session\|render_scope_for_runtime\|host_runtime::tab::Tab" \
  apps/chatminal-desktop/src/ --include="*.rs" \
  | grep -v "desktop_host_runtime/"
```

## Build + test gate

```bash
cargo check --workspace
cargo check --workspace --all-targets
cargo test -p chatminal-runtime -- --test-threads=1
cargo test -p chatminal-session-runtime -- --test-threads=1
cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1
cargo test --manifest-path apps/chatminald/Cargo.toml -- --test-threads=1
```

Tất cả phải pass trước khi đóng phase.

## Docs updates

### `docs/system-architecture.md`
Xóa Layer 3 description cũ:
```
Layer 3 (render compat, HostMux):
  HostRenderScope (Tab) — 1:1 với mỗi session
```
Thay bằng:
```
desktop render:
  WorkspaceLayout → session_id → DesktopSessionHost.pane(session_id) → GPU draw
```

Update "Remaining intentional compatibility" section — `HostRenderScope` không còn trong danh sách.

### `docs/codebase-summary.md`
Update `Desktop private adapter` section — bỏ `session_pane.rs` mô tả cũ nếu đã refactor.

## Todo

- [x] Chạy grep audit lần cuối để tìm dead code còn sót
- [x] Xóa từng item trong deletion checklist (theo thứ tự: engine_runtime_adapter → session_host → chatminal_runtime/mod → desktop_host_runtime/mod)
- [x] Sau mỗi xóa: `cargo check --workspace` để bắt lỗi sớm
- [x] Chạy full grep gate — phải 0 results
- [x] Chạy full build + test suite — tất cả pass
- [x] Update `docs/system-architecture.md`
- [x] Update `docs/codebase-summary.md`

## Success criteria

- Grep gate: 0 kết quả ngoài `desktop_host_runtime/`
- Full test suite pass
- Docs phản ánh đúng architecture mới
- `HostRenderScope` chỉ còn tồn tại như type alias trong `desktop_host_runtime/mod.rs` (để overlay compat giữ nguyên) — không được tạo instance nào trong session spawn/render path

## Risk

- Một số overlay code (`confirm.rs`, `launcher.rs`) dùng `OverlayRenderScope = Tab` — đây là intentional compat, KHÔNG xóa. Phân biệt rõ `HostRenderScope` (bị xóa) với `OverlayRenderScope` (giữ).
- `host_launcher_tabs` **đã xác nhận** còn được gọi tại `overlay/launcher.rs:74` — phải migrate trong phase này, không thể defer.
