# Scout report #2 - app/runtime orchestration

Date: 2026-03-01
Scope: `src/main.rs`, `src/config.rs`, `src/message.rs`, `src/app.rs` (+ đối chiếu `src/session/*` cho session lifecycle)

## 1) Bootstrap/runtime

Evidence:
- `src/main.rs:10-26`
- `src/app.rs:36-76`

Findings:
1. Process bootstrap set SIGPIPE ignore bằng `libc::signal(SIGPIPE, SIG_IGN)` trong `unsafe` block, tránh app chết khi PTY peer đóng pipe.
2. Logger init qua `env_logger::init()` trước Iced runtime.
3. Iced app wiring dùng `AppState::boot`, `AppState::update`, `AppState::view`, attach `subscription`, set `window.min_size = 800x600`.
4. `AppState::boot()` load config, derive `cell_width/cell_height` từ `font_size`, tạo event channel PTY size 64, inject receiver vào static `SESSION_EVENT_RX`.
5. Boot tạo session đầu tiên ngay (`create_new_session()`), rồi enqueue initial resize task từ `window::latest().and_then(window::size)`.

Runtime nuance:
1. `SESSION_EVENT_RX` lưu `Option<Receiver>` trong `OnceLock<Mutex<...>>`; stream lấy receiver bằng `guard.take()` -> single-consumer semantics.
2. `boot()` cho phép replace receiver nếu lock đã tồn tại (`*guard = Some(event_rx)`), tránh giữ receiver stale qua lần boot sau.

## 2) Config/clamp

Evidence:
- `src/config.rs:3-103`
- `src/config.rs:73-87`
- `src/config.rs:114-148` (tests)

Findings:
1. Config surface: `shell`, `scrollback_lines`, `font_size`, `sidebar_width` (all optional).
2. Numeric bounds:
- `scrollback_lines`: `100..=200_000`
- `font_size`: `8.0..=48.0`
- `sidebar_width`: `160.0..=640.0`
3. `normalized()` áp dụng fallback + clamp:
- `scrollback_lines` dùng `usize::clamp`
- float dùng `clamp_f32`, reject `NaN/Infinity`, fallback default rồi clamp.
4. `load_config()` behavior:
- thiếu config dir/file -> `Config::default()`
- TOML parse fail -> `.unwrap_or_default()` => default (silent fallback)
- parse success -> `.map(Config::normalized)`
5. Tests có 2 case chính: clamp upper/lower, non-finite fallback.

Risk/impact:
1. Parse fail currently silent, không log warning; khó debug user config typo.
2. `Config::default()` đã safe nhưng không đi qua `normalized()` ở fallback path; hiện vẫn đúng vì default constants đã nằm trong range.

## 3) Message flow

Evidence:
- `src/message.rs:6-21`
- `src/app.rs:78-156`
- `src/app.rs:192-196`
- `src/app.rs:287-314`

Message contract:
1. PTY/state messages: `TerminalUpdated`, `SessionExited`.
2. UI intent: `SelectSession`, `NewSession`, `CloseSession`, `ScrollTerminal`.
3. Runtime events: `KeyboardEvent`, `WindowResized`.

Flow path:
1. `subscription()` batch 2 streams:
- PTY stream `Subscription::run(pty_event_stream)`
- Global UI event stream `event::listen().map(Message::KeyboardEvent)`
2. `pty_event_stream()` map `SessionEvent::Update` -> `Message::TerminalUpdated`; `SessionEvent::Exited` -> `Message::SessionExited`.
3. `update()` mutate state theo từng message:
- `NewSession` -> tạo session + focus
- `SelectSession` -> set active + reset scroll offset
- `CloseSession` -> close in manager + cleanup maps + choose next active
- `SessionExited` -> convert thành `Task::done(CloseSession(id))`
- `TerminalUpdated` -> update snapshot + scroll offset reconciliation
- `KeyboardEvent` -> parse shortcut/input bytes
- `WindowResized` -> recompute rows/cols + resize all sessions
- `ScrollTerminal` -> clamp viewport offset

