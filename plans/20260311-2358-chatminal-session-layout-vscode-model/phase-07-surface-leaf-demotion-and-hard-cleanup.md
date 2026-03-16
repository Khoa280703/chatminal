# Phase 07 - Surface Leaf Demotion And Hard Cleanup

## Overview
- Priority: P1
- Status: completed
- Brief: hạ hoặc xoá nốt `surface/leaf` khỏi app/public path sau khi render/actions/persistence đã chuyển xong.

## Requirements
- `surface/leaf` chỉ còn ở runtime-private adapter slice nếu thực sự cần.
- Review lại rename, dead code, bridge, docs.

## Related Code Files
- Modify: `apps/chatminal-desktop/src/chatminal_runtime/*`
- Modify: `crates/chatminal-session-runtime/src/*`
- Modify: `docs/*`

## Success Criteria
- Grep app layer không còn product-facing `split_leaf` / `active_leaf` / `surface_id_for_session` flow.
- Desktop app path dùng `session/view/layout + host_tab/host_leaf` cho public/compat flow; `surface/leaf` chỉ còn ở runtime-private engine slices.
