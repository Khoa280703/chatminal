# Windows Follow-up Gap List - Phase 06

Generated: 2026-03-05

## P0
1. Validate Named Pipe transport under long-running interactive load (input/output burst).
2. Verify Ctrl+C/Ctrl+Break semantics against ConPTY edge cases.
3. Verify IME path with Vietnamese/Japanese/Chinese layouts on Windows 11.

## P1
1. Add Windows GUI smoke path for `window-wezterm-gui` launcher.
2. Add parity checklist for fullscreen TUIs (`vim`, `nvim`, `htop` alt alternatives, `fzf`).
3. Capture RTT/RSS benchmark baseline on `windows-latest` and compare with Linux/macOS envelope.

## P2
1. Add installer/runbook notes for WezTerm binary discovery on Windows.
2. Add reliability metrics export for pipe reconnect and dropped events.

## Ownership Suggestion
- Transport and IPC: `apps/chatminald` owner.
- Input fidelity: `apps/chatminal-app` owner.
- CI and release: workflow/release owner.
