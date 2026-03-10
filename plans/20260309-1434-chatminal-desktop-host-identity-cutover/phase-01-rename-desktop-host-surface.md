# Phase 01 Rename Desktop Host Surface

## Context Links
- [README.md](/Users/khoa2807/development/2026/chatminal/README.md)
- [Cargo.toml](/Users/khoa2807/development/2026/chatminal/Cargo.toml)
- [apps/chatminal-app/src/main.rs](/Users/khoa2807/development/2026/chatminal/apps/chatminal-app/src/main.rs)
- [apps/chatminal-desktop/Cargo.toml](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml)

## Overview
- Priority: High
- Status: Completed
- Goal: đổi bề mặt product/host từ `chatminal-desktop` sang `chatminal-desktop` nhưng không đập engine internals

## Key Insights
- Product identity và engine identity đang trộn vào nhau
- Rename an toàn nhất là đổi package/bin/launcher/docs/scripts trước
- Engine crates `chatminal-chatminal-*` còn có thể giữ tạm để tránh blast radius quá lớn

## Requirements
- `make window` vẫn chạy
- desktop binary không còn lộ `chatminal-desktop` ở command/package path chính
- test/runtime compatibility không gãy

## Related Code Files
- Modify: `Cargo.toml`
- Modify: `apps/chatminal-desktop/Cargo.toml`
- Modify: `apps/chatminal-app/src/main.rs`
- Modify: `apps/chatminal-app/src/terminal_desktop_launcher.rs`
- Modify: `README.md`
- Modify: `Makefile`
- Modify: docs/smoke scripts liên quan launcher desktop

## Implementation Steps
1. Đổi package/bin name của desktop host
2. Đổi launcher command surface từ `window-desktop` sang `window-desktop` và giữ `window` là alias chính
3. Đồng bộ README/Makefile/scripts
4. Build/test lại desktop path

## Success Criteria
- package/binary surface của desktop host dùng naming Chatminal
- `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`
- `cargo check --workspace`

## Risk Assessment
- rename package/bin có thể làm gãy launcher tests và smoke scripts
- nếu không dọn tiếp CLI/env naming thì `chatminal` vẫn còn lộ ở bề mặt compatibility

## Next Steps
- Phase 02: tiếp tục bóc nốt CLI/env surface (`dashboard`, `attach`, `bench-rtt`, `desktop|legacy`) và gỡ product links/updater upstream
