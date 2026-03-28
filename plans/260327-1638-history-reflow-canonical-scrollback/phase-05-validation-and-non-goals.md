# Phase 05 - Validation And Non Goals

## Context Links
- [plan.md](./plan.md)
- Các phase 01-04

## Overview
- Priority: P1
- Status: in_progress
- Brief: khóa regression bằng test + manual checklist, rồi dọn dead code và chốt non-goals.

## Key Insights
- Bug này đụng runtime, persistence, restore, resize lifecycle và compat rollout cùng lúc.
- Nếu không test mixed-source session riêng, rất dễ pass happy path nhưng vỡ rollout thật.

## Requirements
- Functional requirements:
  1. Có test cho canonical writer/reducer.
  2. Có test cho mixed-source merge reader.
  3. Có test cho restore ở width khác và resize sau reopen.
  4. Có manual checklist cho shell thường, joined sessions, switch profile, offline/online.
  5. Non-goal alt-screen/TUI được viết rõ.
- Non-functional requirements:
  1. Giữ compile/test xanh.
  2. Không để debug logs và compat shims thừa sau cutover.

## Architecture
- Test ưu tiên ở `chatminal-runtime` và `chatminal-store`.
- Desktop manual validation tập trung vào reopen/resize/join/profile switching.
- Explicit non-goals của release đầu:
  - alt-screen/TUI exact replay
  - full VT recorder semantics
  - main-screen multi-line redraw/progress UIs dùng cursor-motion nhiều dòng
  - perfect migration lossless từ legacy wrapped text

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state/tests.rs`
  - `crates/chatminal-store/src/lib.rs` tests
  - test files desktop liên quan restore/resize nếu đã có harness phù hợp
  - docs nếu rollout policy được chốt trong release này

## Implementation Steps
1. Thêm unit tests cho reducer + canonical writer.
2. Thêm unit/integration tests cho mixed-source merge.
3. Thêm integration tests cho reopen ở width khác và resize sau reopen.
4. Manual validate trong desktop app.
5. Cleanup dead code, fallback reader/writer thừa, debug logs.
6. Update docs nếu cutover đủ hoàn tất.

## Todo List
- [x] Unit tests reducer semantics tối thiểu.
- [x] Unit tests canonical writer.
- [x] Unit tests mixed-source merge.
- [x] Unit tests multi-record same-seq ordering.
- [x] Integration tests reopen/resize.
- [x] Manual desktop checklist.
- [x] Cleanup dead code.

## Validation Notes
- Automated validation currently covers runtime/store semantics that underpin reopen/resize:
  - canonical writer
  - mixed-source merge
  - restore snapshot contract
  - logical-line retention
- Desktop smoke launch đã được chạy trong session này:
  - app boot thành công sau `make clean-data`
  - process không crash ngay sau startup
- Manual UI sign-off steps được ghi ở [manual-validation-checklist.md](./manual-validation-checklist.md).

## Success Criteria
- `cargo check --workspace` pass.
- `cargo test -p chatminal-runtime` pass.
- `cargo test --manifest-path crates/chatminal-store/Cargo.toml` pass.
- Manual: reopen ở width khác và resize sau reopen đều đúng.
- Manual: mixed-source session không mất/duplicate history.

## Risk Assessment
- Regression input/title/cursor do restore pipeline.
- Regression preview semantics do API split.
- Giảm thiểu bằng giữ lại bộ test strip volatile sequences đã có và thêm tests mixed-source.

## Security Considerations
- Xác minh sanitization vẫn giữ nguyên behavior cho title/cursor/private mode stripping.

## Next Steps
- Nếu phase này xanh thì mới cân nhắc plan riêng cho alt-screen/TUI fidelity.

## Unresolved Questions
- Không có.
