# Phase 04: UI And Desktop Facade Parallel

## Goal
Dọn phần typed-handle/raw-id còn sót trong UI và desktop facade sau khi host adapter đã ổn định.

## Lanes
### Lane 04A: TermWindow And Overlay Cleanup
- Ownership:
  - `apps/chatminal-desktop/src/desktop_termwindow_*`
  - `apps/chatminal-desktop/src/termwindow/*`
  - `apps/chatminal-desktop/src/overlay/*`
- Scope:
  - giảm raw `pane_id() as u64` ở UI/internal boundary đã scope
  - chuyển các helper UI sang typed handle/runtime id nơi không ảnh hưởng render logic

### Lane 04B: Desktop Facade Cleanup
- Ownership:
  - `apps/chatminal-desktop/src/chatminal_runtime/mod.rs`
  - `apps/chatminal-desktop/src/frontend.rs`
  - `apps/chatminal-desktop/src/desktop_commands.rs`
  - `apps/chatminal-desktop/src/desktop_spawn.rs`
- Scope:
  - dọn API chain/facade thừa
  - ép các facade còn sót dùng typed host/runtime contract mới

## Parallel Safety
- 04A và 04B không đụng cùng file.

## Gate
- `cargo check -p chatminal-desktop`
- desktop tests xanh
