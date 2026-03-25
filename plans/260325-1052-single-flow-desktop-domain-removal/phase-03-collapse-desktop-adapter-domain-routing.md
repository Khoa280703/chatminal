# Context Links

- [Plan](./plan.md)
- [desktop_host_runtime/mod.rs](../../apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)
- [session_pane.rs](../../apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs)
- [termwindow/mod.rs](../../apps/chatminal-desktop/src/termwindow/mod.rs)

# Overview

- Priority: P1
- Status: completed
- Brief: Gỡ `spawn_target_id` khỏi các tương tác desktop adapter nơi product path không cần biết đến execution target.

# Key Insights

- `pane.spawn_target_id()` và target lookup đang còn là cầu nối từ host primitives sang desktop shell.
- Không nhất thiết xoá trait `SpawnTarget` ngay; chỉ cần product adapter không còn phụ thuộc.

# Requirements

- Desktop render/input path không cần đọc/resolve public pane target name.
- Session pane metadata không cần target cho product routing.

# Architecture

- `session_id`, `runtime_id`, `terminal_instance_id` đủ làm desktop-facing identity.
- `spawn_target_id` chỉ còn là host-runtime internal concern.

# Related Code Files

- Modify:
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/termwindow/mod.rs`
  - `crates/chatminal-host-runtime/src/pane.rs`

# Implementation Steps

1. Audit all desktop-facing reads of `spawn_target_id`.
2. Replace with session/runtime identities where possible.
3. Remove public helper APIs that expose target name/id to desktop product path.
4. Keep any unavoidable host internal usage private.

# Todo List

- [x] Remove public target name exposure helpers from desktop path
- [x] Replace pane target usage with session/runtime identity
- [x] Verify no product feature regresses

# Success Criteria

- Desktop product path no longer needs `spawn_target_id` to route/focus/render sessions.

# Risk Assessment

- Overlay/session navigator/legacy helpers may still assume target access.

# Security Considerations

- Identity mapping must stay deterministic to avoid cross-session misrouting.

# Next Steps

- Phase 04 in progress: docs sync done một phần, guardrails/tests còn lại chưa thêm.
