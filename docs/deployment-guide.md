# Deployment Guide

Last updated: 2026-03-03

## Runtime Target
Deploy and run the active runtime only:
- `src-tauri/` + `frontend/`

Legacy note:
- root `src/` + root `Cargo.toml` are legacy and not default deployment path.

## Prerequisites
- macOS 13+ or Linux desktop with GUI.
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

## Release Build
```bash
npm --prefix frontend run build
npx --prefix frontend tauri build
```

Debug bundle:
```bash
npx --prefix frontend tauri build --debug
```

## Release Artifacts
Default output roots:
- `src-tauri/target/release/bundle/`
- `src-tauri/target/debug/bundle/` (when using `--debug`)

Typical artifact formats depend on host OS/tooling:
- macOS: `.app`, `.dmg`
- Linux: `.AppImage`, `.deb`, `.rpm` (toolchain-dependent)

## Runtime Settings (`settings.json`)
Config file:
- Linux: `~/.config/chatminal/settings.json`
- macOS: `~/Library/Application Support/chatminal/settings.json`

Example:
```json
{
  "theme": "system",
  "font_size": 14.0,
  "default_shell": "/bin/bash",
  "persist_scrollback_enabled": false,
  "max_lines_per_session": 5000,
  "auto_delete_after_days": 30,
  "preview_lines": 100,
  "sync_clear_command_to_history": false
}
```

Backend normalization:
- `font_size`: `8.0..=48.0`
- `max_lines_per_session`: `100..=5000`
- `auto_delete_after_days`: `0..=3650`
- `preview_lines`: `10..=5000`
- `sync_clear_command_to_history`: default `false` (opt-in DB sync on `clear`)

Legacy shell fallback file (optional):
- Linux: `~/.config/chatminal/config.toml`
- macOS: `~/Library/Application Support/chatminal/config.toml`

```toml
shell = "/bin/bash"
```

Shell resolution order:
1. `settings.json` `default_shell`
2. legacy `config.toml` `shell`
3. `$SHELL`
4. `/bin/zsh`
5. `/bin/bash`
6. `/bin/sh`

Shell must pass `/etc/shells` + canonicalization + executable checks.

## Persistence and State
Database path:
- Linux: `~/.local/share/chatminal/chatminal.db`
- macOS: `~/Library/Application Support/chatminal/chatminal.db`

Core tables:
- `profiles`, `sessions`, `scrollback`, `app_state`

Important state keys:
- `active_profile_id`
- `active_session_id:{profile_id}`

## Operational Smoke Checklist
1. Launch app and confirm `load_workspace` succeeds.
2. Create/switch/rename/delete profile flows work.
3. Create session, run commands, and confirm `pty/output` stream.
4. Resize window and confirm `resize_session` behavior for running sessions.
5. Restart app and verify disconnected preview restore.
6. Activate a disconnected session and verify reconnect.
7. Change directories in shell, restart, and verify latest `cwd` is reused.
8. Validate history retention settings by trimming scenarios (line cap/TTL).

## Terminal Compatibility Gate (Linux)
Run these commands in Chatminal before release:
1. `vim /tmp/chatminal-vim.txt`
2. `btop` (or `htop`)
3. `printf '%s\n' alpha beta gamma | fzf`
4. `seq 1 300 | less`
5. `nano /tmp/chatminal-unicode.txt`
6. `printf 'e\u0301 | 你 | 😀\n'`
7. Resize window repeatedly + `stty size`

Pass criteria:
- no cursor drift or prompt overlap after exit from full-screen TUIs
- paging/search hotkeys work in interactive tools
- unicode/wide-char spacing remains correct
- resize produces accurate rows/cols and no hard desync

## Troubleshooting
| Symptom | Likely Cause | Action |
| --- | --- | --- |
| App does not open | No GUI/display context | Run inside local desktop environment. |
| `tauri dev` fails early | Missing system dependencies | Install listed platform dependencies. |
| Session creation fails | Invalid shell path/config | Fix `default_shell`/`shell`; ensure path appears in `/etc/shells`. |
| App opens but no session is available | Startup spawn failed and no persisted sessions | Check logs, then create session manually after fixing shell config. |
| `settings.json` edits seem ignored | Values are clamped/normalized | Check normalization ranges above. |
| Persisted history missing | `persist_history` disabled or data trimmed | Enable persist and review retention settings. |
