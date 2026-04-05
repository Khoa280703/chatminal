# Desktop Host Runtime Blockers

## Purpose
Map phần còn lại khiến `apps/chatminal-desktop` vẫn kéo `chatminal-host-runtime` vào active graph sau khi `chatminal-codec` và `chatminal-lua-bridge` đã cắt xong.

## Blocker Groups

### 1. Overlay and terminal surface vocabulary
- Files:
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Current dependency:
  - `host_runtime::pane::*`
  - `host_runtime::renderable::*`
  - `host_runtime::termwiztermtab::*`
- Why it blocks:
  - Đây là trait/type surface mà rất nhiều UI modules đang consume qua `overlay_shell::*`.
  - Nếu chưa localize lớp này, desktop vẫn phải depend crate ngoài chỉ để lấy terminal pane vocabulary.

### 2. Root window / runtime-entry control plane
- Files:
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Current dependency:
  - `host_runtime::window::*`
  - `host_runtime::root_*`
  - `host_runtime::runtime_entry_*`
  - `host_runtime::focus_*`
  - `host_runtime::resolve_*`
- Why it blocks:
  - Đây là phần giữ `Window/Tab/Pane` như owner vocabulary trong desktop product path.
  - Đây cũng là nơi gắn workspace/title/focus/active-render-target vào host runtime global root.

### 3. Spawn/local shell infrastructure
- Files:
  - `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/lua_bridge_backend.rs`
- Current dependency:
  - `host_runtime::spawn_target::{LocalSpawnHooks, LocalSpawnTarget, SplitSource, SpawnTarget}`
  - `host_runtime::spawn_tab`
  - `host_runtime::split_pane`
- Why it blocks:
  - Local shell runner và split spawn path vẫn đi qua implementation của crate cũ.
  - Chưa cắt nhóm này thì desktop vẫn phải kéo crate cũ cho PTY/local shell fallback.

### 4. Notification and activity bridge
- Files:
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Current dependency:
  - `host_runtime::activity::Activity`
  - `host_runtime::HostRuntimeNotification`
  - `host_runtime::HostRuntimeHandle`
- Why it blocks:
  - Desktop notification hub hiện vẫn convert trực tiếp từ notification enum của crate cũ.
  - Activity lifecycle và subscribe bridge vẫn giữ global host runtime handle sống trong desktop.

## Biggest Blocker
`apps/chatminal-desktop/src/desktop_host_runtime/mod.rs` là choke point lớn nhất.

Lý do:
- file này re-export surface types cho toàn UI path
- file này wrap hầu hết `host_runtime::*` helpers public cho desktop
- nếu chưa cắt file này, bỏ `host_runtime` khỏi `apps/chatminal-desktop/Cargo.toml` là chưa thể

## Recommended Cut Order
1. Localize overlay/terminal surface vocabulary khỏi `host_runtime`
2. Localize spawn/local-shell helpers
3. Rewire root window/runtime-entry control plane sang desktop-local modules + `chatminal-runtime`
4. Xóa notification/activity bridge của crate cũ
5. Bỏ dependency `host_runtime` khỏi `apps/chatminal-desktop/Cargo.toml`
