# Chatminal

Chatminal is a local desktop terminal workspace.

Current runtime stack:
- Host shell: `Tauri v2` (`src-tauri/`)
- Backend: `Rust + portable-pty`
- Frontend: `Svelte 5 + xterm.js` (`frontend/`)

Last updated: 2026-03-02

## Runtime Status
- Active runtime is `src-tauri/` + `frontend/`.
- Legacy Rust/Iced code still exists in `src/` and root `Cargo.toml`.
- Legacy code is kept for reference/backward maintenance only; it is not the default runtime flow.

## Core Features
- Multi-profile workspace management:
  - `list_profiles`
  - `create_profile`
  - `switch_profile`
  - `rename_profile`
  - `delete_profile`
- Multi-session terminal management inside the active profile:
  - create, list, activate, rename, close
  - resize, write input, snapshot hydration
- Real-time PTY events:
  - `pty/output`
  - `pty/exited`
  - `pty/error`
- App lifecycle + tray mode:
  - close main window -> hide to tray (configurable)
  - tray menu supports show/new session/quit completely
  - optional start-in-tray preference
- Persistence (SQLite):
  - profiles
  - session metadata (`name`, `cwd`, `status`, `persist_history`, `last_seq`)
  - scrollback chunks with retention
  - app state keys (`active_profile_id`, `active_session_id:{profile_id}`)
  - lifecycle preference keys (`keep_alive_on_close`, `start_in_tray`)
- Lazy reconnect:
  - disconnected sessions are hydrated from preview
  - PTY respawn happens on `activate_session` (also triggered before input when needed)

## Architecture At A Glance
- `src-tauri/src/main.rs`: Tauri command registration and app wiring.
- `src-tauri/src/service.rs`: PTY session lifecycle, IO, workers, event emit.
- `src-tauri/src/persistence.rs`: SQLite schema, workspace restore, retention.
- `src-tauri/src/config.rs`: `settings.json` + legacy `config.toml` shell fallback.
- `frontend/src/App.svelte`: xterm UI, profile/session UX, invoke/listen bridge.

## Project Layout
- `frontend/`: Svelte app and xterm integration.
- `src-tauri/`: Tauri runtime and PTY service.
- `src/`: legacy Iced implementation.
- `docs/`: project documentation.
- `plans/`: planning artifacts and reports.

## Prerequisites
- macOS 13+ or Linux desktop with GUI (Wayland/X11).
- Rust/Cargo stable (`rust-version = 1.93`).
- Node.js + npm.

macOS:
```bash
xcode-select --install
```

Ubuntu/Debian example:
```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf
```

## Development Run
```bash
npm --prefix frontend install
npx --prefix frontend tauri dev
```

Notes:
- Do not use `cargo run` at repo root for the active runtime.
- GUI runtime requires display access (`DISPLAY` or `WAYLAND_DISPLAY` on Linux).

## Build
```bash
npm --prefix frontend run build
npx --prefix frontend tauri build
```

Debug bundle build:
```bash
npx --prefix frontend tauri build --debug
```

Expected artifact roots:
- Bundles: `src-tauri/target/release/bundle/` (or `src-tauri/target/debug/bundle/` with `--debug`)
- Rust binary: `src-tauri/target/release/`

Bundle formats depend on host platform/toolchain (for example `.app`/`.dmg` on macOS, `.deb`/`.AppImage`/`.rpm` on Linux when supported).

## Runtime Configuration
Primary config file: `settings.json`
- Linux: `~/.config/chatminal/settings.json`
- macOS: `~/Library/Application Support/chatminal/settings.json`

Supported keys:
```json
{
  "theme": "system",
  "font_size": 14.0,
  "default_shell": "/bin/bash",
  "persist_scrollback_enabled": false,
  "max_lines_per_session": 5000,
  "auto_delete_after_days": 30,
  "preview_lines": 100
}
```

Normalization in backend:
- `font_size`: `8.0..=48.0`
- `max_lines_per_session`: `100..=5000`
- `auto_delete_after_days`: `0..=3650`
- `preview_lines`: `10..=5000`

Legacy shell file (still read as fallback):
- Linux: `~/.config/chatminal/config.toml`
- macOS: `~/Library/Application Support/chatminal/config.toml`

```toml
shell = "/bin/bash"
```

Shell resolution order:
1. `settings.json` -> `default_shell`
2. legacy `config.toml` -> `shell`
3. `$SHELL`
4. `/bin/zsh`
5. `/bin/bash`
6. `/bin/sh`

Shell path must pass:
- `/etc/shells` allow-list
- canonicalization
- executable bit check

## Session and CWD Behavior
- Session create `cwd` behavior:
  - use payload `cwd` when provided
  - otherwise use user home directory (`~`) when available
  - fallback to `/` only if home cannot be resolved
- Running sessions are tracked by a CWD sync worker (`500ms` interval).
- Reconnect uses latest persisted `cwd` for session respawn.

## Window Lifecycle Behavior
- If `keep_alive_on_close = true`, closing the main window hides it to tray and keeps PTY sessions alive.
- `Quit Completely` from tray triggers backend graceful shutdown and exits the app process.
- If `start_in_tray = true`, app starts hidden and can be restored from tray.

## Persistence Paths
Database file:
- Linux: `~/.local/share/chatminal/chatminal.db`
- macOS: `~/Library/Application Support/chatminal/chatminal.db`

Key SQLite tables:
- `profiles`
- `sessions`
- `scrollback`
- `app_state`

Important app-state keys:
- `active_profile_id`
- `active_session_id:{profile_id}`

## Quick Smoke Checklist
1. App starts and loads workspace via `load_workspace`.
2. If no sessions exist, UI creates one session.
3. Creating/switching/renaming/deleting profiles works.
4. Creating/activating/renaming/closing sessions works.
5. Terminal input goes through `write_input` and output appears via `pty/output`.
6. Restart restores disconnected previews; activate reconnects and resumes output.
7. `cwd` changes persist after restart/reconnect.

## Troubleshooting
| Symptom | Likely Cause | Action |
| --- | --- | --- |
| App window does not open | Missing GUI/display environment | Run in local desktop environment. |
| `tauri dev` fails before app launch | Missing WebKit/GTK libs | Install Linux prerequisites above. |
| macOS build/dev fails early | Missing Xcode CLI tools | Run `xcode-select --install`. |
| Session creation fails | Invalid shell config | Fix/remove `default_shell`/`shell`; ensure shell is in `/etc/shells`. |
| App opens but no session appears | Session spawn failed on startup | Check terminal logs, validate shell path and permissions, then create session manually. |
| No persisted preview/history | `persist_history` disabled or retention trimmed | Enable persist for session; verify retention settings. |

## Documentation
- [Docs Index](./docs/index.md)
- [Project Overview and PDR](./docs/project-overview-pdr.md)
- [Codebase Summary](./docs/codebase-summary.md)
- [System Architecture](./docs/system-architecture.md)
- [Code Standards](./docs/code-standards.md)
- [Deployment Guide](./docs/deployment-guide.md)
- [Design Guidelines](./docs/design-guidelines.md)
- [Project Roadmap](./docs/project-roadmap.md)
- [Development Roadmap](./docs/development-roadmap.md)
- [Project Changelog](./docs/project-changelog.md)
