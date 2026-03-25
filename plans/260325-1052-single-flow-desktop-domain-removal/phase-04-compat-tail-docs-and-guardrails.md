# Context Links

- [Plan](./plan.md)
- [system-architecture.md](../../docs/system-architecture.md)
- [codebase-summary.md](../../docs/codebase-summary.md)
- [project-changelog.md](../../docs/project-changelog.md)

# Overview

- Priority: P2
- Status: pending
- Brief: Khoanh hoặc xoá phần `domain` còn sót trong compat/private zone, cập nhật docs và thêm guard chống regression.

# Key Insights

- `domain` vẫn bám trong host-runtime, lua bridge, config compat.
- Không cần hard-delete hết trong cùng phase nếu chưa có replacement cho compat.

# Requirements

- Docs phản ánh desktop single-flow thật.
- Public path không thể vô tình reintroduce `domain` labels.

# Architecture

- Host-runtime compat zone được ghi rõ là private.
- Nếu `Domain` còn tồn tại, nó không được lộ ra desktop product-facing code.

# Related Code Files

- Modify:
  - `docs/system-architecture.md`
  - `docs/codebase-summary.md`
  - `docs/project-changelog.md`
  - optional guards/tests around command surface

# Implementation Steps

1. Update docs after actual code refactor lands.
2. Add smoke/assertions for command surface not exposing `domain`.
3. Evaluate lua/config compat tail; deprecate or isolate.

# Todo List

- [ ] Update docs
- [ ] Add guardrails/tests
- [ ] Document unresolved compat tail, if any

# Success Criteria

- Architecture docs describe single-flow desktop model, not public domain-based UX.

# Risk Assessment

- Docs drift if phases 01-03 scope changes mid-flight.

# Security Considerations

- No user-facing capability regression hidden by docs cleanup.

# Next Steps

- Optional follow-up: hard-delete `Domain` from host-runtime only when compat tail is gone.
