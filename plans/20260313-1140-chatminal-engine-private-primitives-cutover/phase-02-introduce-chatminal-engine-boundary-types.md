# Phase 02 - Introduce Chatminal Engine Boundary Types

## Context Links
- `crates/chatminal-runtime/src/lib.rs`
- `crates/chatminal-runtime/src/api/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/chatminal_render/mod.rs`
- `crates/chatminal-session-runtime/src/lib.rs`

## Overview
- Priority: P0
- Status: completed
- Brief: tạo layer type/app contract của Chatminal để desktop và runtime không phải nói trực tiếp bằng `Tab/Pane` semantics nữa; engine behavior cũ vẫn được reuse qua adapter.

## Key Insights
- Không nên đổi tên trực tiếp từ `Mux/Tab/Pane` sang `Session/...` khắp codebase trong một nhát.
- Cần một boundary type layer làm adapter ổn định trước, rồi mới migrate callsites.
- Type layer này phải là app-facing và không lộ host engine internals.

## Requirements
- Tạo hoặc chuẩn hóa các type sau ở runtime-facing path:
  - `SessionRenderTargetId`
  - `SessionRenderTargetSnapshot`
  - `SessionTerminalHandle`
  - `SessionLayoutTarget`
  - `SessionGroupId`
  - `SessionGroupSnapshot`
  - `SessionViewBinding`
  - `SessionWindowBinding`
  - `SessionEngineCapability`
- Nếu cần compatibility, dùng type alias/private conversion ở adapter layer, không export host types ra ngoài.
- Dồn các DTO desktop-facing về `chatminal-runtime` hoặc `apps/chatminal-desktop/src/chatminal_runtime/*`.

## Architecture
- `chatminal-runtime`: owner của public boundary types.
- `chatminal-session-runtime`: owner của live execution ids nội bộ.
- `desktop_host_runtime`: implement conversion `Chatminal boundary type <-> host_runtime primitive`.
- `termwindow`: chỉ consume boundary types mới.

## Boundary Type Matrix
- `RuntimeId`:
  - keep internal ở runtime/session-runtime
  - không dùng làm product identity ở UI/action layer
- `TerminalInstanceId`:
  - keep internal/execution-facing
  - chỉ lộ ở render/terminal handling paths hợp lệ
- `SessionRenderTargetId`:
  - app-facing id cho render attachment đang active trong window
- `SessionViewId`:
  - app-facing identity cho một session attachment trong workspace
- `SessionGroupId`:
  - app-facing identity cho future grouping/layout containers
- `SessionTerminalHandle`:
  - UI/runtime handle để focus/copy/selection mà không cần nói `Pane`
- `SessionWindowBinding`:
  - mapping `DesktopWindowId -> active render target/session context`

## File Ownership Matrix
- `crates/chatminal-runtime/src/lib.rs`: export boundary types công khai
- `crates/chatminal-runtime/src/api/mod.rs`: DTO/snapshot contract
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`: facade wrappers + conversions
- `apps/chatminal-desktop/src/chatminal_render/mod.rs`: render DTO theo type mới
- `apps/chatminal-desktop/src/desktop_host_runtime/*`: private conversion only

## Related Code Files
- Refactor: `crates/chatminal-runtime/src/lib.rs`
- Refactor: `crates/chatminal-runtime/src/api/mod.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_render/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`

## Implementation Steps
1. Định nghĩa boundary types mới và nơi export chính thức.
2. Viết conversion helpers từ host primitive sang boundary types trong private adapter.
3. Thay các return type/query type của facade để dùng boundary types mới.
4. Thêm snapshot/render DTO mới nếu hiện tại termwindow còn cần đọc metadata host trực tiếp.
5. Viết unit tests cho conversion và snapshot shape.

## Phase Gates
- `rg -n "host_runtime::(Mux|tab::Tab|pane::Pane)|\\bMuxWindow\\b|OverlayRenderScope" apps/chatminal-desktop/src/chatminal_runtime apps/chatminal-desktop/src/chatminal_render`
  - expected: zero product-facing host primitive leaks
- `rg -n "SessionRenderTargetId|SessionGroupId|SessionTerminalHandle|SessionWindowBinding" crates/chatminal-runtime apps/chatminal-desktop/src`
  - expected: new types exist and are used by facade path

## Todo List
- [x] Boundary types được định nghĩa và export rõ ràng
- [x] Desktop facade trả về type mới thay vì host vocabulary
- [x] Adapter conversion nằm ở private host layer
- [x] File ownership matrix được thực thi ở các type entrypoints chính
- [x] Test shape/snapshot pass
- [x] Không tăng direct dependency desktop -> host_runtime

## Success Criteria
- Có thể migrate callsite mà không cần expose `host_runtime::Mux/Tab/Pane` thêm nữa.
- Desktop product/render path có vocabulary Chatminal riêng.

## Risk Assessment
- Risk: boundary types thiết kế sai khiến Phase 03-04 lại phải churn lớn.
- Mitigation: giữ types tối thiểu, bám sát nhu cầu thật của current callsites.

## Security Considerations
- Không để IDs mới trở thành security token hay public stable external contract khi chưa cần.

## Next Steps
- Phase 03 thay callsites lookup/focus/close sang boundary types mới; không thêm alias ngược từ types mới về `tab/pane`.
- Report: `reports/phase-02-boundary-types-cutover.md`
