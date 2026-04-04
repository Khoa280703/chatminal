# Phase 05: Control Plane Foundation

## Goal
Tách ownership/control-plane khỏi `Mux` đến mức đủ để wiring explicit RuntimeHost ở phase sau không phải chỉnh lại core.

## Lane
### Lane 05A: Control Plane And Singleton Extraction Foundation
- Ownership:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/client.rs`
  - `crates/chatminal-host-runtime/src/window.rs`
  - `crates/chatminal-runtime/src/runtime_host.rs`
- Scope:
  - thu hẹp tiếp `MuxHandle` / `Mux` responsibilities
  - chuẩn hóa control-plane APIs để caller sau này nhận explicit host/runtime owner
  - chuẩn bị ground cho bỏ `static MUX`

## Parallel Safety
- Không tách lane ở phase này vì đây là cụm foundation đụng cùng lõi.

## Gate
- `cargo check --workspace`
- grep/smoke audit cho caller ngoài host-runtime
