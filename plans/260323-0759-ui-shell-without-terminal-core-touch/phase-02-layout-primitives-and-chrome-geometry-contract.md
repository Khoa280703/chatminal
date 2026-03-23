# Phase 02 - Layout Primitives And Chrome Geometry Contract

## Context Links
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/chatminal_layout/workspace_store.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_types.rs`

## Overview
- Priority: P1
- Status: pending
- Brief: làm sạch primitive layout/chrome để sidebar, footer, overlay, split geometry dùng chung contract ổn định

## Objective
- Tách geometry contract của sidebar/header/footer/content/split ra khỏi logic vẽ rải rác.
- Giảm duplicate tính toán padding, bounds, chrome heights.

## Scope
- Chuẩn hóa `padding_left_top`, footer height, chrome height, content viewport.
- Làm rõ session-layout bounds vs overlay bounds vs sidebar bounds.
- Giữ nguyên layout persistence/state model hiện có; chỉ thay adapter và geometry helpers.

## Files Likely Touched
- Modify: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/resize.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
- Create: app-layer helper module only if needed under `apps/chatminal-desktop/src/`
- Delete: none

## Explicit Boundary
- Khong dung terminal core: không đổi terminal size semantics bên trong core, không đổi `WorkspaceLayoutState` ownership, không sửa runtime persistence format.

## Key Insights
- `desktop_termwindow_render_mod.rs` đang là nút tính chrome/padding chính.
- `desktop_termwindow_layout_render.rs` đã là seam tốt cho workspace split/layout render, nên nên kéo primitive về đây hoặc helper kế bên thay vì đụng runtime.

## Requirements
- Functional: sidebar/footer/overlay/split đều lấy bounds từ contract thống nhất.
- Non-functional: không tăng coupling với runtime/store; không duplicate px math ở nhiều file.

## Architecture
- Introduce internal shell geometry model: `chrome_bounds`, `content_bounds`, `sidebar_bounds`, `footer_bounds`, `overlay_scope_bounds`.
- Read-only inputs: window size, dpi, session bar visibility, sidebar enabled.
- Outputs: computed rectangles used by render + hit-test + resize.

## Implementation Steps
1. Inventory các hàm hiện tính kích thước/padding/chrome.
2. Gộp thành một lớp helper layout primitive dùng chung cho render và mouse.
3. Route split/layout/sidebar/footer render qua helper mới.
4. Verify session bar top/bottom modes và sidebar-enabled/disabled đều không lệch bounds.

## Todo List
- [ ] Extract geometry sources
- [ ] Define shared shell bounds contract
- [ ] Rewire render path to shared bounds
- [ ] Rewire hit-test path to same bounds

## Success Criteria
- Một nguồn tính bounds dùng chung cho render, hit-test, resize.
- Không còn magic number rải rác cho chrome/footer/sidebar offsets ở path chính.
- Session bar top/bottom, sidebar on/off, multi-split layout đều vẽ đúng.

## Risk Assessment
- Risk: regress offset click/hit-test hoặc split drag bounds.
- Mitigation: add manual matrix và dùng cùng helper cho mouse/render.

## Security Considerations
- Không được route input sang sai render target vì bounds mapping mới.
- Không đưa metadata runtime mới vào primitive layer.

## Next Steps
- Sang Phase 03 để rebuild sidebar tree/scroll trên geometry contract mới.
