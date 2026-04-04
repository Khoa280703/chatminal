# Phase 09: Config Propagation Parallel

## Goal
Đẩy `configuration()` singleton ra khỏi các path còn scope được bằng 2 lane độc lập.

## Lanes
### Lane 09A: Desktop Config Propagation
- Ownership:
  - `apps/chatminal-desktop/src/main.rs`
  - `apps/chatminal-desktop/src/frontend.rs`
  - `apps/chatminal-desktop/src/stats.rs`
  - `apps/chatminal-desktop/src/selection.rs`
  - `apps/chatminal-desktop/src/overlay/*`
  - `apps/chatminal-desktop/src/termwindow/*`
- Scope:
  - inject/pass snapshot thay cho singleton reads ở desktop path còn lại

### Lane 09B: Host Runtime Config Propagation
- Ownership:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/pty_io.rs`
  - `crates/chatminal-host-runtime/src/localpane.rs`
  - `crates/chatminal-host-runtime/src/tab.rs`
  - `crates/chatminal-host-runtime/src/window.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
- Scope:
  - inject/snapshot config cho host/runtime path còn lại, nhất là constructor/read-loop edge đã scope

## Parallel Safety
- 09A và 09B không đụng cùng file.
- Cả hai phụ thuộc Phase 08 contract freeze.

## Gate
- `cargo check --workspace`
- desktop tests xanh
- grep `configuration()` giảm tiếp trên paths đã scope
