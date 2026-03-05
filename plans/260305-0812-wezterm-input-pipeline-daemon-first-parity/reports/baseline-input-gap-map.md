# Baseline Input Gap Map (Phase 01)

Date: 2026-03-05  
Owner: chatminal core

## Baseline Inventory
- Input sources:
  - `crossterm` attach path
  - `egui` window path
- Runtime paths:
  - `wezterm` pipeline (shared semantic event + encoder)
  - `legacy` fallback pipeline (kill-switch Phase 06)

## Gap Matrix (Priority)
| Case | Priority | Current baseline | Scope |
| --- | --- | --- | --- |
| Ctrl+C interrupt | P0 | pass | Linux/macOS |
| Ctrl+Z stop fg process | P0 | pass | Linux/macOS |
| Alt+Backspace path stability | P0 | pass (pipeline-level) | Linux/macOS |
| Meta/Cmd shortcuts split | P0 | pass in mapper policy | Linux/macOS |
| Reconnect + input continuity | P0 | pass | Linux/macOS |
| IME dedupe Text/Commit | P0 | mitigated (frame-window dedupe) | Linux/macOS |
| TUI tools (vim/nvim/tmux/fzf) | P1 | host-dependent; tool-missing => skip | Linux/macOS |
| Full IME language matrix (vi/ja/zh) | P1 | manual evidence required | Linux/macOS |
| Windows modifier parity (AltGr, right-alt/right-ctrl) | P2 | deferred to Phase 07 | Windows |
| X11 vs Wayland deep matrix | P2 | deferred wave sau | Linux |

## Platform Scope Decision
1. Wave hiện tại ship gate cho Linux/macOS.
2. Windows input parity là follow-up phase độc lập (không block wave Linux/macOS).
3. Full X11/Wayland và dead-key matrix không block wave đầu.

## Host Baseline
1. Linux representative stack: local Ubuntu host + terminal attach/window pipelines.
2. macOS: CI coverage compile/tests + manual matrix required trước release chính thức.
