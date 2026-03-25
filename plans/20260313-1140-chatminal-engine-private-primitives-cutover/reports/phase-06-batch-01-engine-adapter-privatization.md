## Phase 06 Batch 01

Status: completed
Date: 2026-03-13

### Scope shipped
- Siết `desktop_host_runtime/*` xuống `pub(crate)` hoặc private cho host adapter types/exports chính.
- Siết `chatminal_runtime/client.rs` và facade re-exports xuống desktop-private scope.
- Xóa wrapper/helper chết không còn giá trị sau cutover.
- Khẳng định `desktop_host_runtime` là private adapter zone duy nhất còn biết host primitives ở desktop app path.

### Files changed
- `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/engine_runtime_adapter.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_host.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/spawn_target.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/session_pane.rs`
- `apps/chatminal-desktop/src/desktop_host_runtime/pane.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
- `apps/chatminal-desktop/src/chatminal_runtime/client.rs`

### Gates
- `cargo check -p chatminal-desktop`: pass
- `cargo check -p chatminal-lua-bridge`: pass
- `rg -n "pub fn .*host_|pub fn .*pane_|pub fn .*tab_" apps/chatminal-desktop/src/desktop_host_runtime apps/chatminal-desktop/src/chatminal_runtime`: zero matches
- `rg -n "^pub (struct|fn|const|type|mod)" apps/chatminal-desktop/src/desktop_host_runtime`: zero matches

### Outcome
- Public desktop path không còn public host helper surface dư thừa.
- Host primitives bị giới hạn lại đúng private adapter boundary.
