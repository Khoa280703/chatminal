---
title: "History Reflow With Canonical Scrollback"
description: "Đổi persisted history sang canonical width-independent model để reopen/resize đúng mà không vá UI shell."
status: completed
priority: P1
effort: 3-5 ngày
branch: main
tags: [runtime, store, terminal, history, resize, architecture]
created: 2026-03-27
---

# History Reflow With Canonical Scrollback

## Goal
Làm persisted history reflow đúng khi resize và reopen session. Width chỉ là view concern; scrollback phải là canonical data độc lập width.

## Root Cause
- Live terminal reflow được vì runtime buffer còn sống và engine resize trực tiếp.
- Persisted history hiện nằm trong `scrollback_chunks.chunk_text` dưới dạng text đã bị hard-wrap theo width cũ.
- Restore path đang replay text đó nguyên xi, nên engine không phân biệt được hard newline thật với soft wrap do width cũ.

## Decision
Không vá render shell. Sửa ở runtime/store boundary.

## Non-Negotiable Design Decisions
- Canonical model không lưu presentation wrap.
- Retention và preview phải dựa trên logical lines, không dựa trên rendered lines theo width hiện tại.
- Restore snapshot và preview snapshot là hai contract khác nhau; không ép một API phục vụ cả hai.
- Rollout phải xử lý mixed session history: một session có thể vừa có legacy rows cũ vừa có canonical rows mới.
- Phase đầu chỉ support shell scrollback thường. Alt-screen/TUI exact replay là non-goal.

## Canonical Model Chosen
Không dùng raw event stream. Không làm terminal recorder đầy đủ.

Model thực dụng được chọn:
- `CommittedLine { seq, ord, ts, text }`
- `OpenFragment { seq, ord, ts, text }`

Semantics:
- `CommittedLine` đại diện cho một logical line đã chốt bằng newline thật.
- `OpenFragment` đại diện cho dòng hiện tại chưa có newline ở cuối, ví dụ prompt/input đang mở.
- Một output chunk có thể materialize ra nhiều records; ordering key chuẩn là `(seq, ord)`, không chỉ `seq`.
- Trước khi persist, output phải đi qua reducer nhỏ để materialize visible text cho các edit semantics tối thiểu cần support trong shell path: `\r`, backspace, erase-in-line và duplicate prompt redraw handling hiện có.
- Không persist private/volatile control sequences.
- Explicit non-goal của release đầu: main-screen multi-line redraw/progress UIs dùng cursor-motion nhiều dòng. Nếu cần fidelity cho nhóm này thì làm phase riêng sau.

## Phases
- [Phase 01](./phase-01-lock-canonical-scrollback-contract.md): completed
- [Phase 02](./phase-02-introduce-runtime-canonical-buffer.md): completed
- [Phase 03](./phase-03-restore-preview-and-resize-reflow.md): completed
- [Phase 04](./phase-04-store-migration-and-legacy-cutover.md): completed
- [Phase 05](./phase-05-validation-and-non-goals.md): completed

## Architecture Guardrails
- `chatminal-runtime` là source of truth cho scrollback semantics.
- `chatminal-store` chỉ persist canonical records và legacy compat data trong giai đoạn chuyển tiếp.
- `apps/chatminal-desktop` chỉ truyền `cols/rows` cho restore/render, không tự sửa text history.
- Không sinh duplicate logic preview/restore giữa desktop và runtime.

## Delivery Rule
Không coi `Phase 01-03` là done nếu chưa có persistence canonical thực sự cho reopen path. Muốn đạt goal reopen ở width khác thì schema + writer + unified logical reader phải cùng xong; width reflow do terminal engine xử lý khi hydrate snapshot vào runtime mới.

## Success Criteria
- Resize cửa sổ: history và live content cùng reflow logic.
- Reopen app/session ở width khác: history render theo width mới.
- Mixed session history không mất dòng, không duplicate dòng khi cutover.
- Preview và retention giữ semantics ổn định theo logical lines.
- Không regress input, join layout, offline/online semantics.

## Open Questions
- Có cần migration one-shot từ legacy `scrollback_chunks` sang canonical không, hay chỉ `legacy-read + canonical-write` trong một release rồi cleanup ở release sau?
- Có cần nén canonical records nếu session output rất lớn?

## Final Notes
- Implementation của plan đã hoàn tất.
- Rollout state hiện tại: `canonical-write + dual-read`.
- Manual UI sign-off checklist nằm ở [manual-validation-checklist.md](./manual-validation-checklist.md).
