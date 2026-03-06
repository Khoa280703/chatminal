# Development Roadmap

Last updated: 2026-03-06

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
16. Hard-cut WezTerm direct runtime dependency (2026-03-06):
   - refactored terminal core boundary to use internal `chatminal-terminal-core` (vt100 parser)
   - removed direct wezterm-window linkage from client
   - established internal terminal state as single source of truth
17. Non-blocking session creation in native UI (2026-03-06):
   - fixed window creation timeout under daemon load
   - improved create-session timeouts for better UX
18. macOS native window input recovery (2026-03-06):
   - giữ daemon IPC ổn định hơn trên Unix socket macOS khi client reconnect/create session
   - tắt terminal IME sync trên macOS để ASCII typing không rơi vào preedit sau ký tự đầu tiên
19. Restore vendored WezTerm GUI as default window runtime (2026-03-06):
   - khôi phục `window-wezterm-gui` + `proxy-wezterm-session`
   - `make window` quay lại launcher WezTerm GUI, giữ `CHATMINAL_WINDOW_BACKEND=legacy` làm fallback
20. Adopt Chatminal-owned WezTerm GUI package (2026-03-06):
   - thêm `apps/chatminal-wezterm-gui` vào root workspace để ownership binary nằm ở Chatminal
   - launcher không còn `cargo run --manifest-path third_party/wezterm/...`; build/chạy package first-party trong `target/` của Chatminal
   - bootstrap vendored native deps chuyển sang script repo-owned `scripts/bootstrap-wezterm-vendor-deps.sh`
21. Continue first-party WezTerm extraction (2026-03-06):
   - copy GUI source entry/assets cần dùng sang `apps/chatminal-wezterm-gui`
   - bóc thêm các crate nhẹ `filedescriptor`, `lfucache`, `ratelim`, `wezterm-gui-subcommands` sang `crates/chatminal-*`
   - dời native vendored deps build-path sang `vendor/wezterm-deps`
22. Continue helper/API extraction wave (2026-03-06):
   - bóc thêm `luahelper`, `tabout`, `termwiz-funcs`, `url-funcs`, `window-funcs` sang `crates/chatminal-*`
   - remap `chatminal-wezterm-gui` và `third_party/wezterm` sang các package first-party tương ứng
23. Continue lua-api extraction around `env-bootstrap` (2026-03-06):
   - bóc thêm `battery`, `color-funcs`, `filesystem`, `logging`, `plugin`, `procinfo-funcs`, `serde-funcs`, `share-data`, `spawn-funcs`, `ssh-funcs`, `time-funcs`
   - remap root workspace và `third_party/wezterm` để giảm thêm phụ thuộc vào `third_party/wezterm/lua-api-crates/*`
24. Continue first-party extraction wave around bootstrap/version/codec (2026-03-06):
   - bóc thêm `base91`, `procinfo`, `wezterm-version`, `mux-lua`, `codec`, `env-bootstrap` sang `crates/chatminal-*`
   - `chatminal-termwiz-funcs` không còn `include_bytes!` theo relative path cũ; terminfo `xterm-256color` được mang sang data của crate first-party
   - `cargo fmt --all` pass; `cargo check -p chatminal-wezterm-gui` chỉ còn blocker native host Linux `xcb-util.pc`
   - `cargo test --manifest-path apps/chatminal-app/Cargo.toml` và `apps/chatminald/Cargo.toml` tiếp tục PASS
25. Continue extraction of lightweight foundation crates (2026-03-06):
   - bóc thêm `async_ossl`, `wezterm-config-derive`, `wezterm-dynamic-derive` sang `crates/chatminal-*`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party cho ba crate nền/proc-macro này
   - re-verify `cargo check -p chatminal-wezterm-gui`; graph compile đi qua batch mới và vẫn chỉ dừng ở native host blocker `xcb-util.pc`
   - `cargo test --manifest-path apps/chatminal-app/Cargo.toml` và `apps/chatminald/Cargo.toml` vẫn PASS
