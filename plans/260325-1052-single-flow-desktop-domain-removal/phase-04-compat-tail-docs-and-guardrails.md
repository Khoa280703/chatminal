# Context Links

- [Plan](./plan.md)
- [system-architecture.md](../../docs/system-architecture.md)
- [codebase-summary.md](../../docs/codebase-summary.md)
- [project-changelog.md](../../docs/project-changelog.md)

# Overview

- Priority: P2
- Status: completed
- Brief: Dọn nốt legacy vocabulary cũ còn sót ở config/docs/compat naming để active product path dùng `target` end-to-end.

# Key Insights

- Sau phase này, active desktop/runtime/config path đã không còn legacy naming cũ.
- `engine_dynamic` không hỗ trợ alias input, nên cleanup config field names là breaking change có chủ đích.
- Các chỗ còn thấy vocabulary cũ sau cùng chỉ là docs lịch sử hoặc thuật ngữ kỹ thuật không liên quan product architecture.

# Requirements

- Docs phản ánh desktop single-flow thật.
- Public path không thể vô tình reintroduce `target` labels.
- Config/runtime vocabulary active path phải thống nhất `target`.

# Architecture

- Host-runtime compat zone được ghi rõ là private.
- Active config/runtime surface dùng `SpawnTarget` / `*_targets`.

# Related Code Files

- Modify:
  - `docs/system-architecture.md`
  - `docs/codebase-summary.md`
  - `docs/project-changelog.md`
  - `crates/chatminal-config/src/config.rs`
  - `crates/chatminal-config/src/lua.rs`
  - `crates/chatminal-engine-gui-subcommands/src/lib.rs`
  - `crates/chatminal-host-runtime/src/spawn_target.rs`

# Implementation Steps

1. Rename config public fields from legacy target-list keys to `*_targets`.
2. Rename Lua helpers and CLI/internal labels còn sót sang `target`.
3. Update docs after actual code refactor lands.
4. Verify targeted crates, rồi chạy full workspace check.

# Todo List

- [x] Rename config public fields/helpers sang `target`
- [x] Update docs
- [x] Verify targeted crates
- [x] Verify `cargo check --workspace`

# Success Criteria

- Architecture docs describe single-flow desktop model, not public legacy execution-target UX.
- Active source path không còn legacy naming cũ cho desktop/runtime/config product flow.

# Risk Assessment

- Breaking config rename: user configs cũ dùng legacy target-list keys hoặc legacy default target key sẽ cần đổi tay.

# Security Considerations

- No user-facing capability regression hidden by docs cleanup.
- Rename chỉ đổi vocabulary/config contract, không đổi execution model một luồng hiện tại.

# Next Steps

- Theo dõi docs lịch sử/report cũ nếu muốn dọn chữ `target` toàn repo, nhưng active source path đã sạch.
