# Planner Report - 2026-03-05 - Embedded WezTerm GUI Next Step

## Scope (next 1 code batch)
Mục tiêu batch: chuyển `window-wezterm-gui` sang runtime embedded in-process (Linux/macOS), **không spawn `proxy-wezterm-session`**; giữ fallback tạm thời để rollback nhanh trong batch đầu.

## Batch Goal (done condition)
`make window` mở runtime embedded mới, attach 1 session active qua IPC daemon-first, có input/resize/output realtime, không còn lệnh proxy trong launch path.

## Steps (actionable, non-generic)

### Step 1 - Cut entrypoint sang embedded runtime
- Thực hiện:
1. Thêm command handler embedded mới và chuyển `window-wezterm-gui` gọi embedded path mặc định trên Linux/macOS.
2. Đổi proxy path thành command explicit để rollback (`window-wezterm-gui-proxy`), không còn là default.
3. Cập nhật usage/help text và Make target mô tả đúng runtime mới.
- Sửa file:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs`
3. `/home/khoa2807/working-sources/chatminal/Makefile`
- Checkpoint test:
1. `cargo check --manifest-path apps/chatminal-app/Cargo.toml`
2. `CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminal-app/Cargo.toml -- --help` (verify command list)

### Step 2 - Tạo IPC mux-domain embedded (single-session vertical slice)
- Thực hiện:
1. Tạo `ChatminalIpcMuxDomain` map `activate/snapshot/input/resize` trực tiếp qua `ChatminalClient` (không stdin/stdout proxy).
2. Tạo `session_id <-> pane_id` map riêng cho embedded runtime.
3. Tạo event pump background thread nhận `PtyOutput/SessionUpdated/WorkspaceUpdated` và đẩy vào channel cho UI thread.
4. Reuse watermark guard từ `terminal_workspace_binding_runtime` để chặn stale backlog.
- Tạo file:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window_wezterm_gui_embedded/mod.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window_wezterm_gui_embedded/chatminal_ipc_mux_domain.rs`
3. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window_wezterm_gui_embedded/chatminal_event_pump.rs`
4. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window_wezterm_gui_embedded/chatminal_session_pane_map.rs`
- Sửa file:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs` (wire module)
- Checkpoint test:
1. `cargo test --manifest-path apps/chatminal-app/Cargo.toml chatminal_ipc_mux_domain -- --nocapture`
2. `cargo test --manifest-path apps/chatminal-app/Cargo.toml terminal_workspace_binding_runtime -- --nocapture`

### Step 3 - Runtime embedded window loop (Linux/macOS first)
- Thực hiện:
1. Thêm `run_window_wezterm_gui_embedded(...)` dùng window/render loop embedded mới; input gửi thẳng `SessionInputWrite`, resize gửi `SessionResize`, output render từ event pump.
2. Boot session rule: ưu tiên session truyền vào > active session > first session > auto-create "Shell".
3. Không implement multi-tab ở batch này; lock scope 1 pane active để giảm risk.
- Tạo file:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_gui_embedded.rs`
- Sửa file:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs`
2. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/Cargo.toml` (thêm dependency path crates cần cho embedded runtime)
- Checkpoint test:
1. `cargo check --workspace`
2. Linux headless manual: `xvfb-run -a env CHATMINAL_DAEMON_ENDPOINT=/tmp/chatminald.sock cargo run --manifest-path apps/chatminal-app/Cargo.toml -- window-wezterm-gui`

### Step 4 - Smoke + CI gate chuyển sang embedded path
- Thực hiện:
1. Thay smoke launcher mock hiện tại bằng smoke embedded runtime (không assert `proxy-wezterm-session` nữa).
2. Cập nhật workflow Linux smoke dùng script embedded mới.
3. Giữ script proxy cũ tạm thời dưới tên legacy để rollback test trong 1-2 batch đầu.
- Tạo file:
1. `/home/khoa2807/working-sources/chatminal/scripts/smoke/window-wezterm-gui-embedded-smoke.sh`
- Sửa file:
1. `/home/khoa2807/working-sources/chatminal/scripts/smoke/window-wezterm-gui-smoke.sh`
2. `/home/khoa2807/working-sources/chatminal/.github/workflows/rewrite-quality-gates.yml`
3. `/home/khoa2807/working-sources/chatminal/Makefile`
- Checkpoint test:
1. `bash scripts/smoke/window-wezterm-gui-embedded-smoke.sh`
2. `bash -n scripts/smoke/window-wezterm-gui-smoke.sh`
3. CI dry check: chạy local subset `cargo test --manifest-path apps/chatminal-app/Cargo.toml`

### Step 5 - Docs + rollback contract cho batch cutover
- Thực hiện:
1. Cập nhật README + roadmap/changelog: `window-wezterm-gui` là embedded default, proxy là fallback tạm.
2. Ghi rõ env rollback trong batch đầu (ví dụ `CHATMINAL_WINDOW_BACKEND=proxy|embedded`) và plan xóa fallback ở batch kế tiếp.
- Sửa file:
1. `/home/khoa2807/working-sources/chatminal/README.md`
2. `/home/khoa2807/working-sources/chatminal/docs/development-roadmap.md`
3. `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`
- Checkpoint test:
1. `cargo check --workspace`
2. `make smoke-window`
3. `make fidelity-matrix-smoke` (ít nhất trên Linux trước, macOS chạy CI/manual)

## Biggest Technical Risk + Mitigation
- Risk lớn nhất: **deadlock/jank giữa GUI event loop và IPC blocking calls** khi bỏ proxy (trước đây proxy absorb IO blocking).
- Tác động: freeze input, resize lag, mất output tail khi event burst.
- Giảm rủi ro:
1. Cấm gọi `ChatminalClient::request` trực tiếp trong GUI thread; mọi request/event qua worker + bounded channels.
2. Áp dụng timeout ngắn + retry bounded + queue depth metric (log) cho input/event pump.
3. Reuse watermark/stale guard và drain-after-exit logic từ runtime hiện tại.
4. Giữ fallback proxy command trong 1-2 batch đầu để rollback tức thì nếu deadlock xuất hiện trên macOS.

## Unresolved Questions
1. Batch đầu có chấp nhận lock scope 1-session/1-pane (không tab/multi-pane) để hạ risk không?
2. Với macOS, có cần thêm một smoke manual bắt buộc với WezTerm binary thật ngay batch này hay cho phép defer sang batch tiếp theo?
