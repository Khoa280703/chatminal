# Phase 02: Host Runtime API Foundation

## Goal
Chốt lớp API host-runtime đủ hẹp để các consumer cutover song song mà không phải tiếp tục bẻ public contract ở giữa chừng.

## Lane
### Lane 02A: Host Runtime Surface Hardening
- Ownership:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `crates/chatminal-host-runtime/src/pane.rs`
  - `crates/chatminal-host-runtime/src/tab.rs`
  - `crates/chatminal-host-runtime/src/window.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
  - `crates/chatminal-host-runtime/src/client.rs`
- Scope:
  - thay các public edge còn lộ `Arc<Tab>` bằng DTO/helper/capability hẹp hơn
  - thay các public edge còn lộ `PaneId`/`TabId` bằng typed wrappers nơi có thể
  - chốt helper contract cho desktop và lua bridge dùng ở phase kế tiếp
- Output:
  - consumer phase sau chỉ cần migrate callsites, không phải phát minh thêm host contract

## Parallel Safety
- Không tách lane ở phase này vì tất cả đụng cùng cluster file lõi.

## Gate
- `cargo check -p chatminal-host-runtime`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
