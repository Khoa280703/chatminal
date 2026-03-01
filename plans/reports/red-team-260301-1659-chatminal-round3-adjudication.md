# Red Team Review Round 3 — Chatminal Plan
**Date:** 2026-03-01 | **Reviewers:** 4 (Security, Failure Mode, Assumption Destroyer, Scope Critic)
**Total raw findings:** 35 | **After dedup:** 15 | **Accepted:** 13 | **Rejected:** 2

---

## Red Team Findings

### Finding 1: `Subscription::run_with` nhận `fn` pointer, closure sẽ không compile — CRITICAL
**Reviewer:** Assumption Destroyer
**Location:** Phase 03, subscription() code block
**Flaw:** API verified: `run_with<D,S>(data: D, builder: fn(&D)->S)` — tham số 2 là **fn pointer**. Closure capture `rx_arc` không thể coerce thành `fn` pointer → compile error.
**Failure scenario:** Toàn bộ Phase 03 không compile, blocking mọi phase sau.
**Disposition:** Accept
**Rationale:** Confirmed bởi web search. Fix: dùng `Subscription::run` (không có data param) hoặc dùng `channel` approach khác; hoặc dùng static fn + `Arc` truyền qua `data`.

---

### Finding 2: `esc_dispatch` sai callback cho `?1049h` — vim/htop hoàn toàn broken — CRITICAL
**Reviewer:** Assumption Destroyer
**Location:** Phase 02, `esc_dispatch` section (Implementation Steps step 2)
**Flaw:** `?1049h` là CSI sequence (`ESC [ ? 1049 h`) → vte gọi `csi_dispatch`. `esc_dispatch` chỉ nhận ESC two-byte sequences (ESC c, ESC M, ...). Plan đặt handler ở sai callback → alternate screen không bao giờ kích hoạt.
**Failure scenario:** User mở vim → không switch sang alternate buffer → thấy bash prompt thay vì vim editor.
**Disposition:** Accept
**Rationale:** CSI vs ESC distinction là chuẩn VT100. Fix: move `?1049h/l` handler vào `csi_dispatch`, match `params` với `1049`.

---

### Finding 3: `blocking_send` deadlock trong close_session — CRITICAL
**Reviewer:** Failure Mode Analyst + Assumption Destroyer (merged)
**Location:** Phase 02, close_session() + Phase 02 Key Insights (bounded channel)
**Flaw:** Reader thread có thể đang block ở `blocking_send()` khi channel full (UI stalled, window minimized). Khi đó `drop(master)` được gọi nhưng reader không reach `read()` tiếp theo để thấy EOF. `reader_handle.join()` block vĩnh viễn → app freeze.
**Failure scenario:** User minimize window → shell burst output → channel full → reader blocked at blocking_send → user close session → join blocks → UI frozen.
**Disposition:** Accept
**Rationale:** Valid race condition. Fix: dùng `try_send` + discard-on-full (latest-wins, acceptable for terminal), hoặc `child.kill()` trước `drop(master)` để force reader EOF via SIGCHLD → slave close. Hoặc join với timeout.

---

### Finding 4: Subscription take-once race khi batch merge — PTY events mất vĩnh viễn — CRITICAL
**Reviewer:** Failure Mode Analyst + Assumption Destroyer (merged)
**Location:** Phase 03 + Phase 05, subscription() / Subscription::batch
**Flaw:** Phase 05 gộp subscription vào `Subscription::batch([pty_sub, event_sub])`. Nếu batch thay đổi identity hash, Iced teardown + recreate. Closure mới gọi `guard.take()` → Option đã None → `log::warn!; return` → toàn bộ PTY events mất vĩnh viễn.
**Failure scenario:** Phase 05 integration: terminal renders blank sau khi add event subscription. Shells chạy background nhưng output không bao giờ hiển thị.
**Disposition:** Accept
**Rationale:** Root cause của nhiều previous findings. Fix: khởi tạo channel bên trong Subscription closure (không cần Arc<Mutex<Option>>), truyền Sender vào SessionManager qua boot(). Subscription luôn có Receiver mới, không cần take-once guard.

---

