# Phase 01 - Freeze Single Terminal Contract

## Context Links
- [README.md](/Users/khoa2807/development/2026/chatminal/README.md)
- [docs/system-architecture.md](/Users/khoa2807/development/2026/chatminal/docs/system-architecture.md)
- [docs/codebase-summary.md](/Users/khoa2807/development/2026/chatminal/docs/codebase-summary.md)
- [plan.md](/Users/khoa2807/development/2026/chatminal/plans/260404-1532-merge-terminal-core-and-emulator/plan.md)

## Overview
- Priority: P1
- Status: done
- Brief: chốt source-of-truth terminal domain trước khi đụng code migration.

## Key Insights
- `chatminal-terminal-core` không còn giữ active behavior quan trọng; active terminal parser/state/input encoding đang ở `engine_term`.
- `chatminal-terminal-core::TerminalSize` chỉ đang tồn tại như một compatibility type shell.
- Nếu không freeze hướng đi ngay từ đầu, team rất dễ rơi vào nửa merge, nửa alias, và kết thúc bằng 2 layer trá hình.
- Hard requirement của wave này: closeout xong phải còn đúng một terminal architecture trong steady-state source.

## Requirements
- Chốt một terminal crate canonical duy nhất cho active product path.
- Chốt rõ các type nào sẽ canonical: ít nhất `TerminalSize`, `TerminalConfiguration`, cursor/screen-facing contracts nếu còn dùng.
- Cấm tạo crate thứ ba chỉ để chứa type nhẹ.

## Architecture
- Canonical owner: `crates/chatminal-terminal-emulator`
- Boundary rule:
  - behavior/state/input/output terminal nằm ở emulator crate
  - mọi active runtime/session/desktop consumer dùng cùng type system từ emulator crate
- Temporary migration rule:
  - cho phép giữ conversion/shim cục bộ rất ngắn hạn trong một phase
  - không cho phép commit closeout vẫn còn dual public contract hay adapter crate song song

## Related Code Files
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Modify: `apps/chatminal-desktop/src/desktop_host_runtime/execution_bridge.rs`
- Modify: `crates/chatminal-terminal-emulator/src/lib.rs`
- Delete later: `crates/chatminal-terminal-core/*`

## Implementation Steps
1. Audit hết mọi import `chatminal_terminal_core::*` còn active.
2. Chốt mapping canonical:
   - `chatminal_terminal_core::TerminalSize` -> `engine_term::TerminalSize`
   - lightweight contracts khác -> dùng trực tiếp emulator type hoặc local private DTO nếu thực sự chỉ là app-boundary detail
3. Ghi doc comment ngắn ở emulator crate hoặc architecture docs để tránh reintroduce layer thứ hai.
4. Xác định trước callsites cần migration đồng loạt để tránh half-migrated tree.

## Todo List
- [x] Freeze canonical terminal crate = `chatminal-terminal-emulator`
- [x] Freeze canonical `TerminalSize` = `engine_term::TerminalSize`
- [x] Inventory callsites `chatminal_terminal_core::*`
- [x] Decide residual naming debt `engine_term` xử lý ngay hay defer

## Success Criteria
- Có danh sách callsites active cần đổi, không còn mơ hồ.
- Có decision record rõ: merge vào emulator, không sinh crate thứ ba.
- Có decision record rõ: không giữ `chatminal-terminal-core` như adapter/compat layer sau closeout.
- Team có thể làm phase 02-03 mà không tranh luận lại direction.

## Risk Assessment
- Risk: chọn canonical sai crate rồi phải rollback.
- Mitigation: dựa trên active behavior trong source, không dựa trên tên crate hay docs cũ.

## Security Considerations
- Không có thay đổi security-facing trực tiếp.
- Phải giữ nguyên terminal input/output semantics; không được vô tình downgrade escape handling.

## Next Steps
- Sang Phase 02 để đổi session-engine/desktop host khỏi `chatminal-terminal-core`.
