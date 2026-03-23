---
title: "Desktop Sidebar Resize"
description: "Short plan to add draggable sidebar resize in Chatminal desktop without touching terminal core."
status: pending
priority: P2
effort: 2.5h
branch: main
tags: [desktop, shell, ui, sidebar, resize]
created: 2026-03-23
---

# Desktop Sidebar Resize

## Scope
- MVP chỉ xử lý desktop shell/UI layer.
- Không đổi `crates/chatminal-terminal-core`.
- Không kéo session engine/runtime vào scope trừ khi thật sự cần persistence cho width; mặc định phase này để width sống trong app state.

## Main Files
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`
  - bỏ width cố định; thêm width state + clamp helper (`min/max/default`) cho sidebar.
- `apps/chatminal-desktop/src/termwindow/mod.rs`
  - đổi `chatminal_sidebar_width*`, `shell_bounds()`, `terminal_grid_origin()` sang đọc width động.
  - thêm `UIItemType` cho resize handle nếu cần.
- `apps/chatminal-desktop/src/termwindow/resize.rs`
  - bảo đảm left padding/content width tính theo sidebar width động khi window resize.
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
  - render divider/drag handle mỏng ở mép phải sidebar, giữ hit area dễ kéo.
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
  - thêm hover cursor + press/move/release cho resize handle; invalidate frame khi kéo.

## Order
1. Cố định data model: thêm sidebar width state + clamp rules trong `chatminal_sidebar/mod.rs`.
2. Nối width state vào geometry shell trong `termwindow/mod.rs` và `termwindow/resize.rs`.
3. Render handle/divider trong `termwindow/render/chatminal_sidebar.rs`.
4. Bắt mouse drag trong `desktop_termwindow_mouseevent.rs`, cập nhật width realtime.
5. Smoke check desktop layout: sidebar, content pane, footer, session bar, split drag không lệch hitbox.

## Notes
- Dùng resize handle riêng, không bắt drag cả nền sidebar; tránh conflict với click session/profile.
- Clamp nên theo px + ratio cửa sổ, giữ logic gần với giới hạn hiện tại (`min`, `max_window_ratio`).
- Persistence width là optional follow-up. Nếu cần, ưu tiên tận dụng string state store hiện có; không thuộc MVP này.

## Validation
- `cargo check -p chatminal-desktop`
- Mở app, kéo sidebar qua lại, verify:
  - tree/sidebar render đúng
  - terminal content co giãn đúng
  - tab/session bar và footer không chồng
  - workspace split drag vẫn hoạt động

## Risks
- Dễ lệch giữa render bounds và hit-test nếu update một bên quên bên kia.
- Resize realtime có thể làm pane background/content_rect lệch 1 cell nếu width động chưa đi hết qua `shell_bounds`.
- Khởi tạo window mới có thể vẫn dùng default width; chấp nhận cho MVP, miễn sau khi drag state trong session hiện tại ổn.

## Unresolved Questions
- Có cần persist sidebar width qua restart ngay ở phase đầu không, hay chấp nhận session-only trước?