### Finding 5: Shell path injection via config — CRITICAL
**Reviewer:** Security Adversary
**Location:** Phase 07, Security Considerations
**Flaw:** `std::fs::metadata(path).is_ok()` không ngăn path traversal hay arbitrary binary execution. `~/.config/chatminal/config.toml` có thể bị kiểm soát (shared machine, dotfile injection).
**Failure scenario:** Attacker viết `shell = "/tmp/evil_script"` → app spawn với full PTY và user env.
**Disposition:** Accept
**Rationale:** Valid supply chain risk. Fix: validate shell path against `/etc/shells` whitelist; canonicalize path; reject symlinks pointing outside system dirs.

---

### Finding 6: `vte::Params` opaque — `params[0]` không compile — CRITICAL
**Reviewer:** Assumption Destroyer
**Location:** Phase 02, csi_dispatch SGR parsing
**Flaw:** `csi_dispatch(&mut self, params: &Params, ...)` — `Params` là iterator type, không phải slice. `params[0]` = compile error. Plan mô tả "iterate with cursor index" không chỉ ra API đúng (`params.iter()` → `Subparams`).
**Failure scenario:** Toàn bộ SGR color parsing (bold, colors, truecolor) sẽ không compile hoặc phải được viết lại từ đầu.
**Disposition:** Accept
**Rationale:** Verified. Fix: dùng `let mut iter = params.iter(); let p0 = iter.next().and_then(|s| s.first()).copied().unwrap_or(0);`

---

### Finding 7: `session_grids` memory leak — CRITICAL
**Reviewer:** Failure Mode Analyst
**Location:** Phase 03 (session_grids field) + Phase 07 (CloseSession arm)
**Flaw:** `session_grids: HashMap<SessionId, Arc<TerminalGrid>>` không được cleanup khi session close. Buffered TerminalUpdated messages sau CloseSession insert zombie entries. 50 sessions × 60KB+ = 3MB min, có scrollback → 300MB.
**Disposition:** Accept
**Rationale:** Definite memory leak. Fix: trong CloseSession arm: `state.session_grids.remove(&id)`. Trong TerminalUpdated: guard với `if state.session_manager.contains(id)`.

---

### Finding 8: Cell width 8.4px hardcoded — HiDPI tính sai cols/rows — HIGH
**Reviewer:** Failure Mode Analyst + Assumption Destroyer (merged)
**Location:** Phase 04, step 2 (Font setup)
**Flaw:** Magic number 8.4px không account for display scale factor. Trên HiDPI 2x: cols/rows báo cho PTY gấp đôi → bash wrap sai cột.
**Disposition:** Accept
**Rationale:** Fix: sau FontLoaded, measure actual advance width via `Paragraph`. Guard rendering trên `font_metrics: Option<(f32, f32)>`.

---

### Finding 9: `new_lines_count` không có trong data flow — scroll anchor drift — HIGH
**Reviewer:** Failure Mode Analyst
**Location:** Phase 06, Requirements + step 2
**Flaw:** `SessionEvent::Update` không mang delta lines_added. `app.rs` không biết bao nhiêu dòng mới → không thể anchor scroll offset đúng.
**Disposition:** Accept
**Rationale:** Fix: thêm `lines_added: usize` vào `SessionEvent::Update`. PtyPerformer track `scroll_up()` calls kể từ flush trước.

---

### Finding 10: Cursor OOB sau resize — panic hoặc corrupt render — HIGH
**Reviewer:** Failure Mode Analyst
**Location:** Phase 02, step 1 (grid.rs resize spec)
**Flaw:** `resize()` không spec cursor clamping. Nếu cursor_col > new_cols, tiếp theo `set_cell(row, old_col, ...)` → OOB panic hoặc corrupt.
**Disposition:** Accept
**Rationale:** Fix: mandate `cursor_col = cursor_col.min(new_cols.saturating_sub(1))`, `cursor_row = cursor_row.min(new_rows.saturating_sub(1))` trong resize spec.

---

### Finding 11: Alternate screen `active_cells()` không được spec — draw() render sai buffer — HIGH
**Reviewer:** Failure Mode Analyst
**Location:** Phase 02 todo + Phase 04 draw()
**Flaw:** Phase 02 yêu cầu dual buffer nhưng Phase 04 `draw()` chỉ reference `grid.cells` không có conditional. Không có `active_cells()` accessor được spec.
**Disposition:** Accept
**Rationale:** Fix: Phase 02 phải spec `TerminalGrid::active_cells(&self) -> &Vec<Vec<Cell>>`. Phase 04 phải mandate dùng `grid.active_cells()`.

