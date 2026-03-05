## Code Review Summary

### Scope
- Files: `apps/chatminal-app/src/main.rs`, `apps/chatminal-app/src/window/native_window_wezterm.rs`, `scripts/migration/phase08-wezterm-gui-killswitch-verify.sh`, `.github/workflows/rewrite-quality-gates.yml`
- LOC reviewed: 823
- Focus: high-findings remediation reverify (phase04-06)
- Scout findings: command routing/env gate, killswitch verification flow, CI gate enforcement

### Overall Assessment
- Không còn `critical/high` trong 4 file focus.
- Hai high issue trước đó đã được xử lý: proxy command không còn public path mặc định; phase08 verify đã có readiness proof + CI ép legacy headless check.

### Critical Issues
- None.

### High Priority
- None.

### Medium Priority
1. Local false-confidence vẫn có thể xảy ra nếu chạy script phase08 trực tiếp mà thiếu `xvfb-run` và không set cờ strict.
- Evidence: `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh:13`, `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh:131`, `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh:139`
- Impact: local run có thể `pass` dù chưa verify runtime legacy thực tế.
- Note: CI Linux đã chặn case này bằng `CHATMINAL_PHASE08_REQUIRE_LEGACY_HEADLESS=1`.

2. Legacy readiness marker được ghi trước khi vào vòng lặp GUI (`run_native`).
- Evidence: `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs:49`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs:58`
- Impact: marker chứng minh init path đã chạy, chưa chứng minh first-frame render; mức rủi ro thấp-trung bình cho semantic “ready”.

### Low Priority
- None.

### Edge Cases Found by Scout
- `proxy-wezterm-session` chỉ chạy khi `CHATMINAL_INTERNAL_PROXY=1` (`main.rs` gate + launcher inject env).
- `window-wezterm-gui` đi qua backend router (`run_window_by_backend`) nên tôn trọng `CHATMINAL_WINDOW_BACKEND=wezterm-gui|legacy`.
- phase08 script xác minh cả proxy argv (`start -- ... proxy-wezterm-session`) và cờ internal proxy trong mock log.
- workflow Linux đã bật strict legacy-headless verify để tránh skip im lặng.

### Positive Observations
- `cargo check --manifest-path apps/chatminal-app/Cargo.toml` pass.
- `bash -n scripts/migration/phase08-wezterm-gui-killswitch-verify.sh` pass.
- `bash scripts/migration/phase08-wezterm-gui-killswitch-verify.sh` pass ở local hiện tại (legacy path skip do thiếu `xvfb-run`, đúng behavior theo script default).

### Recommended Actions
1. Kết luận remediation `critical/high`: đạt cho scope file được yêu cầu.
2. Nếu muốn local gate chặt như CI, set mặc định strict trong wrapper local (`CHATMINAL_PHASE08_REQUIRE_LEGACY_HEADLESS=1`).
3. Nếu muốn readiness signal mạnh hơn, cân nhắc ghi marker sau khi GUI event loop đã khởi tạo frame đầu.

### Metrics
- Type Coverage: N/A (Rust, không dùng tool đo type coverage)
- Test Coverage: N/A (không chạy coverage trong vòng review này)
- Linting Issues: 0 syntax/build issue trong scope kiểm tra (`cargo check`, `bash -n`)

### Unresolved Questions
1. `apps/chatminal-app/src/window/native_window_wezterm.rs` và `scripts/migration/phase08-wezterm-gui-killswitch-verify.sh` hiện chưa vào tracked index (`git ls-files` không thấy); đây có phải trạng thái chủ đích trước khi commit không?
