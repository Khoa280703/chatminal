# Phase 07 - Dependency Prune Delete And Final Verification

## Context Links
- `Cargo.toml`
- `apps/chatminal-desktop/Cargo.toml`
- `crates/chatminal-session-runtime/Cargo.toml`

## Overview
- Priority: P0
- Status: completed
- Brief: xoá dependency, xoá crate không còn downstream, dọn naming cuối và xác nhận zero residual.

## Key Insights
- Chỉ khi bỏ dependency graph và delete code chết thì mới gọi là clean hoàn toàn.

## Requirements
- Remove `mux` khỏi desktop/session-runtime manifests.
- Trace toàn bộ downstream của `crates/chatminal-mux`, migrate hoặc delete hết trong scope plan này.
- Remove `crates/chatminal-mux` khỏi workspace; không chấp nhận kết thúc plan với trạng thái “còn nhưng không dùng trực tiếp”.
- Rename residual `leaf/surface/tab/pane` trong app/public path sang first-party naming.
- Chạy full grep/build/test gates.

## Architecture
- Workspace cuối phải phản ánh đúng runtime hiện tại: desktop/session-runtime first-party, engine core chỉ còn private engine deps thực sự cần.

## Related Code Files
- Refactor: `Cargo.toml`
- Refactor: `apps/chatminal-desktop/Cargo.toml`
- Refactor: `crates/chatminal-session-runtime/Cargo.toml`
- Delete: `crates/chatminal-mux`
- Delete: code/helpers compatibility dead paths phát sinh ở desktop/runtime

## Implementation Steps
1. Trace `cargo tree` và toàn bộ downstream còn phụ thuộc `chatminal-mux`.
2. Remove dep entries và fix compile graph.
3. Delete unused compatibility modules/binaries.
4. Rename remaining public vocabulary.
5. Run grep gates.
6. Run `cargo check --workspace`.
7. Run targeted tests.
8. Mark plan completed only if all zero-residual gates pass.

## Todo List
- [x] Trace all `chatminal-mux` downstream
- [x] Remove `mux` deps
- [x] Remove `chatminal-mux` workspace member
- [x] Delete dead compatibility files
- [x] Rename residual vocabulary
- [x] Run grep/build/test gates

## Current Notes
- Workspace đã chuyển crate nền từ `crates/chatminal-mux` sang `crates/chatminal-host-runtime`.
- Desktop boundary đã bọc `runtime_entry_by_id` / `terminal_by_id` và local overlay layout structs để app path không còn lộ `TabId`, `PaneId`, `PositionedPane`, `PositionedSplit`, `get_tab()`, `get_pane()`.
- Residual `leaf-*` trong runtime/test scoped path đã đổi sang `terminal-instance-*`.
- Gate chạy xong:
  - `cargo check -p chatminal-desktop`
  - `cargo check --workspace`
  - `cargo test -p chatminal-session-runtime -- --test-threads=1`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
  - grep gates scoped desktop/session-runtime/manifests đều zero match

## Success Criteria
- Desktop active path zero `mux`.
- Session-runtime active path zero `mux`.
- Workspace không còn `chatminal-mux`.
- Full gates pass.

## Risk Assessment
- Risk: hidden transitive dependency giữ `mux` sống.
- Mitigation: trace `cargo tree` theo từng crate trước khi delete; nếu còn dependency thì phase chưa được complete.

## Security Considerations
- Phải giữ build reproducibility; không delete nhầm crate còn phục vụ đường chạy khác.

## Next Steps
- Phase complete. Không còn bước nào trong plan này.
