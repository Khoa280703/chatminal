# Phase 05 - Overlay Search Copy Launcher Cutover

## Context Links
- `apps/chatminal-desktop/src/overlay/mod.rs`
- `apps/chatminal-desktop/src/overlay/copy.rs`
- `apps/chatminal-desktop/src/overlay/quickselect.rs`
- `apps/chatminal-desktop/src/overlay/launcher.rs`
- `apps/chatminal-desktop/src/overlay/confirm*.rs`

## Overview
- Priority: P1
- Status: completed
- Brief: đổi overlay subsystem khỏi `TermWizTerminal`, `Tab`, `Pane`, `PaneId`, `TabId`.

## Key Insights
- Overlay là mảng còn gắn rất sâu với hạ tầng WezTerm cũ.
- Đây là lý do plan cũ thường dừng trước “clean compile graph”.

## Requirements
- Overlay host first-party: `OverlayHost`, `OverlayTarget`, `OverlayTerminalBuffer`.
- Copy/search/select chạy trên render snapshot + terminal instance buffer, không trên `mux::Pane`.
- Launcher/confirm dùng `session_view_id` hoặc `terminal_instance_id`.

## Architecture
- Tạo bridge first-party từ terminal instance buffer sang overlay read/write API.
- Overlay không được lấy tab/window/pane từ global mux.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/overlay/mod.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/copy.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/quickselect.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/launcher.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/debug.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/confirm.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/confirm_close_pane.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/prompt.rs`
- Refactor: `apps/chatminal-desktop/src/overlay/selector.rs`

## Implementation Steps
1. Add first-party overlay target abstraction.
2. Port copy overlay from `Pane` delegate sang terminal-instance delegate.
3. Port quickselect/search overlay tương tự.
4. Port launcher/confirm flow sang session-view identifiers.
5. Delete `TermWizTerminal` allocation path khỏi desktop active flow.

## Todo List
- [x] Add overlay host abstraction
- [x] Port copy overlay
- [x] Port quickselect overlay
- [x] Port launcher/confirm ids
- [x] Remove `mux` overlay imports

## Success Criteria
- `rg -n "use mux::|mux::termwiztermtab|\\bTabId\\b|\\bPaneId\\b" apps/chatminal-desktop/src/overlay`
  - expected: zero active lines.

## Risk Assessment
- Risk: selection/search behavior lệch so với trước.
- Mitigation: targeted desktop tests cho copy, quickselect, confirm-close flows.

## Security Considerations
- Phải giữ clipboard/selection boundaries đúng, không leak text ngoài target instance.

## Next Steps
- Sang Phase 06 để bootstrap/frontend/event loop thôi hoàn toàn `mux`.
