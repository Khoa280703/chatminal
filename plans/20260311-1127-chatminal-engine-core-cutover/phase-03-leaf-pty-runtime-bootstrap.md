# Phase 03 - Leaf PTY Runtime Bootstrap

## Context Links
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_threads.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_command.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_registry.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/session.rs

## Overview
- Priority: P0
- Status: completed
- Brief: dựng leaf runtime thật bằng PTY + parser + terminal state nội bộ để Phase 04 có execution core không phụ thuộc `mux::Pane`

## Key Insights
- Không thể nhảy thẳng sang cutover command nếu chưa có live runtime handle thật cho từng `leaf_id`
- `SessionCoreState` nên tiếp tục là metadata/source of truth nhẹ; handle sống cần tách riêng trong registry
- Fix macOS/zsh cho prompt width phải được carry sang runtime mới ngay từ bootstrap để không tái tạo bug cũ khi cutover

## Requirements
- Functional: spawn được PTY leaf thật, đọc output, ghi input, resize, kill và parse vào terminal state riêng
- Non-functional: không đổi desktop/runtime path hiện tại; test hiện có phải pass

## Architecture
- `LeafRuntime` giữ PTY master/child, input queue và `chatminal-terminal-core::Terminal`
- `LeafRuntimeRegistry` giữ live handles theo `leaf_id`, đồng bộ `LeafProcessState` vào `SessionCoreState`
- `StatefulSessionEngine` giữ registry handle song song với `SessionCoreState` để Phase 04 dùng lại trực tiếp
- `leaf_runtime_command` mang command bootstrap helper và macOS/zsh shim cho execution path mới

## Related Code Files
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_threads.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_command.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/leaf_runtime_registry.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_core_state.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/session_engine.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/Cargo.toml

## Current Progress
- Đã tạo `LeafRuntimeSpawn`, `LeafRuntime`, `LeafRuntimeEvent`
- Đã có PTY bootstrap thật: spawn/read/write/resize/kill/wait loop
- Đã parse output vào `chatminal-terminal-core::Terminal` và expose screen/cursor snapshot
- Đã tách `LeafRuntimeRegistry` để giữ live handles và sync process metadata vào `SessionCoreState`
- Đã carry fix `TERM` + `COLUMNS/LINES` cleanup + macOS/zsh startup shim sang command bootstrap mới
- Đã cắm registry handle vào `StatefulSessionEngine` nhưng chưa chuyển command path sang runtime mới
- Đã thêm unit/integration-style tests cho output capture, resize, process metadata sync và cleanup

## Todo List
- [x] Bootstrap PTY leaf runtime thật trong `chatminal-session-runtime`
- [x] Parse output vào terminal core riêng của session engine mới
- [x] Thêm live runtime registry tách khỏi `SessionCoreState`
- [x] Carry fix macOS/zsh startup shim sang execution path mới
- [ ] Chuyển spawn/focus/split/close command path sang `LeafRuntimeRegistry` + session engine ở Phase 04
- [ ] Nối event/subscription render path trực tiếp vào runtime mới ở Phase 05

## Success Criteria
- Có thể spawn một leaf runtime thật độc lập khỏi `mux::Pane`
- Leaf runtime lưu được terminal state và process metadata riêng
- Không có regression ở `chatminal-session-runtime` và `chatminal-desktop` tests

## Risk Assessment
- PTY lifecycle hiện mới bootstrap theo từng leaf; split/focus tree orchestration vẫn còn nằm ở adapter cũ
- Chưa có auto-eviction khỏi registry khi process tự exit; Phase 04/05 sẽ cần nối event handling để cleanup runtime lifecycle đầy đủ

## Security Considerations
- Command bootstrap mới vẫn inherit env theo `portable_pty::CommandBuilder`; chỉ loại `COLUMNS/LINES` và thêm shim cần thiết
- Không thêm network/service boundary mới; chỉ là in-process runtime handle

## Next Steps
- Phase 04: thay `SessionSpawnManager` và command operations bằng session engine thật dùng `LeafRuntimeRegistry`
- Phase 05: desktop tiêu thụ event/snapshot trực tiếp từ runtime mới thay vì `mux` snapshot
