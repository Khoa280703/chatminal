# Phase 06 - Desktop Bootstrap Frontend And Event Loop Cutover

## Context Links
- `apps/chatminal-desktop/src/main.rs`
- `apps/chatminal-desktop/src/frontend.rs`
- `apps/chatminal-desktop/src/update.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/*`

## Overview
- Priority: P0
- Status: completed
- Brief: loại `Mux` global khỏi bootstrap, frontend subscriptions, update loop, target bootstrap.

## Key Insights
- Dù render/action đã sạch, desktop vẫn chưa sạch nếu `main/frontend` còn tạo `Mux` global và route event qua `MuxNotification`.

## Requirements
- Tạo event bus first-party cho desktop window/runtime.
- Frontend loop subscribe `chatminal-runtime` và session host events, không subscribe mux notifications.
- `main.rs` bootstrap window/target/session runtime first-party, không dựng `Arc<mux::Mux>`.

## Architecture
- Add `DesktopRuntimeBus` hoặc module tương đương cho window invalidation, input/output notifications, workspace changes.
- `chatminal_runtime/spawn_target.rs` nếu còn cần target abstraction thì phải first-party hoặc engine-private không lộ `mux`.

## Related Code Files
- Refactor: `apps/chatminal-desktop/src/main.rs`
- Refactor: `apps/chatminal-desktop/src/frontend.rs`
- Refactor: `apps/chatminal-desktop/src/update.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- Refactor: `apps/chatminal-desktop/src/chatminal_runtime/client.rs`
- Refactor: `apps/chatminal-desktop/src/desktop_host_runtime/*`

## Implementation Steps
1. Introduce desktop event bus first-party.
2. Move pane output/input/focus notifications sang bus mới.
3. Rewrite frontend loop/subscriptions.
4. Rewrite startup/bootstrap code để không tạo global mux.
5. Ensure smoke startup/shutdown/session switching pass.

## Todo List
- [x] Add desktop event bus
- [x] Replace frontend subscriptions
- [x] Replace startup bootstrap
- [x] Remove global mux init
- [x] Add smoke tests

## Success Criteria
- `rg -n "Mux::get\\(|MuxNotification|client::ClientId|connui" apps/chatminal-desktop/src/main.rs apps/chatminal-desktop/src/frontend.rs apps/chatminal-desktop/src/update.rs apps/chatminal-desktop/src/chatminal_runtime`
  - expected: zero active desktop-path lines.

## Risk Assessment
- Risk: boot order regression, app start được nhưng session host không attach.
- Mitigation: smoke tests cho fresh start, restore, create session, switch session, close app, reopen.

## Security Considerations
- Không chạm network/auth; chú ý runtime bus không gây deadlock hoặc cross-thread unsafe behavior.

## Next Steps
- Sang Phase 07 để prune dependency, delete crate cũ, verify zero residual.
