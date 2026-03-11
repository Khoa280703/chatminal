# Phase 02 - Desktop Session Surface Facade

## Context Links
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mouseevent.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/tabbar.rs
- /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/render/fancy_tab_bar.rs

## Overview
- Priority: P0
- Status: completed
- Brief: gom mọi logic session-to-surface lookup/activate/close/spawn vào một facade/module riêng trong desktop app

## Key Insights
- Hiện desktop session UX vẫn phải map ngược `tab -> session` trong `termwindow/mod.rs`
- Chưa cần crate mới ngay; cần first cut để giảm coupling trước
- Đây là phase đệm bắt buộc để Phase 03 tách crate không đập nát UI shell

## Requirements
- Functional: session bar/sidebar/switch/close/create phải gọi facade thay vì mux trực tiếp
- Non-functional: không đổi terminal behavior, không đổi visual behavior ngoài bug fix đang làm

## Architecture
- Create module tạm: `apps/chatminal-desktop/src/chatminal_session_surface.rs`
- Module này expose:
  - `active_session_surface_id(...)`
  - `focus_session_surface(...)`
  - `spawn_session_surface(...)`
  - `close_session_surface(...)`
  - `collect_session_surface_snapshots(...)`
- `termwindow` chỉ gọi module này cho session-centric flows
- Trong phase này, module facade là nơi duy nhất ở app layer được phép chạm `mux`, `TabId`, `PaneId`
- Mọi session-centric UI flow phải đi qua facade:
  - session bar
  - sidebar session list
  - session create/activate/close
  - session snapshot mapping cho top bar

## Related Code Files
- Create: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/chatminal_session_surface.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mod.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/termwindow/mouseevent.rs
- Modify: /Users/khoa2807/development/2026/chatminal/apps/chatminal-desktop/src/main.rs

## Implementation Steps
1. Tách helper `pane_chatminal_session_id`, `tab_chatminal_session_id`, focus/close/find logic khỏi `termwindow/mod.rs`
2. Chuyển session bar/sidebar/top bar actions sang facade mới
3. Chuyển session snapshot mapping của top bar sang facade
4. Giữ `activate_tab` và tab-centric commands cho non-session paths, chưa đụng ở phase này
5. Thêm grep gate để mọi call site session-centric ngoài facade không còn đụng `mux`/`TabId`/`PaneId`

## Todo List
- [ ] Tạo desktop facade module
- [ ] Xóa helper session-surface lookup rải trong `termwindow/mod.rs`
- [ ] Đảm bảo close/switch/create session không gọi mux trực tiếp từ UI shell
- [ ] Đảm bảo session-centric flow không còn giữ `tab_idx`/`TabId`/`PaneId` làm public identity ngoài facade

## Success Criteria
- Session-centric UX path ngoài facade không còn direct `mux::*`, `Mux::*`, `TabId`, `PaneId`
- Session UX path không truyền `tab_idx` làm identity chính
- `termwindow/mod.rs` và `mouseevent.rs` không còn helper tab/pane lookup riêng cho flow session
- Build desktop pass

## Risk Assessment
- Risk: regress sync active state giữa session bar và live surface
- Mitigation: giữ behavior cũ, chỉ đổi ownership logic

## Security Considerations
- Không thay persistence hay external IPC
- Không đổi process spawning contract

## Validation Gates
- `cargo check -p chatminal-desktop`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml`
- `rg -n "mux::|Mux::|TabId|PaneId" apps/chatminal-desktop/src --glob '!chatminal_session_surface.rs'`
  - expected: zero results cho session-centric desktop path ngoài facade

## Next Steps
- Sang Phase 03 để đưa facade thành crate live graph mới
