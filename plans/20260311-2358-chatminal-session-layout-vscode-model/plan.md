# Chatminal Session Layout VSCode Model

Status: completed
Goal: chuyển product model của desktop từ `session -> surface -> leaf` sang `session + session_view + layout_tree`, để split/gộp là thao tác layout UI chứ không còn là split nội bộ của một session runtime.

## Phases
- Phase 01 - Product Model Freeze And Compatibility Boundary
- Phase 02 - Workspace Layout Core In Session Runtime
- Phase 03 - Desktop Layout Store And Session View Routing
- Phase 04 - TermWindow Render Cutover To Session View Layout
- Phase 05 - Actions Cutover: Split/Close/Focus/Move/Attach/Clone
- Phase 06 - Persistence And Restore For Layout Views
- Phase 07 - Surface Leaf Demotion And Hard Cleanup

## Progress
- Phase 01: completed
- Phase 02: completed
- Phase 03: completed
- Phase 04: completed
- Phase 05: completed
- Phase 06: completed
- Phase 07: completed

## Key Decisions
- `session` là runtime/process/PTY/history thật.
- `session_view` là một ô UI đang attach vào một `session_id`.
- `layout_tree` là cây split ngang/dọc của các `session_view`.
- `surface/leaf` không còn là product model; chỉ là compatibility/runtime-internal layer trong giai đoạn chuyển tiếp.
- Phiên bản đầu của `clone` tạo session mới từ cùng profile/cwd thay vì nhiều view dùng chung một runtime sống.

## Invariants
- Không đụng `third_party/`.
- Không thay terminal parser/render semantics ngoài boundary cần thiết.
- Không đè phần UI user đang sửa; ưu tiên thêm module mới và cắt dần callsite cũ.
- Mỗi phase phải có build/test gate trước khi sang phase sau.

## Done When
- App/public path chỉ còn nói bằng `session_id + view_id + layout_node_id`.
- Split/gộp không còn mutate cây `leaf` của cùng một session như product behavior.
- `surface/leaf` chỉ còn nằm ở adapter/runtime-private slices hoặc bị xoá hẳn.
