# Phase 01 - Backend Watcher + Event Contract

## Context Links
- Plan: [plan.md](plan.md)
- Next: [phase-02-watcher-lifecycle-cleanup.md](phase-02-watcher-lifecycle-cleanup.md)
- Runtime docs: `/home/khoa2807/working-sources/chatminal/README.md`

## Overview
- Priority: P1
- Status: pending
- Effort: 4h
- Goal: Thêm FS watcher realtime ở Rust backend và emit event contract ổn định cho frontend.

## Key Insights
- Explorer hiện tại chỉ pull (`list_session_explorer_entries`, `read_session_explorer_file`), chưa có push event.
- Service đã có pattern worker thread + emit event (`pty/output`, `pty/exited`) có thể tái dùng.
- Path guard/canonicalization cho explorer đã có, cần giữ nguyên nguyên tắc bảo mật.

## Requirements
- Theo dõi thay đổi filesystem bên dưới root explorer của session.
- Emit event Tauri mới, ví dụ `explorer/fs-changed`.
- Event payload phải chứa `session_id` để frontend lọc đúng session.
- Batch/debounce event để tránh spam khi có burst.

## Architecture
- Dependency:
  - Add `notify` vào `src-tauri/Cargo.toml`.
- Contracts (`models.rs`):
  - `SessionExplorerFsChangedEvent { session_id, root_path, revision, changed_paths, full_resync, ts }`.
- Service (`service.rs`):
  - Thêm watcher runtime state (handle + stop channel + session/root metadata).
  - Thread nhận raw fs events -> normalize relative path trong root -> batch/debounce -> emit.
  - Nếu số path vượt ngưỡng (vd `256`) set `full_resync = true`.

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/Cargo.toml`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/models.rs`
- Modify `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`

## Implementation Steps
1. Khai báo struct event mới ở `models.rs`.
2. Thêm constants debounce/batch (`EXPLORER_EVENT_DEBOUNCE`, `MAX_CHANGED_PATHS_PER_BATCH`).
3. Implement watcher worker: ingest `notify::Event`, lọc path trong root, normalize path.
4. Emit event `explorer/fs-changed` qua `app_handle.emit`.
5. Log warn + degrade gracefully nếu watcher lỗi.

## Todo List
- [ ] Add watcher dependency + event model.
- [ ] Add watcher thread and event batching pipeline.
- [ ] Add safe path normalization before emitting.
- [ ] Add overflow path handling (`full_resync`).

## Success Criteria
- Thay đổi file ngoài app tạo event realtime trong frontend listener.
- Event không leak path ngoài root.
- Burst thay đổi không tạo flood event 1:1 theo raw fs notifications.

## Risk Assessment
- Risk: notify backend khác nhau theo OS, event shape không đồng nhất.
- Mitigation: chuẩn hóa về payload tối giản (changed relative paths + full_resync).

## Security Considerations
- Không emit absolute path cho frontend nếu không cần.
- Reject/skip path không canonicalize hoặc escape root.

## Next Steps
- Gắn watcher lifecycle vào session/root flow để tránh leak (Phase 02).

## Unresolved Questions
- None.
