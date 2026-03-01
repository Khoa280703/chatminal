# Docs Extraction Report - code-standards + design-guidelines

Date: 2026-03-01
Work context: `/home/khoa2807/working-sources/chatminal`
Files analyzed:
- `docs/code-standards.md`
- `docs/design-guidelines.md`

## 1) `docs/code-standards.md`

### Purpose
Chuẩn hóa nguyên tắc kỹ thuật cho implementation Rust hiện tại (`src/`): boundary module, error handling, concurrency, parsing, UI rendering, security, testing, và quy tắc đồng bộ docs.

### Key sections
- `Principles`: 5 nguyên tắc nền (SRP, bounded channels, typed errors, terminal correctness, testability).
- `Module Structure`: chia layer entry/state/session/UI và trách nhiệm từng layer.
- `Naming and Types`: quy ước naming, enum rõ nghĩa, tránh magic numbers.
- `Error Handling Pattern`: bắt buộc `Result<_, SessionError>` ở session-control path + log recoverable errors.
- `Concurrency and Threading Rules`: quy tắc thread lifecycle, payload bounds, lock discipline.
- `Terminal Parsing Rules`: parser ownership ở `pty_worker`, phạm vi ANSI support, test update.
- `UI Rendering Rules`: snapshot read-only, generation cache invalidation, scroll clamp, session ordering.
- `Security Rules`: ignore broken pipe, validate shell (`/etc/shells` + executable), giới hạn input bytes.
- `Testing Standards`: `cargo test` pass + loại unit tests tối thiểu + regression tests.
- `Documentation Sync Rules`: khi đổi runtime/render thì cập nhật architecture/summary/changelog.

### Areas needing update
1. Bổ sung rule nhất quán với policy repo về file size/module split (<200 LOC) để giảm drift giữa docs chuẩn code và guideline vận hành agent.
2. Mục `Documentation Sync Rules` nên chỉ rõ thêm `docs/design-guidelines.md` khi thay đổi UI behavior (hiện chỉ nêu architecture/summary/changelog).
3. Có thể thêm explicit note cho keyboard contract (app-level shortcut vs PTY passthrough), vì hiện hành vi này quan trọng nhưng chưa có trong code-standards.

## 2) `docs/design-guidelines.md`

### Purpose
Định nghĩa contract UX/UI terminal-first của Chatminal: layout 2 pane, typography/cell metrics, render rules terminal, color behavior, input behavior, accessibility gaps, và roadmap UX.

### Key sections
- `Design Intent`: terminal-first, mật độ cao, keyboard-first.
- `Current Layout Contract`: sidebar + terminal canvas, width mặc định sidebar, terminal nền đen.
- `Typography and Cell Metrics`: font/size/cell metrics mặc định.
- `Session List Behavior`: marker active/inactive và footer action.
- `Terminal Rendering Rules`: bg non-default, cursor khi live bottom, attrs, clipping.
- `Color Behavior`: default fg/bg + indexed/truecolor mapping.
- `Input/Interaction Guidelines`: semantics keyboard, alt policy, scroll behavior.
- `Accessibility and Usability Gaps`: liệt kê thiếu sót hiện tại.
- `Future UX Priorities`: 4 ưu tiên UX tiếp theo.

### Areas needing update (đối chiếu code hiện tại)
1. Cell metrics đang lệch implementation.
   - Docs ghi: width `8.4`, height `18.0`.
   - Code thực tế: `CELL_WIDTH_RATIO=0.62`, `CELL_HEIGHT_RATIO=1.2`, `cell_height=max(size*1.2, size+1.0)` => default size 14.0 là `8.68 x 16.8`.
   - Ref: `src/ui/theme.rs:7-19`.
2. Mô tả color mapping chưa đúng tầng xử lý.
   - Docs ghi indexed colors map ở UI.
   - Thực tế UI (`cell_color_to_iced`) chỉ nhận `CellColor::Default | Rgb`; indexed đã resolve từ tầng parser/pty trước đó.
   - Ref: `src/ui/color_palette.rs:5-10`.
3. Alt-key policy trong docs không khớp runtime.
   - Docs ghi Alt combinations nên reserved cho app-level controls.
   - Runtime chỉ consume `Alt+N`, `Alt+W`; Alt combo khác vẫn passthrough vào PTY qua ESC-prefix.
   - Ref: `src/app.rs:228-239`, `src/ui/input_handler.rs:50-53`.
4. Input section thiếu shortcut scrolling đã có.
   - Runtime hỗ trợ `Shift+PageUp/PageDown` scroll theo `current_rows`.
   - Ref: `src/app.rs:242-253`.
5. Terminal rendering rules nên cụ thể thêm cursor style (Block/Underline/Bar/Hidden) để phản ánh behavior hiện có.

## Unresolved questions
1. Có muốn cập nhật luôn `docs/codebase-summary.md` để đồng bộ lại cell metrics (tránh lệch chéo tài liệu) trong cùng đợt docs này không?
