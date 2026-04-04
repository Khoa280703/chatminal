---
phase: 02
status: pending
priority: low
effort: medium
risk: low
---

# Phase 02: Engine Rename Cosmetic

## Overview
Hoàn tất rename/cosmetic sweep `chatminal-engine-* -> chatminal-terminal-*`.

## Why Deferred
- Không đổi behavior.
- Diff rất rộng, làm loãng closeout ownership/runtime hiện tại.
- Không phải blocker cho single-runtime desktop architecture.

## Success Criteria
- Workspace không còn crate name `chatminal-engine-*` trong scope được chọn.
- Import/Cargo metadata/docs đều sync.
- Không tạo regression compile/test.
