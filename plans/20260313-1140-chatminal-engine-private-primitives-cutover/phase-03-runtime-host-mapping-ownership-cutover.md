# Phase 03 - Runtime/Host Mapping Ownership Cutover

## Context Links
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- `crates/chatminal-runtime/src/state/native_api.rs`
- `crates/chatminal-session-runtime/src/session_runtime_state.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: làm cho mọi mapping `session -> runtime -> render_target -> terminal_instance` thuộc ownership của runtime/app boundary, không nằm rải rác ở termwindow hay host helper.

## Key Insights
- Đây là phase quyết định source of truth thật sự.
- Nếu `termwindow` còn tự ghép `render_scope_id`, `pane metadata`, `active tab`, thì vocabulary mới vẫn chỉ là vỏ.

## Requirements
- Runtime facade phải own các query sau:
  - active session lookup
  - active session view lookup
  - active session group lookup
  - active render target lookup
  - render target capability lookup
  - terminal handle for focused session target
  - close/focus routing by session vocabulary
- `desktop_host_runtime` chỉ trả về private host snapshots hoặc apply engine actions.
- Bỏ các helper công khai ở host layer đang encode business semantics.

## File Ownership Matrix
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`: keep; becomes single desktop query/mutation surface
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`: keep; no host vocabulary leak
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`: privatize lookup helpers
- `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`: shrink to IO/render-only helpers
- `crates/chatminal-runtime/src/state/runtime_bridge.rs`: keep; becomes runtime-owned mapping owner
- `crates/chatminal-runtime/src/state/native_api.rs`: keep; app/native facade implementation

## Architecture
- App query path: `termwindow/frontend -> chatminal_runtime facade -> chatminal-runtime -> host adapter if needed`.
- Host query path nội bộ: `desktop_host_runtime` trả về host primitives/private render snapshots.
- Không còn callsite business đi thẳng vào `session_host` để lấy active state.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_termwindow_host_runtime_helpers.rs`
- Refactor: `crates/chatminal-runtime/src/state/runtime_bridge.rs`
- Refactor: `crates/chatminal-runtime/src/state/native_api.rs`

## Implementation Steps
1. Audit toàn bộ helper lấy active runtime/render scope/pane handle.
2. Di chuyển logic resolve vào facade runtime.
3. Rút public methods không cần thiết khỏi `DesktopSessionHost`.
4. Migrate callsites sang boundary queries mới.
5. Xóa fallback path còn đọc pane metadata cho business routing nếu đã có source chuẩn.

## Exact Callsite Targets
- `termwindow/mod.rs`
  - active session lookup
  - render target lookup
  - close/focus routing
- `desktop_spawn.rs`
  - current session resolution
- `desktop_termwindow_close_helpers.rs`
  - close semantics by session/view
- `desktop_termwindow_host_runtime_helpers.rs`
  - only keep terminal IO/render helpers, drop business lookup helpers

## Phase Gates
- `rg -n "active_tab|render_scope_id|pane_metadata_(runtime_id|terminal_instance_id)|host_active_render_scope" apps/chatminal-desktop/src/termwindow apps/chatminal-desktop/src/desktop_termwindow_*`
  - expected: zero business-routing usage outside allowed private render helper scopes
- `rg -n "pub fn .*runtime_id_for_session|pub fn .*render_state_for_runtime|pub fn .*active_terminal_handle" apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
  - expected: only render/IO-safe public helpers remain

## Todo List
- [x] Runtime facade own toàn bộ active lookup
- [x] Host layer không còn public business lookup helpers
- [x] Termwindow không còn tự bridge `session -> host tab/pane`
- [x] Session/view/group lookup source of truth nằm ở runtime facade
- [x] Focus/close routing đi qua facade
- [x] Integration tests pass

## Success Criteria
- `DesktopSessionHost` không còn là app controller trá hình.
- `termwindow` không còn tự biết host topology cho các quyết định business.

## Risk Assessment
- Risk: focus/close regressions, đặc biệt với overlay và pane selection.
- Mitigation: thêm focused integration tests và manual smoke matrix cho activate/close/split/group flows.

## Security Considerations
- Không để runtime mapping leak ra scripting/public APIs ngoài phạm vi cần thiết.

## Next Steps
- Phase hoàn tất; ownership mapping đã dồn về runtime facade.
- Batch logs:
  - `reports/phase-03-batch-01-terminal-handle-routing.md`
