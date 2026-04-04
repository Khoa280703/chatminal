# Phase 01: Regression And Contract Lock

## Goal
Đóng 2 review finding đang mở để phần còn lại không build trên contract sai.

## Lanes
### Lane 01A: Startup Env Regression
- Ownership:
  - `apps/chatminal-desktop/src/main.rs`
  - `crates/chatminal-host-runtime/src/client.rs`
- Scope:
  - khôi phục hoặc thay thế propagation của `config.default_ssh_auth_sock`
  - thêm verify cho startup env snapshot
- Output:
  - không còn risk regression `SSH_AUTH_SOCK`

### Lane 01B: Lua Active Session Contract
- Ownership:
  - `crates/chatminal-lua-bridge/src/window.rs`
  - test files trong `crates/chatminal-lua-bridge/src/*` nếu cần
- Scope:
  - thống nhất contract `active_session()` với `active_session_id()`
  - empty/non-chatminal root window phải có behavior rõ và backward-compatible
- Output:
  - không còn contract mismatch Lua

## Parallel Safety
- Hai lane không đụng file nhau.

## Gate
- `cargo check -p chatminal-lua-bridge -p chatminal-desktop`
- test mục tiêu cho 2 regression
