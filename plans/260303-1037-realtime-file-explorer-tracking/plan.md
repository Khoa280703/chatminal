---
title: "Realtime File Explorer Tracking (Per Session)"
description: "MVP plan to push realtime explorer updates from Rust watcher to Svelte UI with debounce, batching, and safe cleanup."
status: pending
priority: P1
effort: 12h
branch: main
tags: [tauri, rust, svelte, file-explorer, realtime, watcher]
created: 2026-03-03
---

# Realtime Session Explorer Tracking

## Goal
- Explorer giữ root theo từng session như hiện tại.
- Realtime cập nhật tree/file khi thay đổi ngoài app.
- Tránh lag bằng debounce + batch event.
- Cleanup watcher chuẩn khi đổi session/root, đóng session, thoát app.
- Scope MVP: ổn định cross-platform, không làm watcher framework quá phức tạp.

## Phase Checklist
| Phase | Scope | Status | Effort | Detail |
|---|---|---|---|---|
| 01 | Backend watcher + event contract | pending | 4h | [Phase 01](phase-01-backend-watcher-event-contract.md) |
| 02 | Watcher lifecycle + cleanup/guard rails | pending | 3h | [Phase 02](phase-02-watcher-lifecycle-cleanup.md) |
| 03 | Frontend realtime refresh + race control | pending | 3h | [Phase 03](phase-03-frontend-realtime-refresh-flow.md) |
| 04 | Manual QA + release gate + docs sync | pending | 2h | [Phase 04](phase-04-manual-qa-release-gates.md) |

## Files To Modify
- Backend: `/home/khoa2807/working-sources/chatminal/src-tauri/Cargo.toml`
- Backend: `/home/khoa2807/working-sources/chatminal/src-tauri/src/models.rs`
- Backend: `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`
- Frontend: `/home/khoa2807/working-sources/chatminal/frontend/src/lib/types.ts`
- Frontend: `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`
- Docs (post-implementation): `/home/khoa2807/working-sources/chatminal/README.md`
- Docs (post-implementation): `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
- Docs (post-implementation): `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Top Risks + Mitigation
| Risk | Impact | Mitigation |
|---|---|---|
| FS event burst quá lớn | UI lag/flood invoke | Backend debounce 120-250ms, cap changed paths, `full_resync` flag |
| Race khi đổi session/root | stale refresh, sai tree | Event payload có `session_id` + `revision`; frontend drop stale event |
| Watcher leak | tăng CPU/memory theo thời gian | Centralized `start/stop` API, stop trước mỗi rebind, stop trong shutdown |
| Khác biệt backend OS watcher | mất event/không ổn định | Dùng `notify` backend mặc định + fallback polling khi watcher init fail |
| Path/symlink escape | lộ file ngoài root | Giữ canonical root guard hiện có, chỉ emit relative path trong root |

## Manual Test Checklist (MVP)
1. Chọn root cho session A, tạo/sửa/xóa file ngoài app => explorer tự cập nhật.
2. Mở file preview, sửa file bằng editor ngoài app => preview refresh đúng.
3. Burst thay đổi (copy/delete nhiều file) => app vẫn responsive, không spam lỗi.
4. Đổi root session A liên tục => watcher cũ stop, chỉ root mới phát event.
5. Chuyển session A/B liên tục => event không cross session.
6. Close session hoặc delete profile chứa session đang watch => không panic/leak.
7. Quit completely từ tray/menu => watcher thread dừng sạch.
8. Verify Linux + macOS (và Windows nếu CI/manual env có sẵn).

## Unresolved Questions
1. Có cần watch tất cả session có root hay chỉ active session? (MVP đề xuất: active session only)
