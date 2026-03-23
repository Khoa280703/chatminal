---
title: "Fix desktop sidebar profile/session scrolling"
description: "Add sidebar scroll state, wheel handling, and clipped rendering for long profile/session lists."
status: pending
priority: P2
effort: 1.5h
branch: main
tags: [bugfix, desktop, sidebar, scroll, rust]
created: 2026-03-19
---

# Mini Plan

## Scope
- Bug: sidebar profile/session của Chatminal desktop không scroll được khi item vượt chiều cao hiển thị.
- Code paths chính: `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`, `apps/chatminal-desktop/src/termwindow/mod.rs`, `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`, `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`.

## Steps
1. Thêm sidebar scroll state tối thiểu trong `TermWindow` hoặc `ChatminalSidebar`: lưu `scroll_offset_px`, reset/clamp khi snapshot đổi, profile expand/collapse đổi, hoặc window resize làm giảm chiều cao khả dụng.
2. Route `WMEK::VertWheel` cho `ChatminalSidebarBackground` và item con trong `desktop_termwindow_mouseevent.rs`: chỉ scroll khi pointer nằm trong sidebar, đổi offset theo delta, clamp về `[0, max_scroll]`, rồi `context.invalidate()`.
3. Tách phần tính layout sidebar trong `termwindow/render/chatminal_sidebar.rs`: tính `content_height`, `viewport_height`, `max_scroll`, apply `y` offset âm cho body list và clip render/UI hitbox theo viewport để item ngoài vùng nhìn không ăn click.
4. Verify bằng case nhiều profile/session + nhiều profile expand: wheel up/down mượt, item cuối truy cập được, không bleed xuống footer, click/hit-test map đúng item sau khi scroll, offset tự hồi về hợp lệ khi số item giảm.

## Done
- Sidebar scroll được bằng mouse wheel trên desktop.
- Không có overdraw/hitbox sai ngoài vùng viewport.
- Offset luôn hợp lệ sau refresh snapshot, expand/collapse, resize.
