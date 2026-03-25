# Context Links

- [Plan](./plan.md)
- [README](../../README.md)
- [System Architecture](../../docs/system-architecture.md)
- [desktop_commands.rs](../../apps/chatminal-desktop/src/desktop_commands.rs)
- [desktop_host_runtime/mod.rs](../../apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)

# Overview

- Priority: P1
- Status: pending
- Brief: Xoá `domain` khỏi toàn bộ desktop product-facing surface, nhưng chưa đụng private host-runtime structure.

# Key Insights

- User confusion đến từ public/menu vocabulary, không phải từ low-level engine trước tiên.
- `domain` hiện còn lộ qua menu động, command labels, attach/detach phrasing, spawn variants.
- Phase này nên giữ runtime behavior cũ nhưng đổi/collapse entry points của desktop path.

# Requirements

- Public-facing desktop path không còn label/menu/action nào dùng chữ `domain`.
- Không làm hỏng create session, split session, activate session, session navigator.
- Không xoá private launcher engine nếu session navigator còn dùng.

# Architecture

- Product path chỉ nên có: `session`, `session view`, `workspace`, `terminal instance`.
- `domain` bị xem là private execution detail; callers desktop product không route theo nó.

# Related Code Files

- Modify:
  - `apps/chatminal-desktop/src/desktop_commands.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_actions_impl.rs`
  - `apps/chatminal-desktop/src/desktop_termwindow_actions_items.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/main.rs`

# Implementation Steps

1. Dọn menu/command expansion còn generate từ `host_domain_menu_entries()`.
2. Collapse public command labels `SpawnSession(DefaultDomain|DomainName|DomainId)` về wording single-flow.
3. Chặn `AttachDomain`/`DetachDomain` khỏi desktop product surface nếu không còn intended UX.
4. Giữ internal resolver tạm thời, nhưng không cho desktop product path gọi trực tiếp bằng domain-specific variants.

# Todo List

- [ ] Remove dynamic domain menu entries from desktop shell surface
- [ ] Rename or hide remaining public domain-based command defs
- [ ] Audit right-click / palette / command palette exposure
- [ ] Verify `cargo check -p chatminal-desktop`

# Success Criteria

- Không còn item/menu public nào nói về `domain`.
- Desktop session creation vẫn chạy bình thường bằng một flow duy nhất.

# Risk Assessment

- Config cũ còn emit `AttachDomain`/`SpawnSessionDomain::DomainName`.
- Có thể cần giữ parse compat nhưng map về single-flow behavior.

# Security Considerations

- Không mở rộng surface spawn/attach.
- Không bypass runtime validation hiện có.

# Next Steps

- Nếu phase này xong sạch, chuyển sang phase 02 để collapse spawn resolution thực sự.
