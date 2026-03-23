# Phase 01 - Boundary Freeze And Shell Surface Inventory

## Context Links
- `README.md`
- `docs/codebase-summary.md`
- `docs/system-architecture.md`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`

## Overview
- Priority: P1
- Status: pending
- Brief: khóa phạm vi UI shell, chốt vocabulary, gom inventory trước khi đụng layout/render polish

## Objective
- Freeze phạm vi chỉ ở desktop UI shell quanh terminal.
- Gắn rõ owner cho sidebar/session bar/footer/overlay/layout primitives.

## Scope
- Lập matrix `safe touch` vs `no touch`.
- Map render/input/layout seams trong `apps/chatminal-desktop`.
- Chốt naming cho `render target`, `session`, `overlay`, `terminal chrome`.

## Files Likely Touched
- Modify: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Modify: `apps/chatminal-desktop/src/frontend.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- Modify: `apps/chatminal-desktop/src/desktop_termwindow_event_helpers.rs`
- Create: none preferred
- Delete: none

## Explicit Boundary
- Khong dung terminal core: không sửa `crates/chatminal-terminal-core/**`, không đổi parser/state semantics, không thêm behavior vào PTY/runtime/store path.

## Key Insights
- Shell source of truth đang nằm ở `termwindow/*` + `desktop_termwindow_*` + `tabbar.rs` + `chatminal_sidebar`.
- `desktop_host_runtime/*` chỉ là adapter seam; phase này chỉ được consume seam, không mở rộng host/runtime ownership.

## Requirements
- Functional: mọi phase sau phải dựa trên inventory này, không leak scope sang core/runtime.
- Non-functional: giữ roadmap implementable bởi app-layer team, không yêu cầu schema/protocol changes.

## Architecture
- Define ba vùng: `shell safe zone`, `adapter seam`, `black box`.
- `shell safe zone`: render tree, mouse hit test, chrome metrics, overlay visuals, session bar visuals.
- `adapter seam`: helper resolve render target/session/overlay.
- `black box`: terminal core + parser/state semantics + PTY/store ownership.

## Implementation Steps
1. Audit toàn bộ file shell path đang ảnh hưởng sidebar/session bar/footer/overlay/layout.
2. Đánh dấu explicit no-touch list trong plan comments/docstrings nếu cần.
3. Chuẩn hóa terminology dùng trong roadmap để phase sau không drift.
4. Chốt acceptance gates: `cargo check -p chatminal-desktop`, git diff no-touch core.

## Todo List
- [ ] Write safe-touch inventory
- [ ] Freeze no-touch zone list
- [ ] Confirm adapter seams only
- [ ] Confirm validation gates for all later phases

## Success Criteria
- Có boundary matrix rõ ràng cho toàn roadmap.
- Mọi phase sau chỉ tham chiếu app-layer files cụ thể.
- Không còn ambiguity về việc phase nào được chạm runtime/core.

## Risk Assessment
- Risk: scope creep sang host/runtime helpers hoặc terminal semantics.
- Mitigation: enforce file allowlist từ đầu và add grep-based no-touch gate.

## Security Considerations
- Không nới rộng input routing quyền truy cập qua session boundaries.
- Không expose thêm host/core identifiers ra UI shell public path.

## Next Steps
- Sang Phase 02 để chuẩn hóa layout primitives và geometry contract.
