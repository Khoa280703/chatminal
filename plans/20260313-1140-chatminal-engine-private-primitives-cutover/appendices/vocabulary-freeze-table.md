# Vocabulary Freeze Table

Purpose: chốt từ điển chính thức dùng cho refactor này. Không tự phát minh tên mới ngoài bảng này nếu chưa cập nhật contract.

| Legacy term | Chatminal term | Allowed scope | Notes |
| --- | --- | --- | --- |
| `Mux` | `EngineRegistry` / `SessionExecutionRegistry` | private engine only | Không dùng ở app/UI semantics |
| `Window` | `DesktopWindowBinding` / `SessionWindowBinding` | app + private engine | Tách rõ app binding và host window |
| `Tab` | `SessionRenderTarget` / `SessionView` | app-facing = `SessionRenderTarget` hoặc `SessionView`; private engine = `HostRenderScope` | Không dùng `tab` để chỉ session |
| `Pane` | `TerminalInstance` / `SessionTerminalHandle` | app-facing = terminal instance/handle; private engine = `HostTerminal` | Không dùng `pane` cho product semantics |
| `Surface` | `SessionView` / `SessionGroup` | app-facing forbidden | Chỉ giữ nếu là graphics `wgpu::Surface`/`glium::Surface` |
| `Leaf` | `TerminalInstance` | app-facing forbidden | Chỉ giữ trong session-runtime layout internals |
| `TabBar` | `SessionBar` / `SessionStrip` | desktop UI | `tabbar.rs` sẽ trở thành session bar renderer |
| `CloseTab` | `CloseSessionView` / `CloseSessionGroupEntry` | desktop UI | chọn tên theo action thực tế sau Phase 04 |
| `ActivateTab` | `ActivateSessionView` | desktop UI | gồm cả previous/next view semantics |
| `MoveTab` | `MoveSessionView` | desktop UI | không còn product meaning `tab` |
| `SplitPane` | `SplitSessionGroup` / `ArrangeSessionGroup` | desktop UI + runtime facade | split trên layout/group, không split execution unit |
| `host_tab_id` | `session_render_target_id` hoặc `session_view_id` | no public exposure | host ids chỉ còn private adapter |
| `host_leaf_id` | `terminal_instance_id` | no public exposure | không expose như product id |

## Product Vocabulary Freeze
- `Session`: execution unit
- `SessionView`: một attachment của session vào workspace/layout
- `SessionGroup`: container/group layout cho nhiều session views
- `WorkspaceLayout`: tree của groups + views
- `RenderTarget`: render attachment cho một session view trong window
- `TerminalInstance`: terminal target thực tế dùng cho input/output/render

## Rules
- App/UI/product docs và code mới phải dùng vocabulary Chatminal ở trên.
- Nếu buộc phải giữ legacy term vì compatibility, term đó phải nằm trong translation layer hoặc private adapter, không ở business logic.
- Mọi lệch khỏi bảng này phải cập nhật plan contract trước khi code.
