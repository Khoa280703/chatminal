---
title: "File Explorer Per-Session Root"
description: "Require explicit explorer root per session and persist explorer state independent from terminal cwd/profile."
status: pending
priority: P1
effort: 14h
branch: main
tags: [file-explorer, session, tauri, svelte, sqlite]
created: 2026-03-03
---

# File Explorer Per-Session Plan

## Goal
- Bắt buộc chọn root folder cho từng session khi dùng explorer, không có skip.
- Explorer state gắn theo session, không gắn profile.
- Explorer không follow `cwd` terminal.
- Root explorer chỉ đổi khi user chủ động đổi.

## Phase Checklist
| Phase | Scope | Status | Effort | Detail |
|---|---|---|---|---|
| 01 | Backend contracts + path guards | pending | 4h | [Phase 01](phase-01-backend-session-explorer-contract.md) |
| 02 | SQLite persistence + migration | pending | 3h | [Phase 02](phase-02-persistence-session-explorer-state.md) |
| 03 | Frontend UX + per-session state | pending | 5h | [Phase 03](phase-03-frontend-explorer-root-flow.md) |
| 04 | Tests + regressions + docs sync | pending | 2h | [Phase 04](phase-04-test-regression-and-doc-sync.md) |

## Files Expected To Change
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/models.rs`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/main.rs`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/persistence.rs`
- `/home/khoa2807/working-sources/chatminal/frontend/src/lib/types.ts`
- `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`
- `/home/khoa2807/working-sources/chatminal/README.md`
- `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
- `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Release Gates (Done)
1. `cargo test --manifest-path src-tauri/Cargo.toml` pass.
2. `npm --prefix frontend run build` pass.
3. Session mới hoặc session cũ chưa có root: explorer không cho duyệt cho đến khi chọn root hợp lệ.
4. Đổi `cwd` terminal không đổi root explorer.
5. Đổi profile rồi quay lại: root explorer đúng theo từng session.
6. Chỉ lệnh user-driven (`set_session_explorer_root`) mới đổi root.

## Top Risks
- Path traversal/symlink escape nếu validate path không chặt.
- Migration DB cũ thiếu cột gây load workspace lỗi.
- UX dead-end do rule “không skip” nếu state lỗi.

## Risk Mitigation
- Canonicalize + enforce `target.starts_with(root)` cho mọi explorer read.
- Migration additive, default `NULL`, không phá dữ liệu cũ.
- UI luôn có action rõ ràng để set/change root, kèm error message cụ thể.

## Unresolved Questions
1. V1 chọn root dùng native folder picker hay nhập path text + validate backend? (đề xuất: nhập path text để scope nhỏ, ship nhanh)