26. Continue extraction of parser/layout foundation crates (2026-03-06):
   - bóc thêm `bintree`, `vtparse`, `wezterm-bidi` sang `crates/chatminal-*`
   - giữ tương thích import crate cũ bằng `lib.name` cho `bintree`, `vtparse`, `wezterm_bidi` dù `package.name` đã đổi sang first-party
   - `cargo test -p chatminal-bintree`, `cargo test -p chatminal-vtparse`, `cargo test -p chatminal-wezterm-bidi` đều PASS
   - `cargo check -p chatminal-wezterm-gui` compile đi qua batch mới và vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
   - `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
27. Continue extraction of color/char/input foundation crates (2026-03-06):
   - bóc thêm `wezterm-color-types`, `wezterm-char-props`, `wezterm-input-types` sang `crates/chatminal-*`
   - giữ tương thích import crate cũ bằng `lib.name` cho `wezterm_color_types`, `wezterm_char_props`, `wezterm_input_types`
   - verify package-level PASS:
     - `cargo test -p chatminal-wezterm-char-props`
     - `cargo test -p chatminal-wezterm-input-types`
     - `cargo test -p chatminal-wezterm-color-types --features std,use_serde`
   - `cargo check -p chatminal-wezterm-gui` compile đi qua batch mới và vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
   - `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
28. Continue extraction of terminal data-path crates (2026-03-06):
   - bóc thêm `wezterm-escape-parser` và `wezterm-cell` sang `crates/chatminal-*`
   - giữ tương thích import crate cũ bằng `lib.name` cho `wezterm_escape_parser` và `wezterm_cell`
   - `cargo test -p chatminal-wezterm-escape-parser --features std,use_serde` PASS
   - `cargo test -p chatminal-wezterm-cell --features std,use_serde` PASS sau khi fix feature propagation `wezterm-escape-parser/use_serde`
   - `cargo check -p chatminal-wezterm-gui` compile đi qua batch mới và vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
29. Continue extraction of runtime utility crates (2026-03-06):
   - bóc thêm `wezterm-uds`, `wezterm-toast-notification`, `wezterm-dynamic` sang `crates/chatminal-*`
   - giữ tương thích import crate cũ bằng `lib.name` cho `wezterm_uds`, `wezterm_toast_notification`, `wezterm_dynamic`
   - `cargo test -p chatminal-wezterm-uds`, `cargo test -p chatminal-wezterm-toast-notification`, `cargo test -p chatminal-wezterm-dynamic --features std` đều PASS
   - `cargo check -p chatminal-wezterm-gui` compile đi qua các crate mới và vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
   - `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
30. Continue extraction of PTY/surface runtime crates (2026-03-06):
   - bóc thêm `portable-pty` và `wezterm-surface` sang `crates/chatminal-*`
   - giữ tương thích import crate cũ bằng `lib.name` cho `portable_pty` và `wezterm_surface`
   - `apps/chatminald` chuyển sang dùng `portable-pty.workspace = true` để runtime daemon cũng đi qua crate first-party
   - `chatminal-wezterm-surface` guard image-only test bằng `cfg(feature = "use_image")`; verify package-level dùng `cargo test -p chatminal-wezterm-surface --features std,appdata`
   - `cargo test -p chatminal-portable-pty` PASS; `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
31. Continue extraction of SSH transport crate (2026-03-06):
   - bóc thêm `wezterm-ssh` sang `crates/chatminal-wezterm-ssh`
   - giữ tương thích import crate cũ bằng `lib.name = "wezterm_ssh"`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party
   - `cargo test -p chatminal-wezterm-ssh` PASS (11 unit + 39 integration/e2e trên host hiện tại)
   - `cargo check -p chatminal-wezterm-gui` compile đi qua `chatminal-wezterm-ssh` và vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
   - `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
32. Continue extraction of terminal core crate (2026-03-06):
   - bóc thêm `wezterm-term` sang `crates/chatminal-wezterm-term`
   - giữ tương thích import crate cũ bằng `lib.name = "wezterm_term"`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party
   - mang `termwiz/data/wezterm` vào `crates/chatminal-wezterm-term/data/wezterm` và sửa `include_bytes!` sang path nội bộ crate
   - `cargo check -p chatminal-wezterm-term` PASS
   - `cargo test -p chatminal-wezterm-term` PASS (49 tests)
   - `cargo check -p chatminal-wezterm-gui` compile đi qua `chatminal-wezterm-term` và vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
   - `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
