---
phase: 04
status: done
priority: medium
effort: medium-large
risk: medium
---

# Phase 04: Config Independence

## Overview
Chatminal owns its config system thay vì mượn WezTerm's `configuration()` singleton. Current closeout scope tập trung vào giảm singleton scatter và loại bỏ dead config surface; merge `chatminal-runtime/config.rs` vào config system chính đã được deferred explicit.

## Key Insights
- WezTerm config = 700+ fields, Lua-driven, global `configuration()` singleton
- Chatminal runtime có `RuntimeConfig` riêng (minimal, chỉ session params)
- GUI đọc trực tiếp `config::configuration()` cho fonts, keys, colors, window
- Sau Phase 03 (Mux killed), config là coupling point lớn nhất còn lại

## Architecture Target

### Before
```
chatminal-config (WezTerm legacy, 700+ fields, Lua)
  ↑ used by: desktop (GUI), host-runtime, lua-bridge

chatminal-runtime/config.rs (Chatminal native, 5 fields)
  ↑ used by: runtime only
```

### After
```
chatminal-config (Chatminal owned, ~200 fields, Lua)
  ├── appearance: fonts, colors, cursor, window
  ├── keybindings: key assignments
  ├── terminal: scrollback, shell, env
  ├── session: profiles, workspaces
  └── removed: SSH, TLS, WSL, domains, multiplexer fields

chatminal-runtime/config.rs → KEPT as independent env-based runtime config (explicit deferred merge)
```

## Steps

## Status Audit (2026-04-02)
Snapshot dưới đây là trạng thái lịch sử trước closeout; kết luận cuối cho done gate hiện nằm ở [phase-05-final-closeout.md](./phase-05-final-closeout.md) và [final-closeout-checklist.md](./final-closeout-checklist.md).

- `Step 1 Audit config field usage`: `done`
  - Lý do: audit đủ để cắt dead fields lớn, xác nhận field count đã xuống 194, và xác định rõ phần nào defer.
- `Step 2 Remove unused config fields`: `partial`
  - Lý do: dead SSH/TLS/WSL/auto-update/runtime-options cleanup đã xong, nhưng chưa có bằng chứng sweep exhaustive cho mọi dead field còn lại.
- `Step 3 Restructure Config into sections`: `deferred`
  - Lý do: churn lớn, payoff thấp ở thời điểm này, plan hiện đã chấp nhận defer.
- `Step 4 Replace global singleton`: `deferred`
  - Lý do: hot PTY/parser path vẫn phụ thuộc `configuration()`; thay toàn phần lúc này risk cao.
- `Step 5 Simplify Lua config loading`: `partial`
  - Lý do: `.chatminal.lua` đã là primary path và `.wezterm` compat đã được dọn mạnh, nhưng chưa thể nói toàn bộ legacy Lua surface đã được giản lược xong.

### 1. Audit config field usage (research)
For each field in `Config` struct (~700 fields):
- Grep desktop app for actual read sites
- Categorize: **used** / **unused** / **Lua-only**
- Target: remove 400+ unused fields

### 2. Remove unused config fields
Fields confirmed dead after Phase 01 cleanup:
- All SSH/TLS/WSL/auto-update fields (already done in Phase 01)
- Domain fields: any remaining `*_domains` config
- Daemon fields: `daemon_options`, `front_end` enum variants for headless

Fields to **move** (NOT delete — still active):
- `mux_output_parser_buffer_size` → `TerminalConfig.output_parser_buffer_size`
- `mux_output_parser_coalesce_delay_ms` → `TerminalConfig.output_parser_coalesce_delay_ms`
- These are used in `parse_buffered_data()` per-read-loop iteration (L138,143,227-228 in host-runtime/lib.rs)

### 3. Restructure Config into sections
```rust
pub struct ChatminalConfig {
    pub appearance: AppearanceConfig,  // fonts, colors, cursor, window chrome
    pub keybindings: KeybindingConfig, // key assignments, leader key
    pub terminal: TerminalConfig,      // scrollback, shell, env, TERM value
    pub session: SessionConfig,        // profiles, workspace defaults (from runtime)
}
```
- Migrate `chatminal-runtime/config.rs` fields into `SessionConfig`
- Delete `chatminal-runtime/config.rs`

### 4. Replace global singleton
- `configuration()` global → `ChatminalConfig` passed as `Arc<ChatminalConfig>`
- TermWindow receives config at construction
- RuntimeHost receives config at construction
- No more `config::configuration()` calls scattered everywhere
- **Note**: `configuration()` is called inside per-session reader threads (`parse_buffered_data` calls it every read loop). Config must be injected into `read_from_pane_pty()` constructor — propagation depth is significant.

### 5. Simplify Lua config loading
- Keep Lua for user config (`.chatminal.lua`)
- Remove WezTerm-specific Lua APIs (domains, multiplexer, SSH)
- Rename config file: `.wezterm.lua` → `.chatminal.lua` (with fallback)

## Verification
```bash
cargo check --workspace
cargo test --workspace --lib --bins --tests
grep -r "configuration()" apps/ crates/ --include="*.rs"  # should be 0 (replaced with passed config)
```

## Risk Mitigation
- Phase 03 must complete first (Mux fields reference Mux types)
- Restructure incrementally: move 1 section at a time
- Keep Lua config backward-compatible during transition
- Feature flag `cfg(feature = "legacy-config")` for fallback

## Success Criteria
- [x] Config struct < 300 fields (from 700+) — now 194 fields (was ~202)
- [x] Runtime-path `configuration()` scatter in closeout scope reduced to config-foundation helpers plus test/comment residuals
- [x] `chatminal-runtime/config.rs` disposition decided explicitly — kept as independent env-based runtime config
- [x] Deferred singleton-replacement work declared explicitly in plan
- [x] `.chatminal.lua` config file supported — already implemented
- [x] All tests pass

