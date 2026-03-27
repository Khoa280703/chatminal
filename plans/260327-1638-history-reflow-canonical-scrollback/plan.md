---
title: "History Reflow With Canonical Scrollback"
description: "Tách scrollback persisted khỏi width hiện tại để history resize/reopen đúng như live terminal, không vá UI shell."
status: pending
priority: P1
effort: 2-4 ngày
branch: main
tags: [runtime, store, terminal, history, resize, architecture]
created: 2026-03-27
---

# History Reflow With Canonical Scrollback

## Goal
Làm history persisted reflow đúng khi resize và reopen session. Width chỉ là view/layout concern; scrollback phải là canonical data model độc lập width.

## Root Cause
- Live terminal reflow được vì engine giữ buffer sống và resize trên runtime thật.
- Persisted history hiện được lưu ở `scrollback_chunks.chunk_text` dưới dạng text đã bị hard-wrap theo width tại thời điểm đó.
- Restore path `store.session_snapshot() -> normalize_session_snapshot() -> initial_scrollback` replay lại đúng text đã-wrap, nên engine không thể biết đâu là hard newline thật, đâu là soft wrap cũ.

## Decision
Không vá render shell. Sửa ở runtime/store boundary.

## Phases
- [Phase 01](./phase-01-lock-canonical-scrollback-contract.md): khóa contract scrollback canonical và boundary đọc/ghi.
- [Phase 02](./phase-02-introduce-runtime-canonical-buffer.md): thêm runtime canonical buffer + writer path mới.
- [Phase 03](./phase-03-restore-preview-and-resize-reflow.md): đổi restore/preview/reopen sang render từ canonical buffer theo width hiện tại.
- [Phase 04](./phase-04-store-migration-and-legacy-cutover.md): thêm schema/persistence mới, compat reader, cutover legacy.
- [Phase 05](./phase-05-validation-and-non-goals.md): test, manual validation, scope alt-screen/TUI.

## Architecture Guardrails
- `chatminal-runtime` là source of truth cho scrollback semantics.
- `chatminal-store` chỉ persist canonical form, không persist presentation wrap.
- `apps/chatminal-desktop` chỉ truyền target `cols/rows` cho render/restore; không tự fix text history.
- Không đụng UI shell để “vẽ lại history”.
- Không hứa perfect replay cho alt-screen/TUI trong phase đầu.

## Recommended Delivery
- Làm Phase 01-03 như một feature hoàn chỉnh cho shell scrollback thường.
- Giữ compat đọc `scrollback_chunks` cũ, nhưng chỉ ghi schema canonical mới.
- Sau khi validate ổn mới xóa legacy table/path.

## Success Criteria
- Resize cửa sổ: history và live content cùng reflow logic, không còn phần “history bị đứng width cũ”.
- Reopen app/session ở width khác: history render theo width mới.
- Preview/history API không phụ thuộc width lúc persist.
- Không làm regress input, join layout, offline/online semantics.

## Open Questions
- Có cần migration one-shot từ legacy `scrollback_chunks` sang canonical hay chấp nhận legacy-only-read rồi dần thay bằng dữ liệu mới?
- Alt-screen/TUI có persist gì thêm hay explicit non-goal ở release đầu?
- Có cần nén canonical records nếu session output quá lớn?
