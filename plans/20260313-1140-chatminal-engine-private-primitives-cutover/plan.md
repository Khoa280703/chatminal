# Chatminal Engine Private Primitives Cutover

Status: completed
Goal: hoàn tất refactor để feature mới của Chatminal chỉ cần suy nghĩ bằng `profile/session/workspace/session_view/session_group/layout/render_target`, còn `Mux/Tab/Pane` tụt xuống private engine implementation detail.

## Why This Plan
- `chatminal-runtime` đã là app orchestrator, nhưng desktop/render/config path vẫn leak host vocabulary cũ.
- Nếu không dọn tiếp, mọi feature mới sẽ phải bridge hai mô hình: `session/layout` và `tab/pane`.
- Mục tiêu là refactor ownership + boundary, không rewrite terminal engine.

## Target Feature Model
- `Session`: execution unit của product; không còn là alias của `tab`.
- `SessionView`: attachment của một session vào workspace/layout hiện tại.
- `SessionGroup`: container layout cho nhiều `SessionView`, chuẩn bị cho model kiểu VSCode.
- `WorkspaceLayout`: tree của `SessionGroup` + `SessionView`; không phải host split tree public.
- `RenderTarget`/`TerminalInstance`: engine-facing realization của `SessionView`, private-facing.
- `Clone`: tạo session mới từ session nguồn; grouping là compose nhiều session views, không phải split nội bộ một session.

## Target Module Ownership
- `crates/chatminal-runtime`: app-facing source of truth.
- `crates/chatminal-session-runtime`: execution subsystem + live runtime registry.
- `apps/chatminal-desktop/src/chatminal_runtime/*`: desktop facade duy nhất.
- `apps/chatminal-desktop/src/termwindow/*`: render/input shell dùng vocabulary mới.
- `apps/chatminal-desktop/src/desktop_commands.rs`: compatibility translation layer, không phải product model source.
- `apps/chatminal-desktop/src/overlay/launcher.rs`: launcher UI phải consume translated semantics, không consume tab semantics trực tiếp.
- `apps/chatminal-desktop/src/desktop_host_runtime/*`: private engine adapter duy nhất ở desktop.
- `crates/chatminal-lua-bridge/*`: config/scripting adapter theo vocabulary Chatminal.
- `crates/chatminal-host-runtime/*`: lower engine library; allowed giữ `Mux/Tab/Pane`.

## Contract Appendices
- [Forbidden Symbols Contract](./appendices/forbidden-symbols-contract.md)
- [End-State Manifest](./appendices/end-state-manifest.md)
- [Future Feature Acceptance Matrix](./appendices/future-feature-acceptance-matrix.md)
- [Commit And Cutover Strategy](./appendices/commit-and-cutover-strategy.md)
- [Final Exit Checklist](./appendices/final-exit-checklist.md)

## Phases
- Phase 01: freeze vocabulary, allowed scopes, leakage inventory, grep gates
- Phase 02: tạo boundary types mới và migration contracts
- Phase 03: dồn ownership mapping/lookup về runtime facade
- Phase 04: refactor `termwindow` sang vocabulary Chatminal
- Phase 05: refactor Lua/config surface + compatibility policy
- Phase 06: privatize engine adapter, delete dead paths, tighten visibility
- Phase 07: verify full graph, xử lý `--all-targets`, sync docs, freeze boundary

## Progress
- Phase 01: completed
- Phase 02: completed
- Phase 03: completed
- Phase 04: completed
- Phase 05: completed
- Phase 06: completed
- Phase 07: completed

## Hard Invariants
- Không đụng `third_party/**`.
- Không rewrite terminal engine từ đầu.
- Không để feature mới ở app/UI layer cần mental-map `tab = session`.
- Không để `termwindow` hoặc Lua bridge own business state.
- Mỗi phase phải có grep/build/test gate riêng.

## Completion Gates
- Desktop app-facing path không còn import/trực tiếp thao tác `host_runtime::{Mux, Tab, Pane}`.
- `termwindow/*` không còn dùng `tab/pane` cho product semantics; chỉ còn compatibility names ở private render scopes nếu thật sự cần.
- `desktop_commands.rs` và `overlay/launcher.rs` chỉ còn là compatibility translation/UI shell, không giữ product semantics kiểu `ActivateTab/MoveTab`.
- `chatminal-lua-bridge` không còn public API `get_host_tab`, `get_host_leaf` ngoài shim deprecate có thời hạn hoặc bị xóa hẳn.
- `desktop_host_runtime` là desktop zone duy nhất còn thấy host primitives.
- Feature model tương lai `session -> session_view -> session_group -> workspace_layout` có thể implement mà không mở thêm refactor kiến trúc.
- `cargo check --workspace`, targeted tests, grep gates, và policy `--all-targets` đều xanh.
