---
phase: 01
status: pending
priority: medium
effort: large
risk: high
---

# Phase 01: Config Deep Independence

## Overview
Hoàn tất phần `Phase 04 Step 3/4` đã tách khỏi closeout:
- config sub-struct restructure
- propagate config explicit qua runtime/PTY/read-loop thay vì singleton reads

## Why Deferred
- Churn rộng.
- Rủi ro cao ở PTY/parser/session-engine path.
- Không cần cho closeout ownership/typed-boundary hiện tại.

## Success Criteria
- `configuration()` không còn là active runtime dependency ngoài config foundation thật sự.
- Config được truyền explicit qua các constructor/runtime loops cần thiết.
- `phase-04-config-independence.md` của plan gốc không còn phải giữ deferred items.
