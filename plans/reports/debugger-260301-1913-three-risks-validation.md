# Debug Validation Report - 3 Fixed Risks
Date: 2026-03-01 19:13 (+07)
Scope: Clamp config numeric bounds, Reverse index ESC M, Runtime cell metrics theo font_size.

## Executive Summary
- Verdict cho 3 risk trong scope: **non-blocker**.
- Không thấy edge-case nghiêm trọng mới gây crash/data-corruption trực tiếp từ 3 fix này.
- Có 1 blocker ngoài scope: full test suite đang đỏ do test expectation cũ.

## Findings
1. Clamp config numeric bounds
- Evidence: `src/config.rs` clamp đầy đủ cho `scrollback_lines`, `font_size`, `sidebar_width` bằng `normalized()` + `clamp_f32`.
- Validation: `cargo test -q normalized_clamps_numeric_values` pass, `cargo test -q normalized_handles_non_finite_values` pass.
- Residual edge-case (non-blocker): nếu parse TOML fail ở 1 field, `load_config()` fallback toàn bộ về default (`unwrap_or_default()`), các field hợp lệ còn lại bị mất.

2. Reverse index ESC M
- Evidence: `ESC M` xử lý ở `esc_dispatch` (`src/session/pty_worker.rs`):
  - `cursor_row == 0` -> `scroll_down(1)`
  - else -> `cursor_row -= 1`
- `scroll_down()` trong `src/session/grid.rs` chèn blank row ở top, drop bottom row, không đẩy vào scrollback (hợp semantics RI mặc định full-screen).
- Validation: `cargo test -q scroll_down_inserts_blank_line_at_top` pass.
- Residual edge-case (non-blocker): chưa support scroll region (`CSI r`), nên RI luôn tác động toàn màn hình.

3. Runtime cell metrics theo font_size
- Evidence:
  - `src/app.rs`: boot lấy `font_size` từ config và gọi `metrics_for_font(font_size)`.
  - `handle_resize()` dùng `cell_width/cell_height` runtime để tính cols/rows.
  - `src/ui/theme.rs`: `metrics_for_font` clamp bounds và xử lý non-finite.
- Validation: grep không còn magic hardcoded `8.4`/`18.9`; `cargo check -q` pass.
- Residual edge-case (non-blocker): session khởi tạo ban đầu vẫn 80x24 trước resize event đầu tiên; nếu môi trường không emit resize sớm thì PTY size ban đầu có thể lệch thực tế.

## Out-of-scope but Important
- Full test suite fail:
  - `cargo test -q` fail tại `session::tests::scrollback_capacity_is_enforced`.
  - Current behavior clamp min scrollback = 100 (`TerminalGrid::new`), nhưng test kỳ vọng `3`.
- Impact: blocker cho CI nếu yêu cầu full test pass.

## Recommendation
1. Giữ verdict scope là non-blocker, có thể tiếp tục.
2. Thêm parser-level test cho `ESC M` (feed sequence qua `vte::Parser`) để khoá semantics.
3. Cân nhắc resize initial session ngay sau first window dimension có thật (hoặc explicit bootstrap resize) để tránh 80x24 tạm thời.
4. Sửa test `scrollback_capacity_is_enforced` theo behavior clamp mới để bỏ blocker CI.

## Unresolved Questions
- Có yêu cầu chính thức support scroll region (`CSI r`) trong phase hiện tại không?
- Có coi fail test ngoài scope là blocker release ngay bây giờ không?
