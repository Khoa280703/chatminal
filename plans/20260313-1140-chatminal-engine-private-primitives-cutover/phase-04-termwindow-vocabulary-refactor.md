# Phase 04 - TermWindow Vocabulary Refactor

## Context Links
- `appendices/forbidden-symbols-contract.md`
- `appendices/end-state-manifest.md`
- `appendices/commit-and-cutover-strategy.md`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `apps/chatminal-desktop/src/termwindow/render/*`
- `apps/chatminal-desktop/src/desktop_termwindow_*`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/chatminal_sidebar/mod.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: đổi `termwindow` từ shell nói tiếng `tab/pane` sang shell nói tiếng `session/render_target/layout_target`, nhưng vẫn giữ nguyên behavior render/input.

## Key Insights
- Đây là chỗ nhiều code nhất và dễ sinh “rename half-done”.
- Không nên rename bừa labels/UI constants trước khi đổi type/helper names có hệ thống.

## Requirements
- Đổi tên và tái cấu trúc các nhóm sau:
  - `tab bar` product semantics -> `session bar` / `session strip`
  - `pane` product semantics -> `terminal instance` hoặc `terminal target`
  - `render scope` -> `session render target` nếu app-facing
  - `CloseTab`, `ActivateTab`, `MoveTab` product-level actions -> action names mới phù hợp
  - `SplitPane` product-level action -> `SplitSessionGroup` hoặc `ArrangeSessionGroup`
- Chỉ giữ `tab/pane` nếu đó là private overlay/render compatibility type.
- Tách rõ file nào là app action, file nào là render compatibility helper.

## Rename Matrix
- `TabBarItem` -> `SessionBarItem`
- `TabBarState` -> `SessionBarState`
- `CloseTab` -> `CloseSessionView` hoặc `CloseSessionGroupEntry`
- `ActivateTab` -> `ActivateSessionView`
- `ActivateLastTab` -> `ActivateLastSessionView`
- `MoveTab` -> `MoveSessionView`
- `TerminalPane` UI names -> `TerminalInstance`
- `active_tab_overlay` -> `active_render_target_overlay`
- `render_scope_id` public names -> `session_render_target_id`

## File Ownership Matrix
- `apps/chatminal-desktop/src/tabbar.rs`: keep + rename/refactor
- `apps/chatminal-desktop/src/termwindow/mod.rs`: keep + refactor
- `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`: keep + refactor
- `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`: keep + refactor
- `apps/chatminal-desktop/src/desktop_termwindow_types.rs`: keep + refactor
- `apps/chatminal-desktop/src/desktop_commands.rs`: keep + translation-layer only
- `apps/chatminal-desktop/src/overlay/launcher.rs`: keep + translated action consumer
- `apps/chatminal-desktop/src/desktop_termwindow_mouseevent.rs`: keep + refactor away from `CloseTab` UI semantics
- `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`: keep + refactor
- `apps/chatminal-desktop/src/termwindow/render/*`: selective private compatibility naming allowed

## Architecture
- `termwindow/mod.rs`: coordinator, consume facade/snapshot/boundary types.
- `desktop_termwindow_*`: chia theo action/render/input/selection bằng vocabulary mới.
- `termwindow/render/*`: có thể giữ render compatibility internals, nhưng public entry names phải theo vocabulary mới.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/termwindow/mod.rs`
- Refactor: `apps/chatminal-desktop/src/tabbar.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_layout_render.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_types.rs`
- Refactor: `apps/chatminal-desktop/src/termwindow/render/tab_bar.rs`

## Implementation Steps
1. Tạo rename map cho action names, helper names, enum variants.
2. Refactor type names ở `desktop_termwindow_types.rs` trước.
3. Refactor action enums/callsites ở `desktop_termwindow_actions_*`.
4. Refactor `termwindow/mod.rs` để dùng boundary names mới.
5. Giữ shim tạm thời cục bộ nếu cần, rồi xóa khi tests pass.

## Compatibility Policy
- UI labels:
  - đổi thẳng, không cần shim
- internal method names:
  - cho phép shim tối đa 1 phase nếu churn lớn
- command/keyassignment names từ upstream:
  - nếu đụng config compatibility, giữ translation layer riêng trong `desktop_commands.rs`; không leak vào product code
- launcher UI:
  - chỉ consume translated action names hoặc explicit compatibility translation output

## Phase Gates
- `rg -n "CloseTab|ActivateTab|ActivateLastTab|MoveTab|ActivateTabRelative|ActivateTabRelativeNoWrap|MoveTabRelative|TabBarItem|TabBarState" apps/chatminal-desktop/src/termwindow apps/chatminal-desktop/src/desktop_termwindow_* apps/chatminal-desktop/src/tabbar.rs apps/chatminal-desktop/src/desktop_commands.rs apps/chatminal-desktop/src/overlay/launcher.rs`
  - expected: zero product-level residuals
- `rg -n "UIItemType::CloseTab|active_tab_overlay|active_terminal_instance_from_active_tab|activate_pane_index_in_active_tab|activate_pane_direction_in_active_tab" apps/chatminal-desktop/src/termwindow apps/chatminal-desktop/src/desktop_termwindow_*`
  - expected: zero in product-facing shell

## Todo List
- [x] Product-facing action names không còn `tab/pane`
- [x] Termwindow state/helpers dùng vocabulary mới
- [x] Render helpers chỉ giữ legacy names ở private-only scopes
- [x] File ownership matrix được thực thi ở UI/action modules
- [x] Session bar/session switching code dễ đọc lại
- [x] Desktop tests/render tests pass

## Success Criteria
- Dev đọc `termwindow` không cần mental-map `tab = session` nữa.
- Feature mới ở UI layer có thể viết thẳng theo `session/layout` model.

## Risk Assessment
- Risk: rename quá rộng làm noise lớn và khó review.
- Mitigation: chia commit theo module, giữ grep gates rõ cho renamed symbols.

## Security Considerations
- Không ảnh hưởng transport/security boundary; trọng tâm là naming + ownership readability.

## Next Steps
- Phase 05 dọn scripting/config surface để vocabulary mới thống nhất cả ở runtime hook layer và user extension layer.
- Batch logs:
  - `reports/phase-04-batch-01-render-target-helper-vocabulary.md`
  - `reports/phase-04-batch-02-session-bar-translation.md`
