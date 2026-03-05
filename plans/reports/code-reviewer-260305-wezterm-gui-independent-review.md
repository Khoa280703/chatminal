## Code Review Summary

### Scope
- Files reviewed:
  - apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs
  - apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs
  - scripts/smoke/window-wezterm-gui-smoke.sh
  - Makefile
  - README.md
- Focus: bug/risk/regression only (independent review)
- Edge-case scouting: done (dependents + data flow + races)
- Validation run:
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml terminal_wezterm_gui_launcher`
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml terminal_wezterm_gui_proxy`
  - `bash scripts/smoke/window-wezterm-gui-smoke.sh`

### Overall Assessment
Batch chạy được ở happy-path, nhưng còn 3 risk quan trọng ảnh hưởng fidelity và first-run UX của default path `make window`.

### Critical Issues
- None found.

### High Priority
1. UTF-8 boundary corruption ở proxy input path.
- Evidence: `stdin.read()` chunked bytes rồi decode từng chunk bằng `String::from_utf8_lossy`.
- File/line: `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:57`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:224`.
- Impact: ký tự Unicode/IME có thể bị mutate khi multibyte char bị tách giữa 2 lần read; input gửi vào PTY sai byte sequence.
- Fix ngay:
  - Giữ buffer carry-over giữa các chunk để chỉ decode UTF-8 complete sequence.
  - Hoặc đổi contract `SessionInputWrite` sang byte-safe payload (vd base64/bytes frame) nếu muốn fidelity tuyệt đối.

2. Output starvation/drop khi input burst lớn.
- Evidence: loop drain toàn bộ `input_rx` trước khi poll event; mỗi chunk input gọi request sync. `ChatminalClient` backlog có giới hạn và drop event khi đầy.
- File/line: `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:76`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:112`, `apps/chatminal-app/src/ipc/client_runtime.rs:50`.
- Impact: UI output có thể lag mạnh hoặc mất `PtyOutput` trong paste/load lớn; regression fidelity so với terminal thật.
- Fix ngay:
  - Interleave input/output (vd process tối đa N input chunks mỗi vòng rồi luôn poll event).
  - Bỏ request-sync-per-chunk, chuyển sang send async/batch theo window nhỏ.

3. First-run workspace rỗng: `make window` fail flow.
- Evidence: `make window` chỉ preflight daemon liveness; không đảm bảo có session. Proxy tự resolve active/first session và fail nếu none.
- File/line: `Makefile:63`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:146`.
- Impact: user mới chạy `make window` có thể mở rồi thoát ngay với `no session available in workspace`.
- Fix ngay:
  - Trong target `window`, auto-create session nếu workspace rỗng.
  - Hoặc launcher/proxy fallback tạo session default khi không có session.

### Medium Priority
1. `Ctrl-]` bị hard-reserve làm exit key, không forward được vào app trong terminal.
- File/line: `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:78`, `apps/chatminal-app/src/terminal_wezterm_gui_proxy.rs:220`.
- Impact: conflict với tool cần `Ctrl-]`.
- Recommendation: chuyển sang key khó collision hơn hoặc cho cấu hình env flag.

2. Smoke script thiếu timeout cho `window-wezterm-gui` invocation.
- File/line: `scripts/smoke/window-wezterm-gui-smoke.sh:61`.
- Impact: nếu launcher/proxy bị treo, CI/local smoke có thể treo vô hạn.
- Recommendation: wrap bằng `timeout` tương tự legacy smoke.

### Low Priority
1. WezTerm binary resolver chỉ check `exists()`, không check executable/file.
- File/line: `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:126`.
- Impact: chọn nhầm path không executable -> fail muộn ở spawn, error khó hiểu hơn.
- Recommendation: validate `is_file()` + executable bit (Unix) trước khi accept.

2. Fallback source-build phụ thuộc `cargo` + `third_party/wezterm` tại runtime.
- File/line: `apps/chatminal-app/src/terminal_wezterm_gui_launcher.rs:45`.
- Impact: môi trường đóng gói không có toolchain/source sẽ không launch được fallback.
- Recommendation: log rõ guidance trong error + docs release policy.

### Edge Cases Found by Scout
- Session selection race giữa `WorkspaceLoad` và `SessionActivate` (session có thể bị switch/close bởi client khác trong khoảng này).
- CI gate hiện vẫn smoke legacy path (`window-wezterm-smoke.sh`) nên dễ miss regression của default GUI path.

### Positive Observations
- Launcher/proxy command wiring rõ ràng trong `main.rs` và help text.
- Smoke script mới đã verify arg forwarding + endpoint env forwarding đúng.
- Unit tests cho builder/parser behavior đã có và pass.

### Recommended Actions
1. Fix input decode boundary + interleave loop trong `terminal_wezterm_gui_proxy.rs` (ưu tiên cao nhất).
2. Chặn first-run failure bằng auto-create/fallback session cho `make window`.
3. Thêm timeout vào smoke GUI script; thêm CI gate cho `window-wezterm-gui` path.
4. Harden binary resolver để fail sớm, error rõ.

### Metrics
- Type Coverage: N/A (Rust)
- Test Coverage: not measured in this review
- Linting Issues: not run in this review

### Unresolved Questions
1. Team muốn giữ input contract text-only hay chấp nhận thay đổi protocol sang byte-safe để chốt fidelity?
2. `make window` có yêu cầu bắt buộc auto-create session cho first-run không?
3. CI có chốt chuyển smoke gate mặc định sang `window-wezterm-gui` ngay batch này không?
