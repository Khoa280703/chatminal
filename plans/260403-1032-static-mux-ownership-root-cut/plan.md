---
title: "Static MUX Ownership Root Cut"
description: "Minimal safe Phase 03 cut to stop static host runtime storage from being the real owner."
status: pending
priority: P2
effort: 4h
branch: main
tags: [phase-03, integration-backlog-2, mux, ownership]
created: 2026-04-03
---

# Goal
Tiến `Phase 03 / Integration Backlog 2` bằng nhát cắt nhỏ nhất: `HOST_RUNTIME_ROOT` không còn là owner thật; desktop bootstrap giữ owner thật, còn global access chỉ còn là registry/fallback lookup.

## Minimal Safe Cut Path
1. `crates/chatminal-host-runtime/src/lib.rs`
   - đổi static root từ `Mutex<Option<Arc<HostRuntimeRoot>>>` sang registry không-owning (`Weak`/equivalent).
   - để `MuxHandle` giữ ownership thật của `Arc<HostRuntimeRoot>`.
   - giữ nguyên free-function surface (`try_global_mux()`, `with_root_window(...)`, `spawn_tab(...)`, `split_pane(...)`) để không mở rộng blast radius trong batch này.
2. `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
   - giữ `Arc<MuxHandle>` sống suốt desktop session-host lifecycle.
   - bootstrap/shutdown phải set/drop owner rõ ràng ở đây thay vì dựa vào static root giữ hộ.
3. `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs`
   - bỏ `LocalSpawnHooks::default()` ở desktop seam; gọi explicit `LocalSpawnHooks::mux_default()` để compat Mux owner là quyết định product-boundary, không còn implicit default.

## Real Blockers
- `host_runtime` free helpers và async notification path vẫn assume global discoverability (`try_global_mux()`, `Mux::get()`, `notify_from_any_thread()`), nên không thể xóa lookup global ngay trong batch này; chỉ nên đổi ownership, chưa đổi access model.
- PTY/localpane cleanup vẫn Mux-backed thật ở `crates/chatminal-host-runtime/src/localpane_hooks.rs` và `crates/chatminal-host-runtime/src/pty_io.rs`; nếu đụng semantics ở đây sẽ tràn scope sang `Backlog 3`.
- Desktop hiện bootstrap xong là thả `MuxHandle`; nếu static chuyển sang non-owning mà chưa có holder lâu dài ở `session_host.rs` thì root sẽ rơi ngay sau init.

## Primary File Ownership
- Core owner cut:
  - `crates/chatminal-host-runtime/src/lib.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Seam explicitness:
  - `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs`
- Read-only verify / do not expand unless blocker:
  - `crates/chatminal-host-runtime/src/spawn_target.rs`
  - `crates/chatminal-host-runtime/src/localpane_hooks.rs`
  - `crates/chatminal-host-runtime/src/pty_io.rs`
  - `crates/chatminal-host-runtime/src/tab.rs`

## Done Check
- desktop path vẫn boot/shutdown xanh với owner sống qua whole session-host lifecycle.
- không đổi Lua/runtime public contract trong batch này.
- grep delta mong muốn:
  - static root không còn own `Arc<HostRuntimeRoot>`
  - desktop seam không còn `LocalSpawnHooks::default()` cho local/serial targets

## Unresolved Questions
- Có muốn giữ `MuxHandle` trong `DesktopSessionHost` hay một static desktop-local slot cạnh `HOST_REGISTRY`? Giữ trong host object sạch ownership hơn.
