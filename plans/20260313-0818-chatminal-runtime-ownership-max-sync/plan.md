# Chatminal Runtime Ownership Max Sync

Status: completed
Goal: đồng bộ kiến trúc tối đa để `chatminal-runtime` là application orchestrator duy nhất cho `profile/session/workspace_layout`, `chatminal-session-runtime` là execution subsystem nội bộ, `apps/chatminal-desktop` là thin client chỉ render + dispatch action, và `desktop_host_runtime` chỉ còn engine adapter/render backend.

## Why New Plan
- Các plan trước đã dọn product vocabulary và cắt `mux` khỏi active desktop path.
- Nhưng ownership vẫn còn phân tán giữa `chatminal-runtime`, `chatminal-session-runtime`, desktop helpers, và `desktop_host_runtime`.
- Plan này chỉ thành công khi mọi mutation/query product state đi qua một facade application-level duy nhất.

## Phases
- Phase 01 - Ownership Freeze And Application API Inventory
- Phase 02 - Runtime Facade Consolidation In `chatminal-runtime`
- Phase 03 - Desktop Mutation Routing Cutover
- Phase 04 - Desktop Query And Snapshot Cutover
- Phase 05 - Session Runtime Demotion Under Runtime Core
- Phase 06 - `desktop_host_runtime` Adapter Shrink And Desktop Simplification
- Phase 07 - Dependency Prune Delete And Final Verification

## Progress
- Phase 01: completed
- Phase 02: completed
- Phase 03: completed
- Phase 04: completed
- Phase 05: completed
- Phase 06: completed
- Phase 07: completed

## Target Architecture
- `chatminal-runtime`: source of truth duy nhất cho profile/session/workspace/layout/lifecycle/subscription API.
- `chatminal-session-runtime`: live runtime engine nội bộ được sở hữu và điều phối bởi `chatminal-runtime`.
- `apps/chatminal-desktop`: chỉ consume `runtime facade + desktop snapshot + runtime events`; không tự giữ business routing.
- `desktop_host_runtime`: terminal engine adapter/private render host; không còn quyết định product state.
- Desktop-facing API không lộ trực tiếp `chatminal-session-runtime` types trừ private engine adapter/render slices đã freeze.

## Primary Files In Scope
- `crates/chatminal-runtime/src/*`
- `crates/chatminal-session-runtime/src/*`
- `apps/chatminald/src/*`
- `crates/chatminal-protocol/src/*`
- `apps/chatminal-desktop/src/chatminal_runtime/*`
- `apps/chatminal-desktop/src/chatminal_desktop_session.rs`
- `apps/chatminal-desktop/src/chatminal_layout/*`
- `apps/chatminal-desktop/src/chatminal_render/*`
- `apps/chatminal-desktop/src/chatminal_sidebar/*`
- `apps/chatminal-desktop/src/desktop_host_runtime/*`
- `apps/chatminal-desktop/src/desktop_termwindow_*`
- `apps/chatminal-desktop/src/tabbar.rs`
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/termwindow/*`

## Out Of Scope
- `third_party/**`
- `apps/chatminal-app/**` compatibility TUI path
- terminal parser/render semantics ngoài plumbing/orchestration bắt buộc
- redesign UI/visual styling

## Hard Invariants
- Không đụng `third_party/`.
- Không xoá code UI user đang sửa; chỉ thay orchestration, runtime plumbing, boundary APIs.
- Không tái đưa `mux/tab/pane/leaf` làm product/app-facing concept.
- Không để desktop tự mutate `workspace_layout`, `active_session`, `active_profile` ngoài `chatminal-runtime` facade.
- Mỗi phase phải có grep/build/test gate trước khi sang phase sau.

## Completion Gates
- Desktop action/query path chỉ gọi `chatminal-runtime` facade cho product state.
- `chatminal-runtime` là owner duy nhất của workspace/session/profile mutation flow.
- `chatminal-session-runtime` không còn bị desktop gọi như một application layer ngang cấp.
- `desktop_host_runtime` không còn chứa app ownership logic cho profile/session/workspace.
- `rg -n --glob '!third_party/**' --glob '!vendor/**' --glob '!plans/**' --glob '!docs/**' "use chatminal_session_runtime|chatminal_session_runtime::" apps/chatminal-desktop/src`
  - expected: chỉ còn match trong engine adapter/private host files đã freeze ở Phase 01; zero match ở `chatminal_runtime/*`, `frontend.rs`, `termwindow/*`, `desktop_spawn.rs`, `desktop_mouse_actions.rs`, `desktop_overlay_actions.rs`
- `rg -n --glob '!third_party/**' --glob '!vendor/**' --glob '!plans/**' --glob '!docs/**' "persist_layout\\(|load_persisted_layout\\(|workspace_store::" apps/chatminal-desktop/src`
  - expected: zero match ngoài compatibility-free cache/snapshot module còn được Phase 04 chấp thuận; zero match trong `desktop_host_runtime/*`, `chatminal_desktop_session.rs`, `termwindow/*`
- `rg -n --glob '!third_party/**' --glob '!vendor/**' --glob '!plans/**' --glob '!docs/**' "profile_switch\\(|session_activate\\(|session_close\\(|workspace_layout_(load|save|clear)\\(|ensure_session_runtime\\(|focus_session_state\\(|focus_runtime_state\\(" apps/chatminal-desktop/src`
  - expected: zero direct mutation/query callsite ngoài `apps/chatminal-desktop/src/chatminal_runtime/*`
- `rg -n --glob '!third_party/**' --glob '!vendor/**' --glob '!plans/**' --glob '!docs/**' "active_profile_id|active_session_id|WorkspaceLayoutState|SessionViewId|RuntimeId|TerminalInstanceId" apps/chatminal-desktop/src`
  - expected: zero match ở `chatminal_desktop_session.rs`, `desktop_spawn.rs`, `desktop_mouse_actions.rs`, `desktop_overlay_actions.rs`, `frontend.rs`, `desktop_termwindow_actions_impl.rs`, `desktop_termwindow_session_close_helpers.rs`; chỉ còn match hợp lệ ở facade wrappers, snapshot/render modules, và private host adapter files
- `cargo check --workspace`
- `cargo test --manifest-path apps/chatminald/Cargo.toml -- --test-threads=1`
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`
- `cargo test -p chatminal-runtime -- --test-threads=1`
- `cargo test -p chatminal-session-runtime -- --test-threads=1`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`

## Verification Summary
- `cargo check --workspace`
- `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml -- --test-threads=1`
- `cargo test --manifest-path apps/chatminald/Cargo.toml -- --test-threads=1`
- `cargo test -p chatminal-runtime -- --test-threads=1`
- `cargo test -p chatminal-session-runtime -- --test-threads=1`
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1`
- Ownership grep gates sạch theo scope mục tiêu:
  - `chatminal_session_runtime` chỉ còn trong private engine adapter/host files đã freeze và test-only desktop store path
  - `workspace_store` chỉ còn trong facade wrapper + compatibility-free cache module được Phase 04 cho phép
  - direct desktop mutation verbs chỉ còn trong `apps/chatminal-desktop/src/chatminal_runtime/*`

## Done When
- Một chiều điều phối chuẩn là `desktop -> chatminal-runtime -> chatminal-session-runtime -> desktop_host_runtime`.
- Một chiều event chuẩn là `desktop_host_runtime -> chatminal-session-runtime -> chatminal-runtime -> desktop`.
- Desktop không còn tự ghép product state từ nhiều nguồn.
- Thay đổi profile/session/layout đều qua một runtime facade duy nhất.
