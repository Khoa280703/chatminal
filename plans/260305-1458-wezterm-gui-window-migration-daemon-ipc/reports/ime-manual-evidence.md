# IME Manual Evidence - Phase 05

Generated: 2026-03-05

## Linux Evidence
- Auto smoke (`phase06-input-modifier-ime-smoke.sh`): collected.
  - `reports/fidelity-linux-phase03-fullscreen.json`
  - `reports/fidelity-linux-phase06-input-ime.json`
- IME real keyboard method (manual): moved to external release preflight checklist (host-specific).

## macOS Evidence
- Manual smoke with WezTerm binary: external release preflight checklist (requires macOS host run).

## Required Manual Checklist
| Case | Linux | macOS | Notes |
| --- | --- | --- | --- |
| Vietnamese Telex commit/cancel | external-preflight | external-preflight | Verify no duplicate commit |
| Japanese IME commit/cancel/reconvert | external-preflight | external-preflight | Verify candidate list flow |
| Chinese Pinyin commit/cancel | external-preflight | external-preflight | Verify compose + candidate select |
| Ctrl+C under IME active composition | external-preflight | external-preflight | No stuck input queue |

## Command Template (Manual)
```bash
make daemon
make window
```
Inside terminal:
```bash
cat
# compose with IME -> commit/cancel
# Ctrl+C to interrupt
```

## Sign-off
- Owner: linux-gate-automation
- Date: 2026-03-05