Key behavioral details:
1. `TerminalUpdated`: nếu user đang xem scrollback (`offset > 0`) và output mới tới, offset tăng theo `lines_added`, clamp theo `grid.scrollback.len()`. Giữ viewport ổn định khi terminal chạy.
2. Nếu `grid.use_alternate == true`, offset forced về `0`, disable scrollback context cho alternate screen.
3. `terminal_generation` tăng ở mọi mutation ảnh hưởng render -> invalidation trigger cho terminal canvas.

## 4) Lifecycle session

Evidence:
- `src/app.rs:198-215`, `92-115`, `116`, `273-284`
- `src/session/manager.rs:61-136`, `138-161`
- `src/session/pty_worker.rs:282-324`, `186-191`

Lifecycle chain:
1. Create:
- `AppState::create_new_session()` đặt tên `Session {n}`, gọi `SessionManager::create_session(name, cols, rows)`.
- Manager resolve shell, open PTY, spawn child, spawn reader/writer thread, insert vào `IndexMap`.
- App set active id + scroll offset 0 + generation++.
2. Run:
- Reader thread đọc PTY, parse terminal state, gửi `SessionEvent::Update` (try_send).
- Writer thread nhận bytes từ input queue, write+flush vào PTY.
- UI nhận update, render snapshot `Arc<TerminalGrid>`.
3. Resize:
- App tính cols/rows từ window size, sidebar width, cell metrics; enforce min `10x5`.
- Manager resize toàn bộ session qua `master.resize(...)`.
4. Exit/close:
- Reader gặp EOF/error -> gửi `SessionEvent::Exited` qua short-lived async sender thread.
- `Message::SessionExited` -> `Message::CloseSession`.
- Close path: kill child, drop input_tx/master, join reader, wait child, join writer.
- Nếu close active session, app chọn active mới theo vị trí gần nhất (ưu tiên session trước đó, fallback first).

Lifecycle edge cases:
1. Đóng session cuối cùng => `active_session_id = None`; app không auto-create session mới.
2. `create_new_session()` fail chỉ log error; UI có thể ở trạng thái không session nếu boot fail.
3. `next_session_num` chỉ tăng, không reuse khi close session (expected UX, cần docs nêu rõ).

## 5) Điểm đáng chú ý cần phản ánh trong docs

Nên cập nhật/tăng độ rõ ở `docs/system-architecture.md`, `docs/codebase-summary.md`, `docs/project-overview-pdr.md`, `docs/deployment-guide.md`:

1. Ghi rõ `SESSION_EVENT_RX` là single-consumer receiver + cơ chế `guard.take()`; tránh hiểu nhầm có thể attach nhiều PTY streams cùng lúc.
2. Ghi rõ close-selection policy: đóng tab active sẽ chọn session trước đó (nếu có), fallback tab đầu.
3. Ghi rõ behavior khi đóng tab cuối: app không auto spawn session mới.
4. Ghi rõ scrollback reconciliation rule:
- primary screen: offset tăng theo `lines_added` khi user đang cuộn lên
- alternate screen: offset reset 0
5. Ghi rõ startup sequence có bước initial `WindowResized` task từ `window::latest()` sau khi tạo first session.
6. Ghi rõ config parse fail hiện silent fallback về default (không warning log).
7. Trong deployment/troubleshooting docs, thêm symptom “mở app nhưng không có session” khi shell invalid hoặc spawn fail, kèm hướng dẫn kiểm tra log.

## Unresolved questions
1. Có muốn policy “luôn giữ ít nhất 1 session” sau khi user đóng tab cuối không?
2. Có muốn log warning khi parse `config.toml` fail để dễ hỗ trợ user?
3. `SESSION_EVENT_RX` single-consumer hiện đủ cho lifecycle hiện tại; có plan hỗ trợ multi-window/multi-runtime không?
