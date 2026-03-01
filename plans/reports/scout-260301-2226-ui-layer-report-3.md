# Scout Report #3 - UI Layer

Date: 2026-03-01
Scope: `src/ui/mod.rs`, `src/ui/sidebar.rs`, `src/ui/terminal_pane.rs`, `src/ui/input_handler.rs`, `src/ui/color_palette.rs`, `src/ui/theme.rs`
Work context: `/home/khoa2807/working-sources/chatminal`

## 1) UI structure map

- `src/ui/mod.rs`
  - Re-export 5 submodules: `color_palette`, `input_handler`, `sidebar`, `terminal_pane`, `theme`.
- `src/ui/sidebar.rs`
  - `sidebar_view(sessions, active_id, sidebar_width) -> Element<Message>`.
  - Render list theo thứ tự `SessionManager` đưa vào, item là `button` full width, chiều cao cố định `SESSION_ITEM_HEIGHT`.
  - Active marker: `●`; inactive marker: `○`.
  - Footer có `+ New Session` và hint `Alt+N New • Alt+W Close`.
- `src/ui/terminal_pane.rs`
  - `terminal_pane_view(...) -> Canvas<TerminalCanvas>`.
  - `TerminalCanvas` giữ snapshot grid (`Option<Arc<TerminalGrid>>`), metrics ô chữ, `scroll_offset`, `generation`.
  - `TerminalCanvasState` giữ `canvas::Cache` + `last_generation` để invalidate cache theo generation.
  - Có unit test cho `build_render_rows`.
- `src/ui/input_handler.rs`
  - `key_to_bytes(key, physical_key, modifiers) -> Vec<u8>`.
  - Convert keyboard event sang bytes PTY (Enter, arrows, function keys, Ctrl combos, Alt prefix...).
  - Có test cho arrow, Ctrl-A, Space, Shift+Tab, Alt+Arrow, F-key.
- `src/ui/color_palette.rs`
  - `cell_color_to_iced(CellColor, is_fg)`.
  - UI layer chỉ xử lý `Default` hoặc `Rgb`.
- `src/ui/theme.rs`
  - Constants: `TERMINAL_BG`, `SIDEBAR_WIDTH`, `SESSION_ITEM_HEIGHT`, `DEFAULT_FONT_SIZE`.
  - `metrics_for_font(font_size)` trả `(cell_width, cell_height)` có clamp.

## 2) Terminal render pipeline (chi tiết)

### 2.1 Event -> invalidation -> redraw

- `Canvas::Program::update`:
  - Nếu `self.generation != state.last_generation`: clear cache ngay (`state.cache.clear()`), set `last_generation` mới.
  - Mouse wheel: convert delta pixel/line -> line count, publish `Message::ScrollTerminal { delta }` và `and_capture()`.
- `AppState` tăng `terminal_generation` ở các mutation ảnh hưởng viewport:
  - select session, close session, terminal update, scroll, create session.
- `draw` dùng `state.cache.draw(...)`, nghĩa là cùng generation thì tái dùng geometry, giảm cost render.

### 2.2 Row windowing

- `visible_rows = floor(bounds.height / cell_height.max(1.0))`.
- `build_render_rows(grid, offset, visible_rows)`:
  - Nếu alt screen hoặc `offset == 0`: lấy từ `grid.active_cells()`.
  - Nếu `offset > 0`: lấy từ `grid.scrollback` trước, thiếu thì ghép thêm từ active grid.
- `AppState::update(Message::ScrollTerminal)` clamp offset trong `[0, scrollback_len]` (alt screen => max 0).

### 2.3 Per-cell draw order

1. Fill canvas background bằng `TERMINAL_BG` (đen).
2. Với mỗi cell trong window:
   - Nếu `cell.bg != Default`: fill rect nền cell.
   - Nếu cursor đang ở row/col đó và `scroll_offset == 0`: vẽ cursor overlay theo style:
     - Block: overlay rect alpha 0.35.
     - Underline: strip dưới đáy cell.
     - Bar: dải dọc mỏng bên trái.
   - Nếu cell có ký tự khác rỗng/space: draw text (bold -> font weight bold; non-bold -> monospace thường).
   - Nếu underline attr bật: vẽ line dưới.

### 2.4 Rendering notes

- Italic state giữ trong model (`CellAttrs.italic`) nhưng renderer chưa đổi font style cho italic.
- Vẽ text có offset nhẹ (`x + 8% cell_width`, căn dọc theo `font_size`) để giảm clipping.
- Clip thủ công bằng check `x >= bounds.width || y >= bounds.height`.

## 3) Input mapping (UI -> PTY)

### 3.1 App-level interception (trong `AppState::handle_event`)

- `Alt+N` -> `Message::NewSession` (không gửi PTY).
- `Alt+W` -> `Message::CloseSession(active)` (không gửi PTY).
- `Shift+PageUp` -> scroll lên `current_rows` dòng.
- `Shift+PageDown` -> scroll xuống `current_rows` dòng.

### 3.2 `key_to_bytes` mapping table

