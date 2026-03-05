# Development Roadmap

Last updated: 2026-03-05

## Hoan Tat
1. Tách daemon/client/protocol/store thành các crate/app độc lập.
2. Chuyển source tree sang runtime native-only theo hướng WezTerm core.
3. Dọn compatibility code legacy (`config.toml`, env typo aliases, text ping format).
4. Hoàn tất phase-01 native window foundation (`window-wezterm` + reducer tests + smoke).
5. Hoàn tất phase-02 daemon concurrency/perf:
   - metrics instrumentation (`requests/events/broadcast/drop`)
   - bounded queue + clear-history generation guards
   - RTT/RSS hard-gate script (`scripts/bench/phase02-rtt-memory-gate.sh`)
   - benchmark gate mới nhất pass (`p95=8.688ms`, `p99=13.225ms`, `pass_fail_gate=true`, fail-gate `p95<=50ms`)
6. Hoàn tất phase-03 terminal fidelity/input:
   - input translator + reconnect guards
   - strict matrix smoke (`scripts/fidelity/phase03-fidelity-matrix-smoke.sh`)
7. Hoàn tất phase-04 cross-platform transport:
   - daemon transport modules `transport/{unix,windows}`
   - app transport modules `ipc/transport/{unix,windows,unsupported}`
   - server loop bỏ hard-gate `unix-only`
   - endpoint resolver Windows chuẩn `\\.\pipe\chatminald[-<username>]`
8. Hoàn tất phase-05 quality gates/release dry-run:
   - fidelity/soak/release scripts có JSON report
   - checksum portable Linux/macOS
   - full gate suite pass liên tiếp
9. Hoàn tất batch hardening phase-05 follow-up (2026-03-05):
   - required fidelity cases tập trung control-path (`ctrl-z`, `ctrl-c-burst`, `stress-paste`); các case `alt-backspace/meta-shortcuts-macos` giữ ở dạng informational
   - soak smoke hỗ trợ mode `pr|nightly`, có common report envelope + fallback path
   - IME blur-commit dedupe path ổn định hơn (giảm duplicate risk)
10. Re-verify hậu hardening (2026-03-05):
   - chạy lại full quality gates 2 vòng liên tiếp, toàn bộ PASS
   - final reviewer sign-off: không còn Critical/High
11. Hoàn tất phase-06 rollout/rollback packaging (2026-03-05):
   - cập nhật phase06 checklist theo default `wezterm` + `legacy` kill-switch
   - promote gate cập nhật evidence mới nhất (phase03/phase06/release dry-run)
   - thêm CI nightly soak job Linux (`schedule`) chạy mode `nightly` 2h và upload artifact
12. Hardening soak anti-flake (2026-03-05):
   - soak `pr` mode chạy 2 vòng (1 warmup + 1 evaluated)
   - nightly/PR đều hỗ trợ `warmup_iterations`
   - tách soak stability gate khỏi RTT hard gate bằng `CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE`
13. Đồng bộ phase checklist (2026-03-05):
   - phase04 đã đóng; phase05/06 giữ trạng thái in-progress do còn manual gate
   - automation evidence (phase03/phase06 reports + tests) đã cập nhật
   - promote checklist đồng bộ lại theo trạng thái Linux signed-off + macOS pending
14. Rollback guard mở rộng cho window backend (2026-03-05):
   - thêm env selector `CHATMINAL_WINDOW_BACKEND=wezterm-gui|legacy`
   - thêm script verify `scripts/migration/phase08-wezterm-gui-killswitch-verify.sh`
   - thêm artifact/report cho plan `260305-1458` (compatibility matrix + rollout/windows follow-up)
15. Đóng plan `260305-1458` (2026-03-05):
   - bổ sung module `window_wezterm_gui/chatminal_ipc_mux_domain` + race tests embedded path
   - refactor `proxy-wezterm-session` sang dùng mux-domain module (event/input ordering guard tập trung một chỗ)
   - re-run full gate suite (`check/test/smoke/fidelity/bench/soak/release-dry-run`) PASS
   - toàn bộ checklist plan `260305-1458` đã đóng; phần manual host-specific giữ ở external release preflight checklist

## Active
1. Theo dõi regression qua CI matrix Linux/macOS/Windows sau mỗi batch lớn.
2. Migration sang full `wezterm-gui` runtime (Linux/macOS trước), giữ `chatminald` ownership cho session/profile/history.
   - Đã xong bridge path: `window-wezterm-gui` + `proxy-wezterm-session` + smoke launcher.
   - Đã hard-cut command surface: bỏ `window-wezterm`/`window-legacy`; CLI entrypoint còn `window-wezterm-gui` (shortcut: `make window`).
   - Đã ký canary Linux và rollback drills.
   - Manual host-specific (macOS smoke/IME matrix) theo external preflight checklist trước promotion cross-platform.
3. Nâng mức native window UX parity cho `chatminal-app` (luồng daily-driver) trong giai đoạn chuyển tiếp.
4. Tăng coverage integration/soak dài hạn (long-run sessions, reconnect churn).
