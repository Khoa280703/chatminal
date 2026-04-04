# Phase 07: PTY Owner Migration Parallel

## Goal
Đẩy `03G` từ seam preparation sang ownership migration thật.

## Lanes
### Lane 07A: Host Runtime Compat PTY Path
- Ownership:
  - `crates/chatminal-host-runtime/src/pty_io.rs`
  - `crates/chatminal-host-runtime/src/localpane.rs`
  - `crates/chatminal-host-runtime/src/localpane_hooks.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
- Scope:
  - bỏ dần default cleanup/output fallback còn Mux-backed ở compat path
  - đẩy lifecycle owner ra khỏi branch mặc định hiện tại

### Lane 07B: Session Engine Native PTY Path
- Ownership:
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/*`
- Scope:
  - chuyển output/error/exit owner cuối sang session-engine side rõ ràng hơn
  - làm session-native path trở thành owner chuẩn

## Parallel Safety
- 07A và 07B không đụng cùng file.
- Contract hook phải freeze đầu phase.

## Gate
- `cargo check -p chatminal-host-runtime -p chatminal-desktop`
- desktop tests xanh
- manual smoke cho spawn/output/exit
