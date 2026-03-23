# Phase 07 - Verification Docs Sync And Rollout Gates

## Context Links
- `docs/development-roadmap.md`
- `docs/project-changelog.md`
- `docs/system-architecture.md`
- `docs/codebase-summary.md`
- `apps/chatminal-desktop/Cargo.toml`
- `README.md`

## Overview
- Priority: P1
- Current status: pending
- Brief: gom quality gates cuối để bảo đảm toàn bộ polish chỉ nằm ở shell layer, build sạch, docs đúng, và có đường rollback/scope cut nếu một surface kéo theo core risk.

## Key Insights
- Đây là plan shell-only, nên verification quan trọng nhất là “không lạc scope”.
- Repo đã có các docs kiến trúc/tóm tắt tương đối tốt; chỉ cần sync đúng phần shell ownership nếu implementation đổi nhiều.
- Cần explicit grep/build gates vì visual polish dễ bị xem nhẹ và merge thiếu guardrail.

## Mục tiêu
- Verify compile, smoke interaction, grep no-touch, docs sync.
- Chốt checklist release nội bộ cho shell-only batch.
- Bảo đảm roadmap không vô tình trở thành core refactor trá hình.

## Phạm vi
- `cargo check -p chatminal-desktop`
- Manual smoke: sidebar, session bar, footer, overlay, split layout, wheel scroll
- Docs update nếu ownership/chrome vocabulary đổi thực sự

## Files Likely Touched
- `docs/development-roadmap.md`
- `docs/project-changelog.md`
- `docs/system-architecture.md`
- `docs/codebase-summary.md`
- `README.md`
- `apps/chatminal-desktop/src/termwindow/mod.rs`

## Requirements
- Có checklist smoke theo từng surface.
- Có grep gate no-touch cho terminal core và runtime core.
- Nếu cần docs update, chỉ mô tả shell ownership mới; không viết sai rằng core đã đổi.

## Architecture
- Verification tập trung ở desktop app target.
- Docs chỉ phản ánh shell/UI ownership và safe seams.
- Nếu một bug fix đòi chạm core, phase này phải fail gate và mở plan khác.

## Related Code Files
- Modify: `docs/development-roadmap.md`
- Modify: `docs/project-changelog.md`
- Modify: `docs/system-architecture.md`
- Modify: `docs/codebase-summary.md`
- Modify: `README.md` nếu cần note UI shell polish
- Create: none
- Delete: none

## Implementation Steps
1. Chạy `cargo check -p chatminal-desktop`.
2. Chạy grep no-touch trên `crates/chatminal-terminal-core` và runtime execution paths.
3. Smoke test sidebar/session bar/footer/overlay/layout/motion.
4. Sync docs ownership nếu implementation tạo vocabulary hoặc shell contract mới.
5. Ghi rollback note: cắt motion trước, rồi overlay polish, rồi footer/session bar nếu cần giảm scope.

## Todo List
- [ ] Desktop target compile sạch
- [ ] Grep no-touch pass
- [ ] Smoke checklist pass
- [ ] Docs sync đúng phần shell
- [ ] Có rollback order

## Success Criteria
- Shell polish merge được mà không kéo theo core diff.
- Team có checklist rõ để review và rollback từng surface.
- Docs phản ánh đúng “UI shell improved, terminal core unchanged”.

## Risk Assessment
- Risk: visual batch lớn nhưng thiếu smoke checklist, bug lọt ở mouse/overlay/focus.
- Mitigation: review theo surface và rollback theo phase.

## Security Considerations
- Verify confirm/destructive overlays còn nguyên.
- Không log thêm nội dung terminal, clipboard, session data ngoài mức hiện có.

## Không Đụng Terminal Core
- No-touch tuyệt đối: `crates/chatminal-terminal-core/**`
- No-touch tuyệt đối: `crates/chatminal-runtime/src/state/**`
- No-touch tuyệt đối: `apps/chatminal-desktop/src/desktop_host_runtime/session_engine/**`
- Nếu verification phát hiện chỉ có cách sửa core mới qua gate, stop và tách plan khác

## Next Steps
- Sau phase này mới cân nhắc implementation batch theo thứ tự 02 -> 03/04/05 -> 06 -> 07.
