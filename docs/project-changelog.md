# Project Changelog

## 2026-03-04

### Changed
- Removed legacy runtime stack completely:
  - deleted obsolete host runtime modules (`src/`)
  - deleted obsolete web UI modules (`frontend/`)
- Workspace now only includes active runtime modules:
  - `apps/chatminald`
  - `apps/chatminal-app`
  - `crates/chatminal-protocol`
  - `crates/chatminal-store`
- Removed legacy compatibility paths:
  - no `config.toml` fallback
  - no typo env aliases
  - no legacy text ping compatibility

### CI
- Removed Node/frontend build gates.
- CI validates Rust workspace and active crates/apps only.
