# Phase 01 - Target Architecture And Boundary Freeze

## Context Links
- README: /Users/khoa2807/development/2026/chatminal/README.md
- Desktop termwindow: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- Runtime state: /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state.rs

## Overview
- Priority: P0
- Status: completed
- Brief: khóa target architecture, naming, ownership và migration boundary trước khi code lớn

## Key Insights
- Complexity của mux không biến mất; chỉ được internalize vào session runtime mới
- `window/tab/pane` không được dùng làm business language sau phase này
- `profile -> session -> session surface/layout` là public model mới

## Requirements
- Functional: define module map, public types, adapter boundary
- Non-functional: không làm vỡ build hiện tại, không đổi behavior runtime ở phase này

## Architecture
- `crates/chatminal-runtime`: business state, profile/session metadata, persistence bridge
- `crates/chatminal-session-runtime`: live graph, session surface, layout tree, event bus, engine adapter
- `apps/chatminal-desktop`: consume session graph snapshots, không gọi mux tab trực tiếp ở session UX paths
- Phase rule:
  - hết `Phase 02`: `mux` chỉ được phép xuất hiện trong desktop facade tạm `chatminal_session_surface.rs`
  - từ `Phase 03`: `mux` chỉ được phép xuất hiện trong `engine_surface_adapter.rs`
- Stable identity contract phải được chốt ở phase này:
  - `session_id`: identity business/persistence
  - `surface_id`: identity live surface 1-1 với session instance đang attach
  - `leaf_id`: identity render/input target ổn định trong surface
  - `layout_node_id`: identity topology node cho split tree; không dùng index làm public identity

## Related Code Files
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- Modify: /Users/khoa2807/development/2026/chatminal/crates/chatminal-runtime/src/state.rs
- Create: /Users/khoa2807/development/2026/chatminal/crates/chatminal-session-runtime/src/lib.rs
- Delete later: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/bin/chatminal-mux/main.rs

## Implementation Steps
1. Chốt public naming: `SessionSurface`, `SessionLayoutNode`, `SessionWorkspaceHost`, `SessionEventBus`
2. Chốt file ownership giữa runtime/store/desktop
3. Chốt compatibility rule theo phase: desktop facade tạm ở Phase 02, adapter private từ Phase 03
4. Chốt delete list cuối plan để tránh nợ kiến trúc mới
5. Chốt ownership matrix cho `active_session_id`, `surface_id`, `leaf_id`, `layout_node_id`

## Todo List
- [ ] Viết architecture boundary note trong code comment/module docs của crate mới
- [ ] Chốt public types ban đầu
- [ ] Chốt mapping từ tab/pane cũ sang surface/layout mới
- [ ] Chốt stable id contract và source-of-truth matrix

## Success Criteria
- Có target module map rõ ràng
- Có rule cứng cho nơi được phép gọi mux
- Có source-of-truth matrix rõ cho active session, active surface, active leaf
- Các phase sau có scope không chồng lấn

## Risk Assessment
- Risk: naming lẫn giữa session business và session live surface
- Mitigation: tách `session metadata` và `session surface` thành 2 type khác nhau ngay từ đầu

## Security Considerations
- Không mở thêm IPC/public API mới ở phase này
- Không đổi auth/access semantics của runtime/store

## Validation Gates
- `cargo check --workspace`
- `rg -n "mux::|Mux::|TabId|PaneId" apps/chatminal-desktop/src crates/chatminal-runtime/src`
  - dùng để establish baseline trước migration; chưa áp zero-result ở phase này

## Next Steps
- Sang Phase 02 để cô lập desktop shell khỏi tab-centric call sites
