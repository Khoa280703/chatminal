---
phase: 02
status: pending
priority: medium
effort: medium
risk: low
---

# Phase 02: Rename engine-* → chatminal-terminal-*

## Overview
Rename 18 `chatminal-engine-*` crates → `chatminal-terminal-*` cho naming consistency. Đây là terminal infrastructure, không phải "engine".

## Key Insights
- "engine-" prefix gây confusion — ngụ ý rendering engine, thực tế là terminal primitives
- Rename = Cargo.toml metadata + import paths, không đổi code logic
- Phải rename cả workspace aliases (e.g., `engine-term = { package = "chatminal-engine-term" }`)

## Mapping

| Current | New |
|---------|-----|
| chatminal-engine-term | chatminal-terminal-emulator |
| chatminal-engine-font | chatminal-terminal-font |
| chatminal-engine-surface | chatminal-terminal-surface |
| chatminal-engine-cell | chatminal-terminal-cell |
| chatminal-engine-bidi | chatminal-terminal-bidi |
| chatminal-engine-blob-leases | chatminal-terminal-blob-leases |
| chatminal-engine-char-props | chatminal-terminal-char-props |
| chatminal-engine-color-types | chatminal-terminal-color-types |
| chatminal-engine-config-derive | chatminal-config-derive |
| chatminal-engine-dynamic | chatminal-dynamic |
| chatminal-engine-dynamic-derive | chatminal-dynamic-derive |
| chatminal-engine-escape-parser | chatminal-terminal-escape-parser |
| chatminal-engine-input-types | chatminal-terminal-input-types |
| chatminal-engine-open-url | chatminal-open-url |
| chatminal-engine-gui-subcommands | chatminal-gui-subcommands |
| chatminal-engine-toast-notification | chatminal-toast-notification |
| chatminal-engine-uds | chatminal-uds |
| chatminal-engine-version | chatminal-version |

## Steps

### 1. Rename crate directories
```bash
for each crate: mv crates/chatminal-engine-X crates/chatminal-terminal-X
```

### 2. Update Cargo.toml in each renamed crate
- `[package] name = "chatminal-terminal-X"`
- Internal deps: update path references

### 3. Update workspace root Cargo.toml
- `[workspace.members]`: update paths
- `[workspace.dependencies]`: update package names + aliases

### 4. Update all consumer Cargo.toml files
- Desktop, config, host-runtime, lua-bridge, etc.
- Change `engine-term.workspace = true` → `terminal-emulator.workspace = true`

### 5. Update Rust imports
- `use engine_term::` → `use terminal_emulator::` (hoặc alias giữ nguyên)
- **Strategy**: giữ workspace alias ngắn gọn, e.g. `terminal-emulator = { package = "chatminal-terminal-emulator" }`

### 6. Update author metadata
- Replace `Wez Furlong` → `Chatminal Contributors` trong Cargo.toml
- Keep license unchanged (MIT, same as WezTerm)

## Verification
```bash
cargo check --workspace
cargo test --workspace --lib --bins --tests
grep -r "engine_" crates/ apps/ --include="*.rs" | grep -v "// " | head -20
```

## Risk Mitigation
- Rename 1 crate at a time, verify compilation after each
- Start with leaf crates (no dependents): engine-version, engine-uds, engine-toast-notification
- End with core crates: engine-term, engine-font (most dependents)

## Success Criteria
- [ ] Zero `chatminal-engine-*` crate names in workspace
- [ ] All imports updated
- [ ] `cargo check --workspace` clean
- [ ] Author metadata updated
