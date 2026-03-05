## Code Review Summary

### Scope
- Files: `apps/chatminal-app/src/config.rs`, `apps/chatminal-app/src/main.rs`, `apps/chatminal-app/src/input/*`, `scripts/migration/phase08-wezterm-gui-killswitch-verify.sh`, `Makefile`, `.github/workflows/rewrite-quality-gates.yml`, `plans/260305-1458.../reports/*`
- LOC reviewed: 1812
- Focus: phase04-06 reverify (compatibility, gates, rollout)
- Scout findings incorporated: yes (code-path + CI/script-path)

### Overall Assessment
- Không thấy `critical`.
- Có `high` còn mở, chủ yếu ở rollback/killswitch correctness và input parity.

### Critical Issues
- None.

### High Priority
1. `window-wezterm-gui-proxy` bypass kill-switch backend routing.
   - Evidence: `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:58`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:93`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:249`
   - Impact: đặt `CHATMINAL_WINDOW_BACKEND=legacy` vẫn có đường chạy ép vào WezTerm GUI; rollback contract không còn tuyệt đối.
   - Short fix: bỏ command này khỏi surface public hoặc gate bằng `CHATMINAL_INTERNAL_PROXY=1`; mọi window entrypoint phải qua `run_window_by_backend`.

2. Phase08 legacy verify có false-pass khi timeout (`124`) mà không có readiness proof.
   - Evidence: `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh:142`, `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh:151`
   - Impact: process treo trước khi backend usable vẫn có thể pass gate.
   - Short fix: chỉ chấp nhận `124` nếu có marker positive (log/IPC probe) xác nhận legacy backend đã init thành công; nếu không fail.

### Medium Priority
1. Legacy fallback làm rơi `session_id` arg của `window-wezterm-gui [session_id]`.
   - Evidence: `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs:202`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:253`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/window/native_window_wezterm.rs:36`
   - Impact: semantics command khác nhau giữa backend; test rollback theo session cụ thể không đúng hành vi kỳ vọng.
   - Short fix: parse arg chung cho cả 2 backend (session_id, preview_lines, cols, rows) hoặc bỏ `session_id` khỏi usage nếu legacy không hỗ trợ.

2. GUI AltGr filter chặn rộng mọi `Ctrl+Alt` alnum/punctuation, lệch semantic so với attach path.
   - Evidence: `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/terminal_input_shortcut_filter.rs:57`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/terminal_input_shortcut_filter.rs:63`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/terminal_wezterm_attach_tui.rs:138`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/input/terminal_input_event.rs:160`
   - Impact: một số `Ctrl+Alt+Key` shortcut bị nuốt ở GUI nhưng attach vẫn forward.
   - Short fix: chỉ suppress khi detect pattern AltGr thực (key + text/IME commit tương ứng), còn lại forward chord.

3. Local verify có thể pass dù skip legacy runtime check nếu thiếu `xvfb-run`.
   - Evidence: `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh:12`, `/home/khoa2807/working-sources/chatminal/scripts/migration/phase08-wezterm-gui-killswitch-verify.sh:131`, `/home/khoa2807/working-sources/chatminal/Makefile:123`
   - Impact: dev local thấy “passed” nhưng rollback path thực tế chưa verify.
   - Short fix: trong `make phase08-killswitch-verify` set `CHATMINAL_PHASE08_REQUIRE_LEGACY_HEADLESS=1` mặc định; muốn skip phải explicit opt-out.

4. Gate artifacts cho phase05/06 vẫn đang partial/manual pending.
   - Evidence: `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/fidelity-compare-linux-macos.json:4`, `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/fidelity-compare-linux-macos.json:24`, `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/ime-manual-evidence.md:7`, `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/rollout-checklist.md:24`
   - Impact: chưa đủ bằng chứng để gọi phase04-06 “done” theo release-gate strict interpretation.
   - Short fix: chốt manual evidence + sign-off hoặc cập nhật rõ trạng thái “non-blocking/pending release-only” trong checklist và plan status.

### Low Priority
1. Invalid env value silently fallback về default, dễ che lỗi rollback typo.
   - Evidence: `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs:155`, `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs:162`
   - Impact: nhập sai `CHATMINAL_WINDOW_BACKEND` có thể khiến rollout không chạy theo intent.
   - Short fix: log warning khi parse fail; tùy chọn strict mode để fail-fast trên CI.

### Positive Observations
- Input module tách nhỏ rõ ràng, test unit đầy đủ cho dedupe/mapping.
- CI gate đã thêm phase02 bench + phase03 fidelity + phase08 verify vào Linux path.
- `cargo check` app pass, input unit tests pass.

### Recommended Actions
1. Fix 2 high findings trước khi sign-off phase04-06.
2. Chốt semantics command window (session_id + proxy exposure) để tránh rollback ambiguity.
3. Khóa local gate tránh skip im lặng.
4. Cập nhật plan/checklist trạng thái theo evidence thực tế.

### Metrics
- Type/build gate: `cargo check --manifest-path apps/chatminal-app/Cargo.toml` passed.
- Targeted tests: `cargo test --manifest-path apps/chatminal-app/Cargo.toml input::` passed (17/17).
- Linting: chưa chạy trong vòng review này.

### Unresolved Questions
1. `window-wezterm-gui-proxy` có chủ đích public cho user/operator, hay chỉ intended internal?
2. Với phase08 legacy check, timeout `124` có được coi là đủ bằng chứng thành công không, hay cần readiness marker bắt buộc?
3. macOS IME/manual evidence là release blocker tuyệt đối, hay chỉ blocker cho GA nhưng không blocker cho merge?
