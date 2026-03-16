# Phase 06 - Engine Adapter Privatization And Dead Path Removal

## Context Links
- `apps/chatminal-desktop/src/desktop_host_runtime/*`
- `crates/chatminal-host-runtime/src/*`
- `crates/chatminal-session-runtime/src/*`
- `apps/chatminal-desktop/src/termwindow/*`
- `apps/chatminal-desktop/src/chatminal_runtime/*`

## Overview
- Priority: P0
- Status: completed
- Brief: sau khi boundary mới đã ổn định, đẩy nốt `Mux/Tab/Pane` và các helper tương ứng xuống private-only zone, rồi xóa đường cũ.

## Key Insights
- Đây là phase “thu hoạch” của các phase trước.
- Nếu làm phase này quá sớm sẽ dễ gãy; nếu làm đúng thứ tự thì phần lớn là delete + tighten visibility.

## Requirements
- Thu hẹp visibility của host/engine APIs:
  - `pub -> pub(crate)` hoặc private nếu không còn consumer hợp lệ
- Xóa helper/callsite cũ đã bị supersede.
- Tách rõ module nào là public desktop boundary, module nào là private engine adapter.
- Nếu còn `tab/pane` symbols ở public desktop path, phải migrate hoặc annotate rõ compatibility-only rồi lên batch xóa tiếp.

## File Decision Matrix
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`: keep + privatize exports
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`: keep + shrink public surface
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`: keep private
- `apps/chatminal-desktop/src/desktop_termwindow_render_mod.rs`: delete if graph proves dead
- `apps/chatminal-desktop/src/desktop_termwindow_render_pane.rs`: delete if superseded by `termwindow/layout_render.rs`
- dead helper shims in `chatminal_runtime/*`, `desktop_termwindow_*`, `termwindow/*`: delete once no callsites
- benchmark/test targets outside active maintenance:
  - either fix or explicitly remove from active graph in Phase 07 policy

## Dead Path Candidates To Confirm
- legacy helper wrappers that only rename host IDs
- `#[allow(dead_code)]` methods in `session_host.rs` after callsite migration
- dead desktop module splits created during migration but no longer imported
- deprecate shims from Phase 04-05 once no callsite remains

## Architecture
- `desktop_host_runtime`: private adapter package.
- `chatminal-host-runtime`: lower engine library, có thể còn `Mux/Tab/Pane`, nhưng không được leak ra app-facing desktop path.
- `chatminal-session-runtime`: execution subsystem với live runtime/layout, không phải host vocabulary facade.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- Refactor: `crates/chatminal-host-runtime/src/lib.rs`
- Refactor: `crates/chatminal-session-runtime/src/lib.rs`
- Delete: dead helpers/modules phát sinh sau Phase 02-05

## Implementation Steps
1. Chạy visibility audit cho desktop-facing exports.
2. Thu hẹp `pub` surface ở host/runtime crates.
3. Xóa helper/module cũ không còn callsite.
4. Xóa hoặc sửa `dead_code` suppressions không còn cần.
5. Re-run grep/build/tests sau mỗi batch delete.

## Phase Gates
- `rg -n "pub fn .*host_|pub fn .*pane_|pub fn .*tab_" apps/chatminal-desktop/src/desktop_host_runtime apps/chatminal-desktop/src/chatminal_runtime`
  - expected: zero unnecessary public host-vocabulary helpers
- `rg -n "allow\\(dead_code\\)|dead_code" apps/chatminal-desktop/src/desktop_host_runtime apps/chatminal-desktop/src/termwindow crates/chatminal-lua-bridge crates/chatminal-session-runtime`
  - expected: reduced significantly with every retained one justified

## Todo List
- [x] Desktop-facing path không còn leak host primitives
- [x] Host adapter public surface được thu gọn đáng kể
- [x] Dead modules/helpers bị delete
- [x] `#[allow(dead_code)]` giảm đáng kể ở các module vừa refactor
- [x] File decision matrix được thực thi hoặc annotated rõ những ngoại lệ còn lại
- [x] Workspace build/test vẫn xanh

## Success Criteria
- `Mux/Tab/Pane` chỉ còn ở engine/private adapter zones.
- Public graph của Chatminal nhìn đúng như app riêng, không như fork đang trộn hai mô hình.

## Risk Assessment
- Risk: xóa nhầm code hiếm dùng như overlay/selection/scripting path.
- Mitigation: delete theo batch nhỏ, luôn có grep + targeted tests + smoke.

## Security Considerations
- Không để shrink visibility làm lộ workaround insecure hoặc bỏ validation hiện có ở runtime facade.

## Next Steps
- Phase 07 khóa verification, docs sync, `--all-targets` policy, và final DOD.
- Batch log:
  - `reports/phase-06-batch-01-engine-adapter-privatization.md`
