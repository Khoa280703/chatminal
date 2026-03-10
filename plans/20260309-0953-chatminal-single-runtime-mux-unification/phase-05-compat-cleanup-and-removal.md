# Phase 05 - Compatibility Cleanup And Removal

## Context Links
- [plan.md](./plan.md)
- [docs/system-architecture.md](/Users/khoa2807/development/2026/chatminal/docs/system-architecture.md)
- [scripts/smoke/window-desktop-smoke.sh](/Users/khoa2807/development/2026/chatminal/scripts/smoke/window-desktop-smoke.sh)
- [scripts/migration/phase08-chatminal-desktop-killswitch-verify.sh](/Users/khoa2807/development/2026/chatminal/scripts/migration/phase08-chatminal-desktop-killswitch-verify.sh)

## Overview
- Priority: P1
- Status: completed
- Brief: xoá code compatibility không còn dùng, đồng bộ docs/tests/scripts với single-runtime architecture.

## Key Insights
- Nếu không dọn, repo sẽ tiếp tục drift giữa desktop runtime thật và compatibility layers.
- Đây là phase chốt để repo phản ánh đúng triết lý `Chatminal == integrated Chatminal runtime`.

## Requirements
- Không còn dead path rõ ràng trong desktop runtime.
- Docs và scripts không còn mô tả proxy/daemon là đường chạy chính của window.
- Test suite mới chứng minh session switch/profile/sidebar path in-process.

## Architecture
- Giữ lại:
  - `chatminal-runtime`
  - `chatminal-store`
  - `apps/chatminal-chatminal-desktop`
- Optional giữ như compatibility package riêng:
  - `apps/chatminald`
  - `apps/chatminal-app`
  - `crates/chatminal-protocol`
- Nếu giữ compatibility packages, root docs phải đánh dấu rõ `desktop hot path: none of these`.

## Related Code Files
- Modify:
  - `README.md`
  - `docs/system-architecture.md`
  - `docs/codebase-summary.md`
  - `docs/deployment-guide.md`
  - `docs/development-roadmap.md`
  - smoke/migration scripts liên quan
- Delete candidate:
  - `apps/chatminal-app/src/ipc/*` nếu compatibility CLI bị loại luôn
  - `apps/chatminald/src/server.rs`
  - `apps/chatminald/src/transport/*`
  - `crates/chatminal-protocol` nếu không còn nhu cầu remote protocol

## Implementation Steps
1. Chạy audit `rg` cho toàn bộ `proxy-desktop-session`, `chatminald`, `IPC`, `protocol` references.
2. Xoá code compatibility không còn nằm trong luồng chạy đã chốt.
3. Viết test/smoke mới cho in-process runtime path.
4. Đồng bộ docs kiến trúc, changelog, roadmap, release checklist.

## Todo List
- [ ] Audit dead code sau cutover
- [ ] Dọn scripts/docs
- [ ] Bổ sung tests cho direct runtime path
- [ ] Chốt package nào còn giữ như compatibility

## Success Criteria
- Repo mô tả đúng kiến trúc mới, không còn drift.
- `make window`/smoke/tests chạy trên single-runtime path.
- Không còn reference runtime bắt buộc tới proxy/daemon trên desktop.

## Risk Assessment
- Risk: xoá sớm compatibility path sẽ làm gãy automation cũ.
- Mitigation: chỉ xoá sau khi smoke/test replacement đã xanh.

## Security Considerations
- Nếu giữ compatibility server, phải xác định rõ nó là optional remote surface và review riêng.

## Next Steps
- Sau phase này mới nên tối ưu sâu bytes-path, lock granularity, and persistence batching.
