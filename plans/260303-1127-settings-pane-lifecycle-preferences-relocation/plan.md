---
title: "Settings Pane + Lifecycle Pref Relocation"
description: "Thêm settings pane trong layout hiện có và chuyển 2 lifecycle preferences ra khỏi profile menu với thay đổi tối thiểu."
status: pending
priority: P2®
effort: 2h
branch: main
tags: [frontend, svelte, settings, lifecycle]
created: 2026-03-03
---

# Goal
- Có `Settings` pane/page trong cùng layout terminal app.
- Move 2 controls `keep_alive_on_close`, `start_in_tray` từ profile menu sang settings.
- Không đổi logic terminal/explorer hiện có.
- Giữ diff nhỏ, reuse logic hiện tại (`loadLifecyclePreferences`, `setLifecyclePreferences`).

# Plan (4 Steps)
1. Refactor điều hướng pane tối thiểu trong `App.svelte`:
- Mở rộng `activePaneMode` thành `terminal | explorer | settings`.
- Đổi control header sang explicit mode buttons (hoặc 1 nút Settings riêng), giữ hành vi resize chỉ khi quay lại terminal.

2. Thêm `settings-pane` trong main layout:
- Render pane mới cùng cấp với terminal/explorer pane, dùng `pane-active`/`pane-hidden`.
- Tạo section `Lifecycle` chứa 2 checkbox bind vào `lifecyclePreferences`, gọi lại `setLifecyclePreferences(...)` như flow cũ.

3. Dọn profile menu:
- Remove 2 block `profile-pref-row` khỏi profile dropdown.
- Giữ nguyên các action profile (switch/create/rename/delete), không đổi command backend.

4. CSS tối thiểu + regression check:
- Bổ sung class style cho `settings-pane` và controls, giữ visual đồng bộ theme hiện tại.
- Verify thủ công: mode terminal/explorer vẫn hoạt động, resize terminal không lỗi, 2 prefs vẫn persist đúng sau reload.

# Files To Modify
- `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`
- `/home/khoa2807/working-sources/chatminal/frontend/src/styles.css`

# Unresolved Questions
- Nút mở `Settings` nên đặt trong `terminal-header` (nhanh, ít thay đổi) hay thêm entry trong sidebar/footer để UX rõ hơn?
