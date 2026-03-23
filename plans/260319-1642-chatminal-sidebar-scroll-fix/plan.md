---
title: "Fix scroll sidebar profile/session Chatminal desktop"
description: "Mini-plan sửa sidebar profile/session chưa scroll được khi item nhiều"
status: pending
priority: P2
effort: 1h
branch: main
tags: [bugfix, desktop, rust, sidebar, scroll]
created: 2026-03-19
---

# Mini Plan

## Scope
- Bug: sidebar profile/session của desktop không scroll được khi số profile/session vượt chiều cao panel.
- Mục tiêu: thêm scroll state tối thiểu, route `mouse wheel` vào sidebar, render theo viewport có clipping/offset, rồi verify không vỡ hit-test/click.

## Files
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`

## Steps
1. Thêm sidebar scroll state nhỏ gọn trong `TermWindow` hoặc `ChatminalSidebar`: lưu `scroll_offset_px` và helper clamp theo `content_height - viewport_height`, reset/clamp lại khi snapshot/sidebar height đổi.
2. Route `WMEK::VertWheel` cho vùng `ChatminalSidebarBackground` và item con trong `desktop_termwindow_mouseevent.rs`; chỉ consume wheel khi hover sidebar, cập nhật offset theo delta tự nhiên rồi `context.invalidate()`.
3. Tách render sidebar thành `viewport_height` và `content_height`; khi build tree ở `termwindow/render/chatminal_sidebar.rs`, apply `y` offset âm theo `scroll_offset_px` và clip phần render ngoài panel để item dư không tràn/hit sai.
4. Đồng bộ hit area với phần đã render: chỉ giữ UI items nằm trong viewport sau offset/clipping, để click profile/session vẫn đúng dù list đã scroll.
5. Verify bằng case nhiều profile/session, hover + wheel lên/xuống, resize window, expand/collapse profile, đổi active session; chạy tối thiểu `cargo check -p chatminal-desktop`.

## Unresolved Questions
- Offset nên nằm trong `TermWindow` (UI state thuần render/input) hay `ChatminalSidebar` (state sidebar tập trung); ưu tiên `TermWindow` nếu muốn tránh poll/runtime snapshot làm lẫn state UI tạm thời.
