# Context Links

- [Plan](./plan.md)
- [desktop_host_runtime/mod.rs](../../apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)
- [session_pane.rs](../../apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs)
- [termwindow/mod.rs](../../apps/chatminal-desktop/src/termwindow/mod.rs)

# Overview

- Priority: P1
- Status: pending
- Brief: Gỡ `domain_id` khỏi các tương tác desktop adapter nơi product path không cần biết đến execution target.

# Key Insights

- `pane.domain_id()` và domain lookup đang còn là cầu nối từ host primitives sang desktop shell.
- Không nhất thiết xoá trait `Domain` ngay; chỉ cần product adapter không còn phụ thuộc.

# Requirements

- Desktop render/input path không cần đọc/resolve public pane domain name.
- Session pane metadata không cần domain cho product routing.

# Architecture

- `session_id`, `runtime_id`, `terminal_instance_id` đủ làm desktop-facing identity.
- `domain_id` chỉ còn là host-runtime internal concern.

# Related Code Files

- Modify:
  - `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/termwindow/mod.rs`
  - `crates/chatminal-host-runtime/src/pane.rs`

# Implementation Steps

1. Audit all desktop-facing reads of `domain_id`.
2. Replace with session/runtime identities where possible.
3. Remove public helper APIs that expose domain name/id to desktop product path.
4. Keep any unavoidable host internal usage private.

# Todo List

- [ ] Remove public domain name exposure helpers from desktop path
- [ ] Replace pane domain usage with session/runtime identity
- [ ] Verify no product feature regresses

# Success Criteria

- Desktop product path no longer needs `domain_id` to route/focus/render sessions.

# Risk Assessment

- Overlay/session navigator/legacy helpers may still assume domain access.

# Security Considerations

- Identity mapping must stay deterministic to avoid cross-session misrouting.

# Next Steps

- Phase 04: compat tail cleanup and docs.
