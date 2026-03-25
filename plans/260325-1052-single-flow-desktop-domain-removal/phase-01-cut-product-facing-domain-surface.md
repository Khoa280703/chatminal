# Context Links

- [Plan](./plan.md)
- [README](../../README.md)
- [System Architecture](../../docs/system-architecture.md)
- [desktop_commands.rs](../../apps/chatminal-desktop/src/desktop_commands.rs)
- [desktop_host_runtime/mod.rs](../../apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)

# Overview

- Priority: P1
- Status: completed
- Brief: Xoá legacy vocabulary cũ khỏi toàn bộ desktop product-facing surface, nhưng chưa đụng private host-runtime structure.

# Key Insights

- User confusion đến từ public/menu vocabulary, không phải từ low-level engine trước tiên.
- legacy vocabulary cũ vẫn còn lộ qua menu động, command labels, attach/detach phrasing, spawn variants.
- Phase này nên giữ runtime behavior cũ nhưng đổi/collapse entry points của desktop path.

# Requirements

- Public-facing desktop path không còn label/menu/action nào dùng vocabulary cũ.
- Không làm hỏng create session, split session, activate session, session navigator.
- Không xoá private launcher engine nếu session navigator còn dùng.

# Architecture

- Product path chỉ nên có: `session`, `session view`, `workspace`, `terminal instance`.
- execution target bị xem là private execution detail; callers desktop product không route theo đó.

# Related Code Files

- Modify:
  - `apps/chatminal-desktop/src/desktop_commands.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/main.rs`

# Implementation Steps

1. Dọn menu/command expansion còn generate từ host execution-target entries.
2. Collapse public command labels `SpawnSession(DefaultTarget|TargetName|TargetId)` về wording single-flow.
3. Chặn `AttachTarget`/`DetachTarget` khỏi desktop product surface nếu không còn intended UX.
4. Giữ internal resolver tạm thời, nhưng không cho desktop product path gọi trực tiếp bằng target-specific variants.

# Todo List

- [x] Remove dynamic execution-target menu entries from desktop shell surface
- [x] Rename or hide remaining public target-based command defs
- [x] Audit right-click / palette / command palette exposure
- [x] Verify `cargo check -p chatminal-desktop`

# Success Criteria

- Không còn item/menu public nào nói về legacy vocabulary cũ.
- Desktop session creation vẫn chạy bình thường bằng một flow duy nhất.

# Risk Assessment

- Config cũ còn emit `AttachTarget`/`SpawnSessionTarget::TargetName`.
- Có thể cần giữ parse compat nhưng map về single-flow behavior.

# Security Considerations

- Không mở rộng surface spawn/attach.
- Không bypass runtime validation hiện có.

# Next Steps

- Phase 02 completed: desktop spawn path đã bị collapse về single-flow.
