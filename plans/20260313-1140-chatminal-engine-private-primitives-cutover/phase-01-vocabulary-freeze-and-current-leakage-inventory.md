# Phase 01 - Vocabulary Freeze And Current Leakage Inventory

## Context Links
- `plans/20260313-0818-chatminal-runtime-ownership-max-sync/plan.md`
- `appendices/forbidden-symbols-contract.md`
- `appendices/end-state-manifest.md`
- `appendices/future-feature-acceptance-matrix.md`
- `appendices/vocabulary-freeze-table.md`
- `reports/phase-01-current-leakage-inventory.md`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- `apps/chatminal-desktop/src/termwindow/mod.rs`
- `crates/chatminal-lua-bridge/src/lib.rs`
- `docs/system-architecture.md`

## Overview
- Priority: P0
- Status: completed
- Brief: khóa vocabulary đích của Chatminal và lập inventory chính xác mọi điểm còn leak `Mux/Tab/Pane` trước khi refactor sâu.

## Key Insights
- Nếu chưa chốt vocabulary đích, mọi refactor sau sẽ chỉ là rename rời rạc.
- Cần phân biệt rõ `product concept` với `engine primitive`.
- Một số chỗ còn từ cũ chỉ là UI label, một số chỗ là ownership leak thật; phải tách hai loại này.

## Requirements
- Định nghĩa vocabulary app-facing chuẩn:
  - `profile`
  - `session`
  - `workspace`
  - `session_view`
  - `session_group`
  - `workspace_layout`
  - `render_target`
  - `terminal_instance`
- Chốt từ cũ nào bị cấm ở app-facing path:
  - `mux`
  - `tab`
  - `pane`
  - `surface`
  - `leaf`
- Chốt từ cũ nào còn được phép tồn tại ở private engine layer.
- Lập inventory theo file/function/symbol cho các leakage còn sót.

## Architecture
- Vocabulary map phải chốt 1-1 trước khi code:
  - `Mux` -> `SessionExecutionRegistry` hoặc `EngineRegistry` trong private engine scope
  - `Window` -> `DesktopWindowBinding` ở app-facing; `HostWindow` ở private engine scope
  - `Tab` -> `SessionRenderTarget` ở app-facing; `HostRenderScope` ở private engine scope
  - `Pane` -> `TerminalInstance` ở app-facing; `HostTerminal` ở private engine scope
  - `Surface` -> cấm ở product layer; thay bằng `SessionView` hoặc `SessionGroup`
  - `Leaf` -> cấm ở product layer; thay bằng `TerminalInstance`
- Feature model phải freeze ngay:
  - `Session` là execution unit
  - `SessionView` là một attachment của session vào layout
  - `SessionGroup` là tree container cho nhiều views
  - split/group về sau thao tác trên `SessionGroup`, không thao tác trực tiếp trên host leaf tree
- Output của phase này là một boundary map:
  - App layer: `chatminal-runtime`, desktop facade, sidebar/session bar, action routing
  - Render shell: `termwindow`, overlay, selection, mouse/key handling
  - Engine adapter: `desktop_host_runtime`, `chatminal-session-runtime`, `chatminal-host-runtime`
  - Config/script: `chatminal-lua-bridge`
- Boundary map phải chỉ ra nơi nào là public, nơi nào là private-only.

## Related Code Files
- Refactor: `plans/20260313-1140-chatminal-engine-private-primitives-cutover/plan.md`
- Refactor: `docs/system-architecture.md`
- Audit/Map: `apps/chatminal-desktop/src/main.rs`
- Audit/Map: `apps/chatminal-desktop/src/desktop_commands.rs`
- Audit/Map: `apps/chatminal-desktop/src/overlay/launcher.rs`
- Read/Audit: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Read/Audit: `apps/chatminal-desktop/src/desktop_host_runtime/*`
- Read/Audit: `apps/chatminal-desktop/src/termwindow/*`
- Read/Audit: `crates/chatminal-lua-bridge/src/*`

## Implementation Steps
1. Freeze vocabulary table `old -> new -> allowed scope`.
2. Grep toàn repo active path theo các keyword `mux`, `tab`, `pane`, `surface`, `leaf`.
3. Phân loại kết quả thành:
   - product leak
   - render-shell legacy name
   - private adapter acceptable
   - test/docs only
4. Lập file matrix `keep / rename / refactor / privatize / delete`.
5. Lập danh sách symbol cần đổi tên và danh sách API cần rút vào private.
6. Chốt gating queries dùng lại cho các phase sau.

## File Decision Matrix
- `apps/chatminal-desktop/src/chatminal_runtime/*`: keep + refactor
- `apps/chatminal-desktop/src/desktop_host_runtime/*`: keep + privatize
- `apps/chatminal-desktop/src/desktop_termwindow_*`: keep + rename/refactor
- `apps/chatminal-desktop/src/termwindow/*`: keep + refactor
- `apps/chatminal-desktop/src/desktop_commands.rs`: keep + compatibility translation isolation
- `apps/chatminal-desktop/src/overlay/launcher.rs`: keep + refactor
- `crates/chatminal-runtime/src/*`: keep + refactor
- `crates/chatminal-session-runtime/src/*`: keep + refactor/private-only tightening
- `crates/chatminal-lua-bridge/src/*`: keep + cutover/deprecate/delete selective
- `crates/chatminal-host-runtime/src/*`: keep; only privatize surface if needed

## Phase Gates
- Freeze grep set:
  - `\\bmux\\b|mux::`
  - `\\btab\\b|Tab\\b|tab_`
  - `\\bpane\\b|Pane\\b|pane_`
  - `\\bsurface\\b|Surface\\b|surface_`
  - `\\bleaf\\b|Leaf\\b|leaf_`
- Chốt allowlist scopes:
  - `apps/chatminal-desktop/src/desktop_host_runtime/*`
  - `crates/chatminal-host-runtime/src/*`
  - `crates/chatminal-session-runtime/src/*` chỉ cho engine-private meanings
  - tests/docs/plans

## Review Rule
- Inventory chỉ được coi là complete khi đã reconcile với:
  - [Forbidden Symbols Contract](./appendices/forbidden-symbols-contract.md)
  - [End-State Manifest](./appendices/end-state-manifest.md)
  - [Future Feature Acceptance Matrix](./appendices/future-feature-acceptance-matrix.md)

## Todo List
- [x] Viết vocabulary freeze table
- [x] Lập leakage inventory theo module
- [x] Lập file decision matrix
- [x] Chốt allowed zones cho vocabulary cũ
- [x] Chốt grep gates cho Phase 02-07
- [x] Review inventory để tránh over-delete

## Success Criteria
- Có danh sách chính xác file nào sẽ refactor, file nào giữ nguyên tạm thời.
- Có file-level decision đủ để Phase 02-06 không phải đoán.
- Có vocabulary đích không mơ hồ cho toàn bộ phases sau.
- Không còn tranh cãi “tab ở đây là product hay engine primitive”.

## Deliverables
- [Vocabulary Freeze Table](./appendices/vocabulary-freeze-table.md)
- [Phase 01 Current Leakage Inventory](./reports/phase-01-current-leakage-inventory.md)

## Risk Assessment
- Risk: gộp nhầm render-shell legacy name vào app leak rồi xóa quá tay.
- Mitigation: bắt buộc annotate từng leakage theo scope và owner.

## Security Considerations
- Không thay đổi runtime permissions, transport, target attachment ở phase inventory.

## Next Steps
- Phase 02 tạo type layer và contracts đúng theo vocabulary freeze table, không tự nghĩ tên mới ngoài freeze table.
