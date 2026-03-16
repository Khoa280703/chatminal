# Phase 05 - Actions Cutover: Split/Close/Focus/Move/Attach/Clone

## Overview
- Priority: P0
- Status: completed
- Brief: chuyển mọi thao tác người dùng sang level `view/layout/session`.

## Requirements
- `split`: tạo view mới + attach session mới/existing session.
- `close view`: đóng ô, không mặc định kill session trừ khi explicit.
- `close session`: kill runtime thật.
- `clone`: bản đầu tạo session mới từ cùng profile/cwd.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/spawn.rs`
- Modify: `apps/chatminal-desktop/src/termwindow/paneselect.rs`
- Modify: `apps/chatminal-desktop/src/chatminal_session_surface.rs`
- Modify: `crates/chatminal-session-runtime/src/session_engine.rs`

## Success Criteria
- App action path không còn gọi `split_leaf` như public behavior.
- Focus/close path của window-level interactions không còn giả định active tab là source of truth duy nhất.
