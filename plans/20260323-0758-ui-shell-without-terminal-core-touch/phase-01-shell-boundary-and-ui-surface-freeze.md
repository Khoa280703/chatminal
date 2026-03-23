# Phase 01 - Shell Boundary And UI Surface Freeze

## Context Links
- `README.md`
- `docs/codebase-summary.md`
- `docs/system-architecture.md`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`

## Overview
- Priority: P1
- Current status: pending
- Brief: khóa ranh giới implementation để toàn bộ roadmap chỉ đi trong desktop shell/UI chrome, không rò vào terminal core hay execution core.

## Key Insights
- Repo hiện đã tách khá rõ `termwindow/*` là render/input shell và `crates/chatminal-terminal-core` là parser/state core.
- `desktop_termwindow_*`, `tabbar.rs`, `chatminal_sidebar/mod.rs`, `overlay/*` là bề mặt đúng scope.
- Nếu không freeze boundary ngay từ đầu, phase layout/overlay rất dễ kéo theo thay đổi runtime contract.

## Mục tiêu
- Chốt danh sách module được phép sửa và module cấm sửa.
- Chốt vocabulary cho roadmap: `shell UI`, `chrome`, `sidebar tree`, `session bar`, `footer`, `overlay`, `layout primitives`.
- Tạo checklist acceptance “UI shell only”.

## Phạm vi
- Inventory entry points của shell desktop.
- Xác định safe seams giữa UI shell và runtime facade.
- Chốt grep/build gates cho các phase sau.

## Files Likely Touched
- `docs/system-architecture.md`
- `docs/codebase-summary.md`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- `apps/chatminal-desktop/src/chatminal_render/mod.rs`

## Requirements
- Mọi phase sau phải tham chiếu được ít nhất một safe seam cụ thể.
- Không phase nào được phụ thuộc vào sửa parser, PTY, scrollback storage, session engine core.
- Tạo được grep rule để phát hiện diff lạc scope.

## Architecture
- Boundary trên xuống: `TermWindow` coordinator -> shell render/input modules -> runtime facade/query sẵn có.
- Boundary dưới xuống: terminal core/execution core chỉ được xem như provider state/render target, không đổi contract.
- Mọi polish phải đi qua composition ở shell layer, không qua mutation của core.

## Related Code Files
- Modify: `docs/system-architecture.md`
- Modify: `docs/codebase-summary.md`
- Modify: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`
- Create: none
- Delete: none

## Implementation Steps
1. Đọc lại entry files và lập matrix `allowed-touch` vs `no-touch`.
2. Gắn phase gates vào plan/docs nội bộ.
3. Chuẩn hóa naming cho shell slots: sidebar, chrome top, content, footer, overlay plane.
4. Chốt danh sách query/action nào chỉ được consume read-only từ facade.
5. Freeze acceptance checklist để phase sau không drift.

## Todo List
- [ ] Có matrix allowed-touch rõ ràng
- [ ] Có no-touch list rõ cho terminal core
- [ ] Có grep gate lạc scope
- [ ] Có naming chung cho shell primitives

## Success Criteria
- Dev đọc plan biết chính xác được sửa file nào và không được sửa file nào.
- Không còn ambiguity giữa “UI shell polish” và “runtime/core refactor”.
- Mọi phase sau đều có boundary statement nhất quán.

## Risk Assessment
- Risk: boundary viết quá rộng làm phase sau vô thức chạm runtime.
- Mitigation: freeze bằng file/module list cụ thể, không dùng mô tả mơ hồ kiểu “backend”.

## Security Considerations
- Không đụng auth/transport.
- Tránh thêm đường mutation mới từ UI sang runtime ngoài các action đang có.

## Không Đụng Terminal Core
- No-touch tuyệt đối: `crates/chatminal-terminal-core/**`
- No-touch tuyệt đối: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`
- No-touch tuyệt đối: `crates/chatminal-runtime/src/state/**`
- Không đổi terminal parser, PTY lifecycle, session execution, scrollback persistence contract

## Next Steps
- Phase 02 dùng boundary đã khóa để chuẩn hóa layout primitives và chrome slots.
