# Phase 02 - Chatminal IPC Mux Domain Adapter

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/crates/chatminal-protocol/src/lib.rs](/home/khoa2807/working-sources/chatminal/crates/chatminal-protocol/src/lib.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/ipc/client.rs](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/ipc/client.rs)
- [/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/request_handler.rs](/home/khoa2807/working-sources/chatminal/apps/chatminald/src/state/request_handler.rs)
- [/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-gui/src/frontend.rs](/home/khoa2807/working-sources/chatminal/third_party/wezterm/wezterm-gui/src/frontend.rs)

## Overview
- Priority: P1
- Status: Completed
- Effort: 1w
- Brief: build adapter translating WezTerm mux actions to chatminal IPC without changing daemon ownership; hiện đã có bridge/proxy path, còn thiếu mux-domain embedded path.

## Key Insights
- `ChatminalClient` already has robust request/event framing + timeout behavior.
- Protocol already exposes everything needed for session lifecycle and input/resize/output.
- Biggest risk is semantic mismatch between mux pane lifecycle and daemon session lifecycle.

## Requirements
- Functional:
1. Map pane create/activate/input/resize/close to existing requests.
2. Consume event stream (`PtyOutput`, `SessionUpdated`, `WorkspaceUpdated`) and update mux state.
3. Support snapshot bootstrap for attach/reconnect to preserve history continuity.
- Non-functional:
1. No blocking UI thread on IPC waits.
2. Backpressure and error propagation remain visible to UI.

## Architecture
- New module: `chatminal_ipc_mux_domain`.
- Internal maps: `session_id <-> pane_id`, `profile_id <-> workspace context`.
- Event pump thread/task pushes updates into main GUI/mux thread via channel.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs`
- Create:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window_wezterm_gui/mod.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window_wezterm_gui/chatminal_ipc_mux_domain.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window_wezterm_gui/chatminal_ipc_mux_domain_tests.rs`
- Delete:
1. None

## Implementation Steps
1. Define adapter trait surface for mux actions.
2. Implement request translation table and response normalization.
3. Implement event fan-in path + stale event guards (reuse watermark concept from current binding runtime).
4. Add unit tests for lifecycle race cases (reconnect, stale event, exited session).

## Todo List
- [x] Request mapping cơ bản cho `activate/snapshot/input/resize` đã chạy trong proxy.
- [x] Event ordering/backlog guards cơ bản đã có (fair-drain + batch input + bounded queue).
- [x] Snapshot restore path đã hoạt động khi attach/reconnect qua proxy.
- [x] Hoàn tất module `chatminal_ipc_mux_domain` embedded cho đường chạy window/proxy hiện tại (tách logic lifecycle/input/event khỏi launcher).
- [x] Unit tests race-case cho mux-domain embedded path.

## Success Criteria
- Adapter can drive full session lifecycle using unchanged daemon API.
- No regressions in session/profile/history persistence behavior.

## Risk Assessment
- Risk: mux expects richer state transitions than daemon emits.
- Mitigation: explicit local adapter state machine + fallback workspace reload on ambiguity.

## Security Considerations
- Keep input payload size and daemon-side limits unchanged.
- Preserve local-only transport policy (UDS/NamedPipe).

## Next Steps
- Integrate adapter into new wezterm-gui runtime (Phase 03).

## Unresolved Questions
1. Should adapter cache workspace state aggressively, or always refresh on `WorkspaceUpdated`?
