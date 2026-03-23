# Phase 07 - Verification Regression Gates And Rollout Freeze

## Context Links
- `README.md`
- `docs/code-standards.md`
- `apps/chatminal-desktop/Cargo.toml`
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`

## Overview
- Priority: P1
- Status: pending
- Brief: chốt quality gates cho toàn roadmap để shell polish không kéo theo regress terminal behavior hoặc boundary leak

## Objective
- Xác nhận mọi thay đổi UI shell vẫn giữ terminal core black-box.
- Dựng checklist verify cho layout, hit-test, overlay, motion, session bar, footer.

## Scope
- Build/check gates, grep gates, manual QA matrix, rollback guardrails.
- Không thêm test harness mới ở core; ưu tiên desktop shell validation hiện thực dụng.

## Files Likely Touched
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/render/chatminal_sidebar.rs`
- Modify: `apps/chatminal-desktop/src/tabbar.rs`
- Modify: `apps/chatminal-desktop/src/overlay/mod.rs`
- Create: plan-local QA checklist only if needed
- Delete: none

## Explicit Boundary
- Khong dung terminal core: validation phải chứng minh core untouched; mọi grep/diff gate phải fail nếu có edit trong `crates/chatminal-terminal-core/**`.

## Key Insights
- Roadmap này rủi ro chính nằm ở geometry/hit-test/overlay state regressions, không phải parser correctness.
- Gate đơn giản nhưng cứng hiệu quả hơn thêm framework test lớn ở giai đoạn này.

## Requirements
- Functional: shell flows quan trọng pass trên matrix sidebar on/off, session bar top/bottom, overlay active/inactive, multi-session layout.
- Non-functional: build pass, no-touch core diff pass, repaint/input regressions không rõ rệt.

## Architecture
- Validation stack:
  1. Static scope gate: `git diff --name-only`
  2. Build gate: `cargo check -p chatminal-desktop`
  3. Manual matrix gate: sidebar/session bar/footer/overlay/layout
  4. Smoke gate: scroll, resize, close, switch, overlay spawn/cancel

## Implementation Steps
1. Define no-touch diff/grep checklist cho core/runtime-sensitive zones.
2. Define manual QA matrix theo từng shell surface.
3. Verify performance-sensitive interactions: wheel scroll, hover, split drag, overlay open/close.
4. Freeze rollout only after all matrix cells pass.

## Todo List
- [ ] Add no-touch core validation step
- [ ] Write shell QA matrix
- [ ] Verify build and smoke flows
- [ ] Freeze rollout checklist

## Success Criteria
- `cargo check -p chatminal-desktop` pass.
- Không có modified file trong `crates/chatminal-terminal-core/**`.
- Sidebar/session bar/footer/overlay/layout/motion matrix pass.
- Team có rollback rule rõ nếu polish phase gây hit-test/render regressions.

## Risk Assessment
- Risk: polish đổi nhiều file shell cùng lúc làm khó isolate regressions.
- Mitigation: phase-by-phase landing, matrix verify mỗi phase, giữ changesets nhỏ.

## Security Considerations
- Verify destructive actions vẫn có affordance rõ và target đúng.
- Verify overlay/session switching không cross target sau refactor.

## Next Steps
- Sau khi toàn bộ phase xong mới cân nhắc docs sync ngoài plan folder và implementation handoff.
