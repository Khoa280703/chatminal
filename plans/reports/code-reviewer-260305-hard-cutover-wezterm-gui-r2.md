# Code Review - hard-cutover wezterm-gui r2

## Scope
- Files reviewed:
  - apps/chatminal-app/src/main.rs
  - apps/chatminal-app/src/config.rs
  - apps/chatminal-app/src/input/mod.rs
  - apps/chatminal-app/src/input/terminal_input_event.rs
  - apps/chatminal-app/src/input/terminal_input_shortcut_filter.rs
  - apps/chatminal-app/src/input/ime_commit_deduper.rs
  - Makefile
  - README.md
  - docs/project-changelog.md
  - docs/development-roadmap.md
  - plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md
  - plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-03-wezterm-gui-linux-macos-window-runtime.md
- Validation run:
  - `cargo check --manifest-path apps/chatminal-app/Cargo.toml` ✅
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml config::tests::resolve_input_pipeline_mode_falls_back_to_wezterm_on_invalid_value -- --nocapture` ✅
  - `cargo test --manifest-path apps/chatminal-app/Cargo.toml input::ime_commit_deduper::tests::clear_resets_seen_entries -- --nocapture` ✅

## Findings

### High
- None.

### Medium
1. Internal proxy command vẫn public trên CLI surface, chưa clean đúng mục tiêu hard-cut user surface/"remove proxy alias".
- Refs: [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:57](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:57), [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:224](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:224), [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs:173](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs:173)

2. Docs drift rollback path: changelog ghi kill-switch wired vào `window-wezterm` (command đã bị remove ở cutover hiện tại), dễ gây hiểu sai khi vận hành.
- Refs: [/home/khoa2807/working-sources/chatminal/docs/project-changelog.md:236](/home/khoa2807/working-sources/chatminal/docs/project-changelog.md:236), [/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:56](/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:56)

### Low
1. Phase-03 plan self-contradiction: phần Related Code Files nói defer legacy removal đến phase 06, nhưng Todo đã đánh dấu cutover remove xong ngay phase 03.
- Refs: [/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-03-wezterm-gui-linux-macos-window-runtime.md:45](/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-03-wezterm-gui-linux-macos-window-runtime.md:45), [/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-03-wezterm-gui-linux-macos-window-runtime.md:58](/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-03-wezterm-gui-linux-macos-window-runtime.md:58)

## Unresolved Questions
1. `proxy-wezterm-session` có chủ đích giữ public để debug/manual ops, hay cần internal-only (launcher gọi nội bộ, không public usage)?
