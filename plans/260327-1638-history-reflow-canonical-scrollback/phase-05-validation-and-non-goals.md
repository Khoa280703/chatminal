# Phase 05 - Validation And Non Goals

## Context Links
- [plan.md](./plan.md)
- Các phase 01-04

## Overview
- Priority: P1
- Status: pending
- Brief: khóa regression bằng test + manual checklist, đồng thời chốt non-goal để không phình scope.

## Key Insights
- Bug này dễ tái phát vì đụng runtime, persistence, restore và resize lifecycle cùng lúc.
- Nếu không khóa non-goal, task sẽ trượt thành terminal recorder/full VT replay.

## Requirements
- Functional requirements:
  1. Có test cho persist -> reopen -> resize -> reopen-again.
  2. Có manual checklist cho shell thường, joined sessions, switch profile, offline/online.
  3. Non-goal alt-screen/TUI được viết rõ.
- Non-functional requirements:
  1. Giữ compile/test xanh.
  2. Không để debug logs và compat shims thừa sau cutover.

## Architecture
- Test ưu tiên ở `chatminal-runtime` và `chatminal-store`.
- Desktop manual validation tập trung vào reopen/resize/join/profile switching.

## Related Code Files
- Modify:
  - `crates/chatminal-runtime/src/state/tests.rs`
  - `crates/chatminal-store/src/lib.rs` tests
  - test files desktop liên quan restore/resize nếu đã có harness phù hợp

## Implementation Steps
1. Thêm unit tests cho canonical append/render.
2. Thêm integration tests cho reopen ở width khác.
3. Manual validate trong desktop app.
4. Cleanup dead code, fallback reader/writer thừa, debug logs.
5. Update docs nếu cutover hoàn tất.

## Todo List
- [ ] Unit tests canonical writer.
- [ ] Unit tests width-aware render.
- [ ] Integration tests reopen/resize.
- [ ] Manual desktop checklist.
- [ ] Cleanup dead code.

## Success Criteria
- `cargo check --workspace` pass.
- `cargo test -p chatminal-runtime` pass.
- `cargo test --manifest-path crates/chatminal-store/Cargo.toml` pass.
- Manual: reopen ở width khác và resize sau reopen đều đúng.

## Risk Assessment
- Regression input/title/cursor do restore pipeline.
- Giảm thiểu bằng giữ lại bộ test strip volatile sequences đã có.

## Security Considerations
- Xác minh sanitization vẫn giữ nguyên behavior cho title/cursor/private mode stripping.

## Next Steps
- Nếu phase này xanh thì mới cân nhắc alt-screen/TUI fidelity plan riêng.

## Unresolved Questions
- Không có.
