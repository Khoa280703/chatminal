# Phase 03 - Delete Terminal Core And Collapse Deps

## Context Links
- [phase-02-cut-session-engine-off-terminal-core.md](/Users/khoa2807/development/2026/chatminal/plans/260404-1532-merge-terminal-core-and-emulator/phase-02-cut-session-engine-off-terminal-core.md)
- [crates/chatminal-terminal-core/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-terminal-core/src/lib.rs)
- [crates/chatminal-terminal-emulator/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-terminal-emulator/src/lib.rs)

## Overview
- Priority: P1
- Status: done
- Brief: xóa layer cũ khỏi steady-state architecture sau khi product path đã cutover.

## Key Insights
- Search hiện tại cho thấy active usage của `chatminal-terminal-core` chỉ còn `TerminalSize` lane ở desktop runtime.
- Khi Phase 02 xong, crate này nhiều khả năng chỉ còn dead API + docs lịch sử.
- Nếu không xóa hẳn, repo sẽ tiếp tục phát sinh code mới bám vào lớp cũ chỉ vì nó còn tồn tại.

## Requirements
- Remove dependency edges tới `chatminal-terminal-core` khỏi active packages.
- Delete crate; không giữ lại adapter crate/song song layer dưới bất kỳ tên gọi mềm nào sau closeout.
- Update Cargo manifests, workspace membership, và docs active scope.

## Architecture
- Steady-state chỉ còn một terminal crate cho active product path: `chatminal-terminal-emulator`.
- Lightweight type surface nếu vẫn cần phải nằm trong emulator crate hoặc local private module của consumer, không tồn tại như crate riêng.
- Không chấp nhận trạng thái “đã merge nhưng vẫn còn core làm adapter”.

## Related Code Files
- Modify: workspace `Cargo.toml`
- Modify: package manifests đang còn dependency `chatminal-terminal-core`
- Delete: `crates/chatminal-terminal-core/Cargo.toml`
- Delete: `crates/chatminal-terminal-core/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/system-architecture.md`
- Modify: `docs/codebase-summary.md`
- Modify: `docs/index.md`

## Implementation Steps
1. Dùng `cargo tree` và `rg` để chắc chắn dependency `chatminal-terminal-core` không còn trong active graph.
2. Gỡ dependency khỏi manifests liên quan.
3. Xóa crate `crates/chatminal-terminal-core`.
4. Dọn docs active scope để không còn mô tả dual-layer.
5. Grep lại toàn repo cho `chatminal-terminal-core` và xử lý residual active references.

## Todo List
- [x] Active manifests không còn depend vào `chatminal-terminal-core`
- [x] Workspace không còn member `crates/chatminal-terminal-core`
- [x] Crate `chatminal-terminal-core` bị xóa
- [x] Active docs được sync với reality mới
- [x] Residual mentions chỉ còn trong archive/history khi có chủ đích

## Success Criteria
- `cargo tree -p chatminal-desktop | rg 'chatminal-terminal-core'` trả về zero.
- `rg -n "chatminal-terminal-core|chatminal_terminal_core" README.md docs apps crates` chỉ còn archive/comment được chấp nhận rõ ràng.
- Không còn crate-level duplicate terminal layer trong workspace active path.
- Không còn bất kỳ adapter crate/song song contract nào duy trì `core` ở steady-state.

## Risk Assessment
- Risk: xóa crate xong mới phát hiện external/internal test lane còn dùng.
- Mitigation: trước khi delete, chạy grep + cargo tree + workspace check có chủ đích.

## Security Considerations
- Không có security delta trực tiếp.
- Cần bảo đảm không mất bất kỳ parser/emulator capability nào của active product path khi dọn crate cũ.

## Next Steps
- Sang Phase 04 để verify closeout và chốt naming debt còn lại.