## Completed Work (2026-04-02)
- Removed 8 dead config fields: `color_scheme_dirs`, `command_palette_fg_color`, `command_palette_bg_color`, `pane_select_bg_color`, `runtime_options`, `ulimit_nofile`, `ulimit_nproc`, `xim_im_name`
- Deleted dead `runtime_options.rs` module (daemon leftover)
- Hardcoded ulimit defaults (were configurable with no user-facing knob)
- Confirmed `.chatminal.lua` already primary config file
- Confirmed `CHATMINAL_CONFIG_FILE` env var in place
- Confirmed zero `.wezterm` references in config crate
- Reduced desktop-side config singleton scatter:
  - `frontend.rs` now keeps an initial config snapshot and refreshes it from config reload notifications
  - `main.rs` passes `ConfigHandle` into frontend bootstrap instead of re-reading global config there
  - `main.rs` now routes the root startup/bootstrap snapshots through `current_config_handle()` instead of duplicating direct singleton reads
  - `selection.rs` now reads the word-boundary set from caller-owned config data
  - `overlay/copy.rs` now renders from the `TermWindow` config snapshot instead of pulling global config on each render pass
  - `colorease.rs` now receives `animation_fps` from caller-owned config at each construction site instead of reading the global singleton while scheduling animation frames
  - `stats.rs` now seeds and refreshes `periodic_stat_logging` through an atomic snapshot instead of polling `configuration()` inside the background logging loop
  - `stats.rs` now routes the remaining root reads through `periodic_stat_logging_secs()` instead of duplicating direct singleton expressions
  - `customglyph.rs` now snapshots block AA policy once per draw path from the glyph-cache font/config snapshot instead of repeating singleton reads across hot render branches
- Reduced host-runtime-side singleton scatter in low-risk flows:
  - `spawn_target.rs` now snapshots config once per `build_command()` flow
  - `tab.rs` now stores `unzoom_on_switch_pane` as constructor-time config on `TabInner`
  - `localpane.rs` now reuses local config snapshots for exit-behavior and stateful-close checks instead of repeating singleton reads inside those flows
  - `lua-bridge/src/lib.rs` now snapshots the fallback root-window size before entering the `with_root_window(...)` closure instead of reading the config singleton from inside that closure
  - `host-runtime/src/lib.rs` now routes root exit/workspace fallback reads through `default_exit_behavior()` and `default_workspace_name()` helper boundaries instead of scattering those reads inline
  - `host-runtime/src/lib.rs` now also centralizes `switch_to_last_active_tab_when_closing_tab()` and `unzoom_on_switch_pane()` helper reads
  - `window.rs` and `tab.rs` now consume those helpers, so constructor-time behavior snapshots no longer pull `configuration()` directly from those files

## Deferred Items
- **Config sub-struct restructure** (Step 3): Organizational churn touching 100+ files with no functional benefit. Config is already clean at 194 fields.
- **`configuration()` singleton replacement** (Step 4): Propagation depth into per-session PTY reader threads makes this extremely high risk. Would require injecting `Arc<Config>` into `read_from_pane_pty()` → `parse_buffered_data()` call chain. Recommend deferring to future focused sprint.
- **RuntimeConfig merge** (partial Step 3): `chatminal-runtime/config.rs` is env-var-based, independent from Lua config. No merge needed.
- These deferred items are explicit `out-of-scope` for the final done gate of `260401-0949-architecture-unification`; they are follow-up cleanup, not blockers for closing the current plan.
- Follow-up destination:
  - [../260403-1800-post-unification-followups/phase-01-config-ownership-completion.md](../260403-1800-post-unification-followups/phase-01-config-ownership-completion.md)

## Audit Reset (2026-04-03)
- Claim `done` trước đó chỉ đúng nếu chấp nhận reduced scope, nhưng reduced scope đó chưa được khóa xong ở plan tổng.
- Code reality hiện tại:
  - grep `configuration(` trong scope sản phẩm đã sạch khỏi hot runtime paths ngoài foundation/test/comment slices
  - nhưng Step 3/4 vẫn chưa được quyết định cuối: làm tiếp trong plan này hay tách hẳn follow-up
- Phase này chỉ được coi là đóng khi [phase-05-final-closeout.md](./phase-05-final-closeout.md) chốt explicit scope decision và sync lại `plan.md` + checklist.

## Closeout-Ready Facts
- Remaining `configuration(` hits hiện audit được ở:
  - `crates/chatminal-config/src/lib.rs`
  - `crates/chatminal-config/src/terminal.rs`
  - `apps/chatminal-desktop/src/shapecache.rs` test code
  - comment/documentation references
- Điều đó có nghĩa: kỹ thuật thì phase này gần xong, nhưng trạng thái plan chưa được phép flip `done` cho tới khi scope decision được ký gửi rõ trong phase 05.

## Closeout Completed (2026-04-03)
- `configuration(` trong scope closeout hiện chỉ còn ở:
  - `crates/chatminal-config/src/lib.rs`
  - `crates/chatminal-config/src/terminal.rs`
  - `apps/chatminal-desktop/src/shapecache.rs` test code
  - comment/documentation references
- `Phase 04 Step 3/4` đã được tách chính thức sang follow-up plan:
  - [phase-01-config-ownership-completion.md](../260403-1800-post-unification-followups/phase-01-config-ownership-completion.md)
- Vì vậy phase này được coi là `done` cho current closeout scope.
