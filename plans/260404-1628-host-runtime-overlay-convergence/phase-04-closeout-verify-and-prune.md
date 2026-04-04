---
phase: 04
status: completed
priority: medium
effort: small
risk: low
---

# Phase 04: Closeout Verify And Prune

## Context Links
- [plan.md](./plan.md)
- [docs/system-architecture.md](/Users/khoa2807/development/2026/chatminal/docs/system-architecture.md)
- [docs/codebase-summary.md](/Users/khoa2807/development/2026/chatminal/docs/codebase-summary.md)
- [docs/project-changelog.md](/Users/khoa2807/development/2026/chatminal/docs/project-changelog.md)

## Overview
- Priority: P2
- Current status: completed
- Mục tiêu: xác nhận 3 phase trước đã thực sự cắt seam, xóa dead helpers/test seams thừa, và sync docs active scope theo reality mới.

## Key Insights
- Cleanup kiến trúc chỉ hoàn tất khi grep, tests, docs và dead helpers cùng phản ánh steady-state mới.
- Đây không phải phase docs-only; nó là gate để tránh sót adapter/wrapper một-hop còn sống.

## Requirements
- Chỉ dọn những gì được unlock bởi phase 1-3.
- Không mở rộng scope sang history/config compat.
- Changelog/docs phải mô tả rõ residual seams nào còn intentional.

## Architecture
- Repo steady-state sau closeout phải đọc được như: desktop host canonical, host-runtime root-native, overlay shell contract canonical.
- Bất kỳ compat alias còn giữ phải explicit, nhỏ, và được ghi rõ là test-only/private.

## Related Code Files
- Modify: docs active scope
- Modify: residual helper/test files unlocked by previous phases
- Delete: dead wrappers, dead aliases, dead imports, stale tests/docs references

## Implementation Steps
1. Grep repo để tìm residual `legacy_*`, `MuxHandle`, `mux_default()`, `overlay_compat` trong active scope.
2. Xóa dead helpers/imports/tests unlocked bởi cutover.
3. Sync `docs/system-architecture.md`, `docs/codebase-summary.md`, `docs/project-changelog.md`, roadmap nếu cần.
4. Chạy verification spine đầy đủ và lưu kết quả trong closeout note của plan.

## Todo List
- [x] Repo-wide grep seam residuals
- [x] Xóa deadcode còn sót
- [x] Sync docs active scope
- [x] Chạy full verification spine
- [x] Ghi closeout note vào plan khi hoàn tất

## Success Criteria
- Grep active scope không còn seam ngoài phần explicit private/test-only đã được chấp nhận.
- Docs active scope mô tả đúng reality mới.
- Verification spine pass toàn bộ.

## Risk Assessment
- Nguy cơ chính là quên dọn residual nhỏ làm docs và code lệch nhau.

## Security Considerations
- Không thay behavior product; chỉ verify cleanup không làm gãy lifecycle/resource handling.

## Closeout Note
- Grep active scope còn `overlay_shell` canonical owner ở `desktop_host_runtime/mod.rs`; không còn `overlay_compat`, `MuxHandle`, `legacy_*mux`, `legacy_host_client_slot`, `legacy_host_workspace_slot` trong active source.
- Docs active scope đã sync trong:
  - `docs/system-architecture.md`
  - `docs/codebase-summary.md`
  - `docs/project-changelog.md`
- Verification thực thi cho closeout:
  - `cargo check -p chatminal-desktop`
  - `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml desktop_host_runtime::session_host -- --test-threads=1`
  - `cargo check -p chatminal-host-runtime`
  - `cargo test -p chatminal-host-runtime --lib -- --test-threads=1`
  - `cargo check --workspace`
  - `cargo test --workspace --lib --bins --tests`
  - `git diff --check`
  - `make window` smoke: build/start thành công; process được terminate chủ động sau timeout để không giữ GUI session mở

## Next Steps
- Wave này đã closeout xong; phần còn lại nếu có là cleanup lịch sử/docs cũ ngoài scope plan này.
