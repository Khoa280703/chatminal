---
phase: 04
status: pending
priority: medium
effort: medium-large
risk: medium
---

# Phase 04: Config Independence

## Overview
Chatminal owns its config system thay vì mượn WezTerm's `configuration()` singleton. Merge `chatminal-runtime/config.rs` vào config system chính. Xóa dead config fields còn sót.

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

chatminal-runtime/config.rs → DELETED (merged into chatminal-config)
```

## Steps

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
- [ ] Config struct < 300 fields (from 700+)
- [ ] Zero `configuration()` global singleton calls
- [ ] `chatminal-runtime/config.rs` deleted
- [ ] Config passed explicitly via `Arc<ChatminalConfig>`
- [ ] `.chatminal.lua` config file supported
- [ ] All tests pass