33. Continue extraction of config crate (2026-03-06):
   - bóc thêm `config` sang `crates/chatminal-config`
   - giữ tương thích import crate cũ bằng `lib.name = "config"`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party
   - `cargo check -p chatminal-config` PASS
   - `cargo test -p chatminal-config` PASS (8 tests)
   - `cargo check -p chatminal-wezterm-gui` vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
   - `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
34. Continue extraction of mux crate (2026-03-06):
   - bóc thêm `mux` sang `crates/chatminal-mux`
   - giữ tương thích import crate cũ bằng `lib.name = "mux"`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party
   - bật `chrono/clock` cục bộ trong crate để tương thích với root workspace đang tắt chrono default features
   - `cargo check -p chatminal-mux` PASS
   - `cargo test -p chatminal-mux` PASS (4 tests)
   - `cargo check -p chatminal-wezterm-gui` vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
   - `apps/chatminal-app` và `apps/chatminald` tiếp tục PASS full tests
35. Continue extraction of termwiz crate and workspace cleanup (2026-03-06):
   - bóc thêm `termwiz` sang `crates/chatminal-termwiz`
   - giữ tương thích import crate cũ bằng `lib.name = "termwiz"`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party
   - dọn toàn bộ shadow manifests/subtree copies còn sót trong `crates/`:
     - `chatminal-mux/mux`
     - `chatminal-config/derive`
     - `chatminal-filedescriptor/filedescriptor`
     - `chatminal-ratelim/ratelim`
     - `chatminal-wezterm-client/wezterm-client`
     - `chatminal-wezterm-gui-subcommands/wezterm-gui-subcommands`
   - `find crates -mindepth 3 -name Cargo.toml` đã sạch
   - `cargo check -p chatminal-termwiz` PASS
   - `cargo test -p chatminal-termwiz` PASS (44 tests + 3 doc tests)
   - `cargo test --manifest-path apps/chatminal-app/Cargo.toml` PASS
   - `cargo test --manifest-path apps/chatminald/Cargo.toml` PASS
   - `cargo check -p chatminal-wezterm-gui` vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
36. Continue extraction of mux-server bridge crate (2026-03-06):
   - bóc thêm `wezterm-mux-server-impl` sang `crates/chatminal-wezterm-mux-server-impl`
   - giữ tương thích import crate cũ bằng `lib.name = "wezterm_mux_server_impl"`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party
   - `cargo check -p chatminal-wezterm-mux-server-impl` PASS
   - `cargo test -p chatminal-wezterm-mux-server-impl` PASS (0 tests)
   - sau wave này, root workspace chỉ còn `window` trỏ `third_party/wezterm` trước khi được bóc tiếp ở bước kế
   - `cargo check -p chatminal-wezterm-gui` vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
37. Continue extraction of window crate (2026-03-06):
   - bóc thêm `window` sang `crates/chatminal-window`
   - giữ tương thích import crate cũ bằng `lib.name = "window"`
   - remap root workspace và `third_party/wezterm/Cargo.toml` sang package first-party
   - root workspace không còn path dep trực tiếp nào trỏ `third_party/wezterm/*`
   - `cargo test --manifest-path apps/chatminal-app/Cargo.toml` PASS
   - `cargo test --manifest-path apps/chatminald/Cargo.toml` PASS
   - `cargo check -p chatminal-wezterm-gui` compile đi qua `chatminal-window` và vẫn chỉ dừng ở blocker native host Linux `xcb-util.pc`
38. Enforce third_party reference-only guard (2026-03-06):
   - thêm `scripts/verify-third-party-wezterm-reference-only.sh`
   - thêm shortcut `make verify-third-party-reference-only`
   - đưa guard này vào `make check` để fail nhanh nếu active build/runtime quay lại dùng `third_party/wezterm` trực tiếp
   - verify guard PASS trên host hiện tại
39. Clean compile + asset/guard follow-up after window extraction (2026-03-06):
   - sửa drift API wrapper IME X11 trong `chatminal-window` để downstream `chatminal-wezterm-gui` compile lại sạch
   - chuyển `terminal.png` sang `apps/chatminal-wezterm-gui/assets/icon/terminal.png` và sửa `include_bytes!` sang path first-party
   - tăng độ chặt `verify-third-party-wezterm-reference-only.sh` để bắt cả relative path deps trỏ `third_party/wezterm`
   - `make check-wezterm-gui` và smoke launcher giờ luôn đi qua guard/backend `wezterm-gui`
   - `cargo check -p chatminal-window` và `cargo check -p chatminal-wezterm-gui` PASS trên host Linux hiện tại
40. Final reference-only cleanup for active workspace (2026-03-06):
   - bỏ hẳn dependency dormant `xcb-imdkit` và feature `x11-ime` khỏi root workspace active + `chatminal-window`
   - đơn giản hóa `chatminal-window/src/os/x11/ime.rs` về no-op IME path ổn định cho build mặc định của Chatminal
   - sửa note `deps/freetype/...` cũ trong `chatminal-config` để không còn trỏ layout upstream
   - refresh `Cargo.lock`; `xcb-imdkit` biến mất khỏi workspace active
   - re-verify `cargo check -p chatminal-wezterm-gui`, smoke launcher, `apps/chatminal-app`, `apps/chatminald`, và `wezterm-font` trên lockfile mới đều PASS
41. Linux linker fallback cho runtime-only X11 libs (2026-03-06):
   - `apps/chatminal-wezterm-gui/build.rs` tự tạo local shim cho `libxcb-image.so` và `libxkbcommon-x11.so` khi host Linux chỉ có versioned runtime libs mà thiếu `-dev` symlink
   - gỡ blocker build `chatminal-wezterm-gui` trên host Ubuntu dev hiện tại mà không cần cài package hệ thống
   - verify lại `cargo build -p chatminal-wezterm-gui` và `scripts/smoke/window-wezterm-smoke.sh` đều PASS
   - `make window` trên shell headless hiện tại không còn fail ở compile/link; app dừng đúng ở guard thiếu `DISPLAY`/`WAYLAND_DISPLAY`, nên blocker còn lại chỉ là môi trường desktop

## Active
1. Theo dõi regression qua CI matrix Linux/macOS/Windows sau mỗi batch lớn.
2. Migration sang full `wezterm-gui` runtime (Linux/macOS trước), giữ `chatminald` ownership cho session/profile/history.
   - Đã xong bridge path: `window-wezterm-gui` + `proxy-wezterm-session` + smoke launcher.
   - Đã hard-cut command surface: bỏ `window-wezterm`/`window-legacy`; CLI entrypoint còn `window-wezterm-gui` (shortcut: `make window`).
   - Đã ký canary Linux và rollback drills.
   - Ownership build đã nằm ở `apps/chatminal-wezterm-gui`.
   - Utility/helper/lua-api/bootstrap/version/codec/foundation proc-macro/parser/layout/color/input/data-path/runtime-utility/PTy-surface/window crates đã là first-party; root workspace không còn path dep trực tiếp vào `third_party/wezterm`.
   - `third_party/wezterm` giờ là reference-only thật đối với active workspace; guard đã khóa path deps/runtime shell refs, asset path active đã được chuyển sang first-party, và `xcb-imdkit` đã bị loại khỏi graph active.
   - `wezterm-font` đã là first-party ở mức package graph; `cargo check -p chatminal-wezterm-font` và `cargo test -p chatminal-wezterm-font` đều đã PASS sau khi vá bootstrap libpng (`pngsimd.c`) ở vendor build script.
   - Manual host-specific (macOS smoke/IME matrix) theo external preflight checklist trước promotion cross-platform.
3. Nâng mức native window UX parity cho `chatminal-app` (luồng daily-driver) trong giai đoạn chuyển tiếp.
4. Tăng coverage integration/soak dài hạn (long-run sessions, reconnect churn).
