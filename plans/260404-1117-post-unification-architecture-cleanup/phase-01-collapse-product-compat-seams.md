---
phase: 01
status: pending
priority: high
effort: medium
risk: medium
---

# Phase 01: Collapse Product Compat Seams

## Overview
Cắt compatibility seam còn nằm trên active desktop product path, chủ yếu ở `apps/chatminal-desktop/src/desktop_host_runtime/*` và `crates/chatminal-host-runtime/*`.

## Why This Phase Exists
- `legacy_*` wrappers trong desktop host adapter vẫn dày.
- `mux_default()` / Mux-backed defaults vẫn còn là owner mặc định ở vài compat boundary.
- Đây là vùng gây duplicate kiến trúc rõ nhất sau unification.

## Scope
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- `crates/chatminal-host-runtime/src/lib.rs`
- `crates/chatminal-host-runtime/src/spawn_target.rs`
- `crates/chatminal-host-runtime/src/pty_io.rs`
- `crates/chatminal-host-runtime/src/localpane_hooks.rs`

## Requirements
- Active product path phải đi qua first-party/runtime-native helpers trước, không widen ngược về compat facade.
- Compat path nếu còn giữ phải bị cô lập rõ: test-only, migration-only, hoặc explicit compat module.

## Implementation Steps
1. Inventory toàn bộ `legacy_*`, `mux_default()`, `Mux` fallback còn chạy trên startup/spawn/focus/render product flow.
2. Di chuyển phần còn active sang `DesktopSessionHost` / `HostRuntimeRoot` / helper typed mới, không qua facade legacy.
3. Đổi default owner ở spawn/PTY/localpane sang host-native path; `mux_default()` chỉ còn explicit compat/test seam.
4. Xóa wrapper one-hop không còn giúp gì ngoài chuyển tiếp sang legacy layer.
5. Chốt lại test matrix cho startup, spawn, focus, split, shutdown.

## Done Criteria
- `apps/chatminal-desktop/src/desktop_host_runtime/*` không còn wrapper `legacy_*` trong active product flow; nếu còn chỉ ở test/explicit compat seam.
- `mux_default()` không còn được dùng làm default product owner.
- `chatminal-host-runtime` product init/shutdown/spawn path đi qua root/host-native helpers nhất quán.

## Risk / Tradeoff
- Risk: cắt quá tay có thể làm gãy overlay/test paths vốn vẫn dùng compat assumptions.
- Tradeoff: giữ lại một compat module hẹp cho tests còn tốt hơn xóa sạch rồi reintroduce lại tạm bợ.