---

### Finding 12: Keyboard events bị drop khi hover sidebar — canvas ambiguity — HIGH
**Reviewer:** Assumption Destroyer
**Location:** Phase 05, Key Insights
**Flaw:** Phase 05 viết "events arrive via `canvas::Program::update()` **or** `iced::event::listen()`" — ngụ ý equivalence sai. Canvas update chỉ nhận events khi mouse hover TRÊN canvas. Keyboard events khi focus ở sidebar → bị drop.
**Disposition:** Accept
**Rationale:** Fix: clarify rằng keyboard events đến CHỈ qua `iced::event::listen()`. Canvas chỉ nhận mouse events (wheel) khi hover. Xóa "or `canvas::Program::update()`" khỏi Key Insights cho keyboard.

---

### Finding 13: `ansi_colours` dep unused + `color_palette.rs` hardcode 256 values — HIGH
**Reviewer:** Scope Critic + Assumption Destroyer (merged)
**Location:** Phase 01 Cargo.toml + Phase 04 color_palette.rs
**Flaw:** Vừa add dep `ansi_colours` với lý do "không hardcode", vừa hardcode `XTERM_PALETTE: [u32; 256]`. Dead dependency + `cargo clippy -D warnings` sẽ fail.
**Disposition:** Accept
**Rationale:** Fix: xóa `ansi_colours` khỏi Cargo.toml. Giữ `color_palette.rs` với static array.

---

### Finding 14: OSC 52 clipboard write attack surface — HIGH
**Reviewer:** Security Adversary
**Location:** Phase 02 + Phase 04
**Flaw:** Plan không block OSC 52. PTY process có thể write arbitrary data vào host clipboard.
**Disposition:** Reject
**Rationale:** MVP không implement OSC 52 support → no attack surface. Implementer sẽ không code cái không được spec. Over-scope cho MVP security concern.

---

### Finding 15: PTY input no size limit — clipboard paste OOM — HIGH
**Reviewer:** Security Adversary
**Location:** Phase 05 + Phase 02
**Flaw:** `input_tx: Sender<Vec<u8>>` channel bounded(4) giới hạn count, không giới hạn size. 100MB clipboard paste = 400MB spike.
**Disposition:** Accept
**Rationale:** Fix: trong send_input(), cap message size: `bytes.truncate(65536)` với log::warn. Acceptable MVP guard.

---

## Dropped Findings (Medium — 10 dropped)
- resize_all_sessions no debounce (Scope F6)
- Alt+[1-9] scope creep (Scope F3)
- Mutex poisoning silent failure (Security F7)
- Shell inherits full env credentials (Security F4)
- Font download no hash verify (Security F6)
- cargo-deny gold plating (Scope F7)
- SessionStatus::Exited + auto-removal contradiction (Scope F8)
- vte exact pin maintenance burden (Scope F9)
- Scrollbar indicator scope creep (Scope F10)
- Zombie child if killed externally (Failure F9)
- Generation counter 2-step complexity (Scope F4)

---

## Summary Table

| # | Finding | Severity | Disposition |
|---|---------|----------|-------------|
| 1 | run_with fn pointer → won't compile | Critical | Accept |
| 2 | esc_dispatch wrong for ?1049h | Critical | Accept |
| 3 | blocking_send deadlock close_session | Critical | Accept |
| 4 | Subscription take-once race on batch | Critical | Accept |
| 5 | Shell path injection via config | Critical | Accept |
| 6 | vte::Params opaque params[0] won't compile | Critical | Accept |
| 7 | session_grids memory leak | Critical | Accept |
| 8 | Cell width 8.4px HiDPI wrong cols/rows | High | Accept |
| 9 | new_lines_count undefined scroll drift | High | Accept |
| 10 | Cursor OOB after resize | High | Accept |
| 11 | active_cells() not spec'd — draw wrong buffer | High | Accept |
| 12 | Keyboard events drop when hover sidebar | High | Accept |
| 13 | ansi_colours dead dep + color_palette dup | High | Accept |
| 14 | OSC 52 clipboard write | High | Reject |
| 15 | PTY input no size limit paste OOM | High | Accept |

**Accepted: 14 | Rejected: 1 | Dropped medium: 11**
