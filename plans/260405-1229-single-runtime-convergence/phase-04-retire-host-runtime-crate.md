---
phase: 04
status: done
priority: critical
effort: high
risk: high
---

# Phase 04: Retire Host Runtime Crate

## Context Links
- [plan.md](./plan.md)
- [desktop-host-runtime-blockers.md](./desktop-host-runtime-blockers.md)
- [chatminal-host-runtime/src/lib.rs](/Users/khoa2807/development/2026/chatminal/crates/chatminal-host-runtime/src/lib.rs)
- [apps/chatminal-desktop/Cargo.toml](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/Cargo.toml)
- [crates/chatminal-lua-bridge/Cargo.toml](/Users/khoa2807/development/2026/chatminal/crates/chatminal-lua-bridge/Cargo.toml)

## Overview
- Priority: P0
- Current status: in_progress
- Mục tiêu: xóa hẳn `chatminal-host-runtime` khỏi active product architecture để repo chỉ còn một runtime crate thực sự là `chatminal-runtime`.

## Key Insights
- Nếu `chatminal-host-runtime` còn sống trong active dependency graph, người đọc và code vẫn còn hai mental models runtime song song.
- Với product reality hiện tại, giữ lại crate này như “utility mỏng” vẫn là debt kiến trúc không cần thiết.
- End-state của phase này phải là: desktop, lua-bridge, và codec không còn depend crate này nữa.
- Slice hiện tại đã cắt `chatminal-codec` khỏi `chatminal-host-runtime` thật sự; `cargo tree -p chatminal-codec` chỉ còn `chatminal-runtime` trên graph riêng của codec.

## Requirements
- `apps/chatminal-desktop/Cargo.toml` bỏ dependency `host_runtime`.
- `crates/chatminal-lua-bridge/Cargo.toml` bỏ dependency `host_runtime`.
- `crates/chatminal-codec/Cargo.toml` bỏ dependency `host_runtime`.
- Toàn bộ phần code còn cần từ `chatminal-host-runtime` phải được:
  - migrate sang `chatminal-runtime`, hoặc
  - migrate sang desktop UI/render modules phù hợp, hoặc
  - xóa nếu không còn giá trị product path.
- Crate `chatminal-host-runtime` không còn là active runtime crate trong workspace end-state.
- Không được kết thúc phase bằng cách đổi tên crate, archive trong active workspace, hay giữ lại như deprecated utility dependency.

## Architecture
- `chatminal-runtime` giữ execution/runtime ownership duy nhất.
- `chatminal-desktop` giữ UI/presentation modules duy nhất.
- Không còn crate thứ ba đứng giữa hay song song để giữ `Window` / `Tab` / `Pane` runtime model.
- `chatminal-host-runtime` bị xóa khỏi active architecture thật sự:
  - không còn Cargo dependency product-path
  - không còn public API active-path
  - không còn được dùng như stepping stone hay compatibility owner

## Related Code Files
- Delete/Move: `crates/chatminal-host-runtime/src/*`
- Modify: workspace `Cargo.toml` nếu cần
- Modify: `apps/chatminal-desktop/Cargo.toml`
- Modify: `crates/chatminal-lua-bridge/Cargo.toml`
- Modify: `crates/chatminal-codec/Cargo.toml`
- Modify: `crates/chatminal-codec/src/lib.rs`
- Modify: mọi caller/import path đang dùng `host_runtime`

## Implementation Steps
1. Inventory toàn bộ caller của `host_runtime` còn lại sau phase 03, gồm desktop + lua-bridge + codec.
2. Port phần còn giá trị sang `chatminal-runtime` hoặc desktop modules phù hợp.
3. Xóa dependency `host_runtime` khỏi desktop, lua-bridge, và codec.
4. Xóa crate khỏi active workspace path.
5. Chạy grep + compile/test để chứng minh graph mới chỉ còn một runtime crate.

## Todo List
- [x] Liệt kê toàn bộ caller còn lại của `host_runtime`
- [x] Port hoặc xóa từng caller
- [x] Bỏ dependency khỏi desktop
- [x] Bỏ dependency khỏi lua-bridge
- [x] Bỏ dependency khỏi codec
- [x] Xóa crate khỏi active workspace path
- [x] Verify graph mới bằng grep + cargo metadata/check/tests

## Phase 04 Progress Notes
- `crates/chatminal-codec/Cargo.toml` đã bỏ `host_runtime.workspace = true` và chuyển sang `chatminal-runtime`.
- `crates/chatminal-codec/src/lib.rs` đã rewire `ClientId`, `ClientInfo`, `RenderableDimensions`, `StableCursorPosition`, `PaneNode`, `SerdeUrl`, `SplitRequest`, `Pattern`, và `SearchResult` qua `protocol_types` backed by `chatminal-runtime`.
- `crates/chatminal-lua-bridge/Cargo.toml` không còn dependency `host_runtime`; source path `crates/chatminal-lua-bridge/src/*` đã đi qua local backend contract + runtime DTO thay vì `host_runtime::*`.
- `apps/chatminal-desktop/src/desktop_host_runtime/lua_bridge_backend.rs` đã bị ép qua `desktop_host_runtime` facade; file này không còn `host_runtime::*` trực tiếp.
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`, [`session_host.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs), và [`spawn_target.rs`](/Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs) đã co spawn boundary xuống trait desktop-local:
  - `HostSpawnTargetHandle` không còn lộ trực tiếp `host_runtime::spawn_target::SpawnTarget`
  - legacy adapter quay lại host_runtime chỉ còn là one-way bridge trong desktop boundary
  - spawn substrate vẫn còn ở host-runtime, nhưng vocabulary công khai của handle đã sạch hơn