- Enter: `\r`
- Backspace: `0x7f`
- Tab: `\t`
- Shift+Tab: `ESC [ Z`
- Escape: `ESC`
- Arrow keys: `ESC [ A/B/C/D`
- Home/End: `ESC [ H/F`
- Insert/Delete/PageUp/PageDown: `ESC [ 2~/3~/5~/6~`
- F1..F4: `ESC O P/Q/R/S`
- F5..F12: `ESC [15~/17~/18~/19~/20~/21~/23~/24~`
- Space(named): `0x20`
- Character text: UTF-8 bytes gốc (`text.as_bytes()`)

### 3.3 Modifier rules

- Ctrl xử lý trước:
  - `Ctrl+[a-z]` -> `byte & 0x1f`
  - `Ctrl+@` hoặc `Ctrl+Space` -> `0`
  - `Ctrl+[`, `Ctrl+\\`, `Ctrl+]`, `Ctrl+^`, `Ctrl+_`, `Ctrl+?` -> `27..31`, `127`
- Alt: nếu bytes không rỗng thì prepend `ESC`.
- Kết quả: combo Alt không bị app consume sẽ đi vào PTY dạng Meta/ESC-prefix.

## 4) Color/theme metrics

### 4.1 Theme constants

- `TERMINAL_BG = Color::BLACK`
- `SIDEBAR_WIDTH = 240.0`
- `SESSION_ITEM_HEIGHT = 48.0`
- `DEFAULT_FONT_SIZE = 14.0`

### 4.2 Cell metrics formula (`metrics_for_font`)

- Input `font_size` clamp về `[8.0, 48.0]` nếu finite, ngược lại fallback `14.0`.
- `cell_width = max(size * 0.62, 5.0)`
- `cell_height = max(size * 1.2, size + 1.0)`

Giá trị mốc:
- size 8.0 -> width 5.0, height 9.6
- size 14.0 (default) -> width 8.68, height 16.8
- size 48.0 -> width 29.76, height 57.6

### 4.3 Color conversion

- `CellColor::Default`:
  - foreground -> trắng
  - background -> đen
- `CellColor::Rgb(r,g,b)` -> `Color::from_rgb8(r,g,b)`
- Lưu ý: indexed ANSI color đã được resolve upstream ở `pty_worker` qua `wezterm` palette rồi, UI chỉ nhận `Default/Rgb`.

## 5) Findings / risks ở UI layer

1. `src/ui/terminal_pane.rs` đang 243 dòng, vượt guideline file-size 200 dòng.
2. `docs/design-guidelines.md` hiện ghi default cell metrics `8.4 x 18.0`, lệch implementation hiện tại (`8.68 x 16.8`).
3. `docs/design-guidelines.md` nói "Alt combinations should remain reserved for app-level session controls", nhưng code chỉ reserve `Alt+N`, `Alt+W`; Alt key khác vẫn forward PTY.
4. Italic attr có trong model nhưng chưa render font-style italic (doc đã ghi "state retained", nhưng cần nêu rõ là intentionally not rendered).

## 6) Đề xuất update `docs/design-guidelines.md`

Các chỗ nên update trực tiếp:

1. `## Typography and Cell Metrics`
   - Sửa formula và số mặc định theo `theme.rs` hiện tại:
     - ratio width `0.62`, ratio height `1.2`, clamp logic `max(..., size + 1.0)`.
     - default metrics `8.68` và `16.8`.
2. `## Terminal Rendering Rules`
   - Thêm cursor-style behavior cụ thể: block/underline/bar/hidden.
   - Nêu rõ cursor chỉ vẽ khi `scroll_offset == 0`.
   - Nêu rõ italic chưa render style (state-only).
3. `## Input/Interaction Guidelines`
   - Thay câu "Alt combinations reserved..." thành:
     - `Alt+N`, `Alt+W` là app shortcuts;
     - Alt combos còn lại passthrough PTY với ESC prefix.
   - Bổ sung `Shift+PageUp/PageDown` scroll theo viewport rows.
   - Bổ sung wheel mapping line/pixel và use `SCROLL_LINES_PER_TICK`.
4. `## Color Behavior`
   - Làm rõ tầng UI không map indexed trực tiếp; indexed đã resolve trước khi vào UI.
   - Giữ default fg/bg như hiện tại.
5. `## Current Layout Contract`
   - Bổ sung separator dọc 1px giữa sidebar và terminal.
   - Bổ sung footer contract: nút tạo session + shortcut hint.

## 7) Cross-doc drift cần lưu ý (ngoài yêu cầu chính)

- `docs/codebase-summary.md` phần cell metrics đang ghi ratio cũ (`0.6`, `1.35`) và output cũ; nên sync lại để tránh drift tài liệu.

## Unresolved questions

1. Có chủ đích giữ italic "state-only" lâu dài không, hay sẽ render italic style ở phase tới?
2. Có muốn chuẩn hóa hướng scroll wheel (natural vs traditional) thành config option không?
3. Có muốn tách `terminal_pane.rs` thành 2 file (`terminal-canvas-program.rs`, `terminal-render-rows.rs`) để bám rule <200 LOC không?
