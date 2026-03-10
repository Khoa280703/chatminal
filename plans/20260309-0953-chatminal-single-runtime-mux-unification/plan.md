# Plan 20260309-0953 - Chatminal Single-Runtime Mux Unification

## Mục tiêu
- Hợp nhất desktop runtime để `Chatminal == Chatminal GUI runtime`, không còn hot-path `GUI -> proxy -> IPC -> daemon`.
- Đưa session/profile/history/explorer về cùng một runtime in-process cho desktop.
- Biến `mux` thành terminal engine native của Chatminal thay vì lớp hiển thị attach vào daemon session.

## Kiến trúc đích
1. `apps/chatminal-chatminal-desktop` là desktop app chính.
2. `crates/chatminal-runtime` là runtime lõi mới: session/profile/history/explorer/persistence/event bus/metrics.
3. `apps/chatminald` chỉ còn là compatibility host mỏng bọc quanh `chatminal-runtime` trong giai đoạn chuyển tiếp; không còn là hot path desktop.
4. `apps/chatminal-app` bị co lại thành launcher/CLI compatibility; `proxy-desktop-session` bị xoá.

## Phase
- [phase-01-runtime-extraction.md](./phase-01-runtime-extraction.md): completed
- [phase-02-gui-runtime-integration.md](./phase-02-gui-runtime-integration.md): completed
- [phase-03-mux-session-native-switching.md](./phase-03-mux-session-native-switching.md): completed
- [phase-04-desktop-hot-path-cutover.md](./phase-04-desktop-hot-path-cutover.md): completed
- [phase-05-compat-cleanup-and-removal.md](./phase-05-compat-cleanup-and-removal.md): completed

## Files trọng tâm
- Runtime source hiện tại: `apps/chatminald/src/{state.rs,session.rs,metrics.rs,state/*}`
- Desktop hot path hiện tại: `apps/chatminal-chatminal-desktop/src/chatminal_sidebar/*`, `apps/chatminal-chatminal-desktop/src/termwindow/mod.rs`
- Proxy/IPC bridge cần xoá dần: `apps/chatminal-app/src/terminal_chatminal_gui_proxy.rs`, `apps/chatminal-app/src/window_chatminal_gui/*`, `apps/chatminal-app/src/ipc/*`, `crates/chatminal-protocol`

## Definition of done
1. Desktop `make window` chạy hoàn toàn in-process, không spawn `proxy-desktop-session`.
2. Sidebar profile/session bind trực tiếp vào runtime state; không còn client IPC/poll loop riêng.
3. Session switch không tạo process mới; chỉ đổi active runtime/mux binding.
4. `chatminald` nếu còn tồn tại chỉ là compatibility wrapper quanh `chatminal-runtime`.
5. Smoke/check/test mới phản ánh single-runtime path và pass trên macOS/Linux.

## Quyết định cứng
- Không giữ hybrid architecture như đích cuối.
- Không để SQLite/JSON IPC nằm trên hot path render/input.
- Không để `chatminald` giữ ownership chính cho desktop runtime sau khi cutover.
