---
phase: 02
status: done
priority: medium
effort: medium
risk: low
---

# Phase 02: Rename engine-* → chatminal-terminal-*

## Overview
Rename 18 `chatminal-engine-*` crates → `chatminal-terminal-*` cho naming consistency. Đây là terminal infrastructure, không phải "engine".

## Closeout Decision (2026-04-03)
- Phase này ban đầu được giữ `deferred / out-of-scope` cho closeout của plan `260401-0949-architecture-unification`.
- Sau đó đã được thực hiện và verify qua follow-up plan `260403-1800-post-unification-followups`.
- Kết quả:
  - package/path/docs vocabulary đã chuyển sang `chatminal-terminal-*` / `chatminal-*`
  - `lib.name` và compatibility alias `engine-*` vẫn được giữ để tránh churn import Rust hàng loạt

## Key Insights
- "engine-" prefix gây confusion — ngụ ý rendering engine, thực tế là terminal primitives
- Rename chủ yếu là package/path/docs; lượt này giữ `lib.name` và Cargo alias `engine-*` để tránh churn import Rust hàng loạt.
- Workspace dependency keys vẫn giữ dạng `engine-*` như compatibility layer, nhưng package/path vocabulary đã chuyển sang `chatminal-terminal-*` / `chatminal-*`.

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
- `[workspace.dependencies]`: update package names + paths; giữ compatibility aliases `engine-*` trong lượt rename này

### 4. Update all consumer Cargo.toml files
- Desktop, config, host-runtime, lua-bridge, etc.
- Consumer manifests tiếp tục dùng compatibility alias `engine-*.workspace = true`; không bắt buộc churn alias ở lượt này

### 5. Rust imports
- Giữ nguyên `use engine_term::`, `use engine_font::`, ... bằng cách giữ `lib.name` và compatibility alias hiện tại
- Không đổi code logic trong phase này

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
- [x] Zero `chatminal-engine-*` crate names in workspace manifests/docs active scope
- [x] Active package/path/docs vocabulary updated; compatibility imports vẫn build
- [x] `cargo check --workspace` clean
- [x] Author metadata updated

## Follow-Up Plan
- Phase này đã được tách khỏi closeout của `260401-0949-architecture-unification`.
- Follow-up explicit:
  - [../260403-1800-post-unification-followups/phase-02-terminal-crate-rename.md](../260403-1800-post-unification-followups/phase-02-terminal-crate-rename.md)
