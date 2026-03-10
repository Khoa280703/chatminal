# Chatminal Desktop Host Identity Cutover

Status: In Progress
Progress: 60%

Mục tiêu:
- bỏ identity sản phẩm `Chatminal GUI` khỏi desktop host hiện tại
- giữ terminal engine internals ổn định để không gãy build/runtime
- chuyển ownership bề mặt desktop về `Chatminal`

Phases:
- Phase 01: rename desktop host surface (`apps/chatminal-chatminal-desktop` -> desktop-facing `chatminal-desktop`) - completed
- Phase 02: rename launcher/commands/scripts/docs về `desktop`/`window` naming - completed
- Phase 03: bóc engine-vs-host ownership trong source tree - pending
- Phase 04: dọn crate/package metadata, bundle metadata, release artifact naming - pending
- Phase 05: re-verify full workspace + docs sync - in progress

Key dependencies:
- `apps/chatminal-desktop`
- `apps/chatminal-app`
- `Makefile`
- `README.md`
- `docs/codebase-summary.md`
- `docs/project-changelog.md`

Batch hiện tại:
- đã đổi desktop host surface + compatibility CLI/env naming
- giữ alias cũ hẹp ở một số entrypoint/env để chuyển tiếp an toàn
- chưa rename engine crates `chatminal-chatminal-*`
- chưa đổi low-level module names nếu chúng còn là engine internals hợp lệ
