---
title: "Fix macOS native window create-session reconnect bug"
description: "Practical plan to stop UI freeze and reconnect errors when creating a session from the macOS native window."
status: pending
priority: P1
effort: 3h
branch: main
tags: [macos, native-window, ipc, session]
created: 2026-03-06
---

# Plan 260306-0921 - macOS native window create-session reconnect fix

## Context
- `+` trên native window gọi `create_session()` tại `apps/chatminal-app/src/window/native_window_wezterm.rs:145` -> `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:49`.
- Request create chạy bằng worker client riêng tại `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:71` và `:503`.
- Khi worker trả success, UI thread dùng lại `main client` để `reload_workspace()` rồi `activate_session()` tại `apps/chatminal-app/src/window/native_window_wezterm_actions.rs:82`.
- `reload_workspace()` gọi `WorkspaceLoad` + hydrate snapshot sync tại `apps/chatminal-app/src/window/native_window_wezterm_controller.rs:89` và `apps/chatminal-app/src/terminal_workspace_binding_runtime.rs:33`.
- Client chính sẽ fail cứng nếu stream đã bị đánh dấu `broken` tại `apps/chatminal-app/src/ipc/client.rs:197`.
- Daemon `SessionCreate` publish `SessionUpdated` + `WorkspaceUpdated` ngay sau spawn runtime tại `apps/chatminald/src/state/request_handler.rs:65`.

## Hypotheses
- H1. Race/retry thiếu ở window main client.
  `WorkspaceUpdated` đánh dấu state `stale`, rồi `update()` auto gọi `reload_workspace()` tại `apps/chatminal-app/src/window/native_window_wezterm.rs:108`. Nhánh này không có reconnect guard. Nếu main client vừa rơi vào `broken`, UI sẽ lặp reload lỗi và tạo cảm giác lag/đứng.
- H2. macOS dễ kích hoạt race hơn do startup shell nặng hơn.
  Daemon mặc định dùng `/bin/zsh` trên macOS tại `apps/chatminald/src/config.rs:159`; Linux mặc định `/bin/bash`. Zsh startup có thể tạo burst output/chậm hơn, khiến window vừa phải xử lý stale event vừa sync reload/activate trên UI thread.
- H3. Đường reload hiện quá nặng cho hot path create-session.
  `bootstrap_workspace_binding_state()` reload full workspace rồi hydrate snapshot từng session tại `apps/chatminal-app/src/terminal_workspace_binding_runtime.rs:42`. Ngay sau create, nhánh này bị gọi ít nhất một lần từ `poll_create_session_result()` và có thể thêm một lần từ stale reload.
- H4. Nếu event pressure cao, daemon/client buffering có thể khuếch đại vấn đề.
  Daemon broadcast drop client khi queue full tại `apps/chatminald/src/state/runtime_lifecycle.rs:115`. Cần verify metrics/log có spike khi repro trên macOS.

## Files to touch
- `apps/chatminal-app/src/window/native_window_wezterm.rs`
  Gate `state.is_stale()` khi đang create session hoặc chuyển sang safe reload helper có reconnect-once.
- `apps/chatminal-app/src/window/native_window_wezterm_controller.rs`
  Dồn `reload_workspace()` vào helper chịu trách nhiệm reconnect, clear stale đúng lúc, tránh loop lỗi liên tục.
- `apps/chatminal-app/src/window/native_window_wezterm_actions.rs`
  Giảm duplicate reload/activate sau create; chỉ dùng một helper `reload/activate with reconnect` thay vì set `last_error` rồi retry rời rạc.
- `apps/chatminal-app/src/terminal_workspace_binding_runtime.rs`
  Nếu cần, giảm reload full sau `WorkspaceUpdated` trong hot path create-session, hoặc bỏ hydrate dư thừa khi session mới đã có `session_id`.
- `apps/chatminal-app/src/ipc/client.rs`
  Chỉ sửa nếu cần expose trạng thái `broken` hoặc thêm reconnect primitive nhỏ; tránh auto-reconnect sâu trong client nếu chưa cần.
- `apps/chatminald/src/state/runtime_lifecycle.rs`
  Chỉ đụng nếu metrics chứng minh queue full/drop client là nguyên nhân thật.

## Implementation outline
1. Thêm một safe helper ở window layer: `reload_workspace_with_reconnect_once()` và dùng cho cả manual reload, stale reload, post-create reload.
2. Khi `poll_create_session_result()` success, tránh gọi full reload hai lần; ưu tiên reconnect trước nếu client đang broken rồi mới reload/activate.
3. Tạm chặn stale auto-reload trong lúc `pending_session_create` còn active, hoặc coalesce thành một reload sau khi create hoàn tất.
4. Instrument log ngắn quanh `event stream error`, `reload workspace failed`, số lần reconnect, và shell path để xác nhận H2/H4 trước khi mở rộng sang daemon.
5. Nếu repro chỉ xảy ra với `/bin/zsh`, giữ fix ở window hot path trước; không đổi default shell nếu chưa có bằng chứng rõ.

## Verify
- Manual macOS:
  `make daemon`
  `make window`
  tạo session bằng `+` liên tiếp 5-10 lần; expected: không freeze đáng kể, không còn `reload workspace failed: daemon stream is in failed state; reconnect is required`.
- Differential check:
  chạy lại trên macOS với `CHATMINAL_DEFAULT_SHELL=/bin/sh` và so với mặc định để xác nhận H2.
- Regression:
  `cargo check --workspace`
  `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
  `cargo test --manifest-path apps/chatminald/Cargo.toml`
- Runtime observation:
  xem log daemon metrics để xác nhận có/không `dropped_clients_full_total` tăng khi repro.

## Unresolved questions
- `broken` của main client trên macOS đang đến từ read disconnect thật, hay từ write/flush fail sau khi UI bị block?
- Session shell startup trên máy repro có `.zshrc`/plugin nào làm burst output lớn bất thường không?
