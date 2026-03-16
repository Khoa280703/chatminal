# Phase 03 - Desktop Layout Store And Session View Routing

## Overview
- Priority: P0
- Status: completed
- Brief: thêm desktop-side store/router cho active `view_id` và map `view -> session_id`.

## Related Code Files
- Create: `apps/chatminal-desktop/src/chatminal_layout/*`
- Modify: `apps/chatminal-desktop/src/main.rs`
- Modify: `apps/chatminal-desktop/src/chatminal_session_surface.rs`
- Modify: `apps/chatminal-desktop/src/chatminal_runtime/session_host.rs`

## Success Criteria
- Desktop query active view qua layout store thay vì suy từ active leaf.
- Session ordering/focus routing của desktop có thể đọc trực tiếp từ `view_id`/layout snapshot.
