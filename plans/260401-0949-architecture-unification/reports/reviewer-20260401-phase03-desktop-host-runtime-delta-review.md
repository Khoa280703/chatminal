# Phase 03 desktop host runtime delta review

Scope:
- apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs
- apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs
- wrappers liên quan trong desktop_host_runtime/mod.rs, frontend.rs, termwindow path

Verification:
- `cargo check -p chatminal-desktop` ✅
- `cargo test --manifest-path apps/chatminal-desktop/Cargo.toml -- --test-threads=1` ❌
  - 83 passed, 2 failed

## Findings

1. High — test fixture cho local-first pane registry đang sai, nên 2 test mới fail và thực tế không verify branch local ownership.
- `host_with_registered_session_pane()` chỉ insert vào `session_pane`, không insert vào `panes`.
- `terminal_binding_for_public_id()` / `frontend_resolve_pane_fallback()` đều đọc từ `self.panes` trước.
- Kết quả: test 1 trả `None`; test 2 rơi qua `legacy_frontend_resolve_pane()` rồi panic vì `HostMux::get()` chưa init.
- Refs: `session_host.rs:1544-1565`, `session_host.rs:1106-1115`, `session_host.rs:1340-1349`, `crates/chatminal-host-runtime/src/lib.rs:671-672`.

2. Medium — `frontend_resolve_focused_pane_fallback()` vẫn lấy focus truth hoàn toàn từ legacy Mux, local host registry chỉ rewrite id sau đó.
- Nếu Mux focus bookkeeping lệch/mất trong giai đoạn ownership cutover, method này trả `None` hoặc stale focused pane dù host registry vẫn có pane binding đúng.
- Consumer trực tiếp là notification suppression ở frontend, nên bug sẽ hiện thành suppress/toast sai hoặc focus-dependent UI sai.
- Refs: `session_host.rs:1352-1363`, `frontend.rs:110-123`.
- Đây là risk suy ra từ code path; chưa có targeted repro trong review này.

3. Medium — read-path pane lookup vẫn fallback về global registry không filter workspace/session, làm ownership của local host chưa thật sự kín.
- `terminal_handle_arc()` fallback sang `host_runtime::terminal_by_id()`.
- `resolve_public_pane_fallback()` fallback sang `legacy_resolve_public_pane_fallback()`, mà legacy path đọc global pane registry/Mux scan không check session membership.
- Callers gồm render/layout/event paths; nếu local registry miss trong lúc stale pane còn tồn tại ở global registry, UI có thể render hoặc route event vào pane không còn thuộc desktop snapshot hiện tại.
- Refs: `desktop_host_runtime/mod.rs:590-606`, `desktop_host_runtime/mod.rs:764-772`, `session_host.rs:357-375`, `session_host.rs:1330-1337`, `termwindow/mod.rs:323-326`, `desktop_termwindow_layout_render.rs:140-143`.
- Đây là risk suy ra từ code path; thiếu test khóa hành vi này.

## Missing tests
- Test `resolve_public_pane_fallback()` prefers local registry and does not resurrect stale pane from legacy/global path.
- Test `terminal_handle_arc()` / `terminal_handle_arc_by_public_id()` do not cross-resolve a removed pane still present in global registry.
- Test `frontend_resolve_focused_pane_fallback()` succeeds when local host binding exists but legacy focus state is absent/stale, hoặc explicit test chứng minh legacy focus vẫn là contract bắt buộc.

## Unresolved questions
- Phase 03C có chủ đích giữ Mux focus as source of truth cho focused pane hay đây chỉ là bridge tạm thời?
- Khi local registry miss nhưng global registry hit, contract mong muốn là compat mềm hay phải hard fail để tránh cross-session leakage?
