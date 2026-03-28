# Manual Validation Checklist

## Scope
- Verify canonical scrollback fixes reopen/resize semantics on real desktop UI.
- Focus on shell scrollback path. Alt-screen/TUI exact replay is non-goal.

## Preconditions
- Run `make clean-data`
- Run `make window`
- Use default shell session with `persist_history = true`

## Checklist
- [ ] Fresh session
  - Run a few commands that emit multiple logical lines.
  - Confirm sidebar preview still shows newest logical tail.
- [ ] Reopen same width
  - Quit app completely.
  - Reopen app with same window width.
  - Confirm history restores exactly once, no duplicate prompt tail.
- [ ] Reopen different width
  - Quit app.
  - Reopen app with noticeably narrower width.
  - Confirm restored history wraps to new width instead of preserving old hard-wrap boundaries.
- [ ] Resize after reopen
  - After restore, drag window wider then narrower.
  - Confirm restored history reflows like live content.
- [ ] Mixed-source session
  - Reuse an older DB/session that still contains legacy `scrollback_chunks`.
  - Produce new output after reopen.
  - Confirm old lines remain visible and new lines append without duplicate same-`seq` artifacts.
- [ ] Prompt-tail restore
  - Leave session at shell prompt without trailing command output.
  - Quit app and reopen.
  - Confirm prompt tail appears once and next command starts on a fresh line.
- [ ] Joined sessions
  - Join two sessions.
  - Reopen app.
  - Confirm each joined terminal restores its own history correctly and resizing layout does not corrupt restored content.
- [ ] Profile switching
  - Switch across profiles after restore.
  - Confirm no session loses restored history when becoming inactive/active again.

## Smoke Result Captured In This Session
- `make clean-data` + `make window` launched successfully.
- Desktop process stayed alive past initial startup window.
- No immediate crash/panic observed in startup log.

## Expected Failures That Are Still Non-Goals
- Alt-screen/TUI exact replay mismatch
- Multi-line cursor-motion/progress UI exact reconstruction from persisted history

## Sign-off
- Status: pending manual execution on real UI