- Verify sau lát cắt `lua-bridge`:
  - `cargo check -p chatminal-lua-bridge -p chatminal-host-runtime -p chatminal-desktop -p chatminal-runtime -p chatminal-codec`
  - `cargo test -p chatminal-lua-bridge --lib -- --test-threads=1`
  - `cargo test -p chatminal-codec --lib -- --test-threads=1`
- Verify incremental mới nhất:
  - `cargo check -p chatminal-desktop -p chatminal-lua-bridge -p chatminal-runtime`
  - `cargo test -p chatminal-lua-bridge --lib -- --test-threads=1`
- `crates/chatminal-runtime/src/pane.rs` và `crates/chatminal-runtime/src/renderable.rs` đã trở thành canonical source cho pane/render helpers; `crates/chatminal-host-runtime/src/pane.rs` và `crates/chatminal-host-runtime/src/renderable.rs` chỉ còn re-export mỏng.
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` đã chuyển `overlay_shell::*`, `HostTerminal`, và các helper `impl_get_logical_lines_via_get_lines` / `terminal_get_*` sang `chatminal-runtime::{pane,renderable}`.
- `HostRuntimeNotification` đã được re-home sang `crates/chatminal-runtime/src/host_notification.rs`; desktop hiện import vocabulary này từ `chatminal-runtime` thay vì trực tiếp từ `host_runtime`.
- Verify sau pane/renderable cutover:
  - `cargo check -p chatminal-runtime -p chatminal-host-runtime -p chatminal-desktop`
  - `cargo test -p chatminal-runtime --lib -- --test-threads=1`
- Verify sau host-notification cutover:
  - `cargo check -p chatminal-runtime -p chatminal-host-runtime -p chatminal-desktop`
- Grep `host_runtime::pane::|host_runtime::renderable::` trong `apps/`, `crates/chatminal-runtime/`, `crates/chatminal-host-runtime/` hiện đã sạch.
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` đã bỏ thêm một lớp dual-path product fallback: các thao tác pane/window/workspace/spawn chính giờ ưu tiên tuyệt đối `DesktopSessionHost`, không còn fallback chạy thẳng xuống `host_runtime` trong active desktop flow.
- `cargo check -p chatminal-runtime -p chatminal-codec -p chatminal-host-runtime -p chatminal-lua-bridge` pass sau cutover này.
- `cargo tree -p chatminal-codec` không còn `chatminal-host-runtime`.
- `cargo tree -p chatminal-desktop -e normal` cho thấy blocker active graph còn lại nằm ở dependency trực tiếp `chatminal-desktop -> chatminal-host-runtime`.

### Desktop blocker map hiện tại
- `spawn/PTY lifecycle`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs`
  - blocker API: `spawn_tab`, `split_pane`, `register_pane_with_output_callback`, `initialize_host_runtime_with_config`, `primary_spawn_target`, `LocalSpawnTarget`, `LocalSpawnHooks`
- `pane/render surface`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
  - nhiều caller UI/overlay đang đi qua `overlay_shell::*`
  - blocker API: `host_runtime::pane::*`, `host_runtime::renderable::*`
- `window/workspace registry`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - blocker API: `window::{Window, WindowId}`, `with_root_window(_mut)`, `root_window_workspace_name`, `active_workspace_name`, `active_identity`, `active_workspace_for_client`, `iter_workspaces`
- `notifications/control plane`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - blocker API: `HostRuntimeNotification`, `Activity`, notification subscribe bridge, focus/input bookkeeping helpers
- `overlay terminal`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/overlay/*`
  - blocker API: `termwiztermtab::{allocate, TermWizTerminal}`

### Largest blocker
- Nhóm khó nhất để bỏ dependency Cargo khỏi desktop là `pane/render surface + overlay terminal`, vì đây là vocabulary cross-cutting đang bị re-export vào nhiều termwindow/overlay modules, không còn là một seam cô lập chỉ trong `session_host.rs`.

## Success Criteria
- `chatminal-host-runtime` không còn là dependency của desktop product path.
- `chatminal-host-runtime` không còn là dependency của lua-bridge.
- `chatminal-host-runtime` không còn là dependency của codec.
- Trong active architecture chỉ còn một runtime crate: `chatminal-runtime`.
- Không còn public/runtime vocabulary `HostRuntimeHandle`, `Window`, `Tab`, `Pane` đi qua cross-crate active path.
- `cargo metadata` không còn node package `chatminal-host-runtime` trong active workspace dependency path của app/runtime consumers.

## Risk Assessment
- Đây là phase blast radius lớn vì động tới Cargo graph thật.
- Nếu phase 02/03 chưa clean owner boundary, phase này sẽ bị block hoặc buộc tạo shim mới, điều đó là không chấp nhận.

## Security Considerations
- Mọi path input/clipboard/download/cleanup migrate từ crate bị retire phải giữ nguyên guard semantics.
- Không để create new direct access path bypass runtime ownership checks.

## Next Steps
- Phase 05 chỉ bắt đầu khi `cargo metadata` và grep dependency graph đều chứng minh chỉ còn một runtime crate sống trong active product path.
