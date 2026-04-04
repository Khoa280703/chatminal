# Phase 06: Explicit RuntimeHost Wiring Parallel

## Goal
Sau khi control-plane foundation ổn, chuyển caller lớn sang explicit runtime/host wiring theo ownership tách biệt.

## Lanes
### Lane 06A: Desktop Explicit Wiring
- Ownership:
  - `apps/chatminal-desktop/src/desktop_host_runtime/*`
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Scope:
  - desktop không còn lệ thuộc singleton shape ở các path đã scope
  - chuẩn bị cho việc bỏ `static MUX`

### Lane 06B: Lua Explicit Wiring
- Ownership:
  - `crates/chatminal-lua-bridge/src/*`
- Scope:
  - Lua bridge nhận host/runtime intent rõ hơn, không còn implicit singleton assumptions ở path đã scope

## Parallel Safety
- 06A và 06B không đụng cùng file.
- Cả hai phụ thuộc Phase 05.

## Gate
- `cargo check -p chatminal-lua-bridge -p chatminal-desktop`
- desktop tests xanh
