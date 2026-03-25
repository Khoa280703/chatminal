# Context Links

- [Plan](./plan.md)
- [desktop_host_runtime/mod.rs](../../apps/chatminal-desktop/src/desktop_host_runtime/mod.rs)
- [desktop_termwindow_spawn.rs](../../apps/chatminal-desktop/src/desktop_termwindow_spawn.rs)
- [keyassignment.rs](../../crates/chatminal-config/src/keyassignment.rs)

# Overview

- Priority: P1
- Status: pending
- Brief: Biến desktop spawn path thành một resolver duy nhất, product caller không còn phải mang `SpawnSessionDomain`.

# Key Insights

- `SpawnSessionDomain` đang là chỗ neo lớn nhất của vocabulary `domain` vào public path.
- Không cần xoá enum toàn repo ngay; đủ nếu desktop path không còn consume nó trực tiếp.

# Requirements

- Desktop caller chỉ gọi một API kiểu `spawn_session()` hoặc `spawn_in_current_flow()`.
- Resolver domain-specific bị đẩy xuống private compat layer.

# Architecture

- New target:
  - product/UI -> desktop facade spawn session
  - facade -> internal single-flow resolver
  - resolver -> private host/runtime compat only if needed

# Related Code Files

- Modify:
  - `apps/chatminal-desktop/src/desktop_termwindow_spawn.rs`
  - `apps/chatminal-desktop/src/desktop_host_runtime/mod.rs`
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `crates/chatminal-config/src/keyassignment.rs`

# Implementation Steps

1. Thêm desktop-only spawn entrypoint không mang domain.
2. Chuyển command/product call sites sang entrypoint mới.
3. Giữ adapter chuyển đổi cũ ở private layer cho compat/config.
4. Đánh dấu `SpawnSessionDomain` là compat-only trong desktop path.

# Todo List

- [ ] Introduce single-flow desktop spawn entrypoint
- [ ] Move domain-specific resolution behind private boundary
- [ ] Repoint product call sites
- [ ] Verify desktop checks

# Success Criteria

- Public desktop code không còn phải chọn default/current/domain-name/domain-id.

# Risk Assessment

- Key assignment compatibility có thể kéo thêm scope.

# Security Considerations

- Giữ nguyên validation của spawn command, cwd, shell launch spec.

# Next Steps

- Phase 03: bỏ `domain_id` khỏi adapter/product interactions.
