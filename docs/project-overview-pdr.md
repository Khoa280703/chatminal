# Project Overview and PDR

Last updated: 2026-03-01
Version: 0.1.0

## Overview
Chatminal is a local desktop terminal workspace for running multiple shell sessions in one window.
Current implementation is Rust + Iced UI, `portable-pty` for process control, and `wezterm-term` for terminal parsing/state.

## Problem
Developers often split work across many terminal windows/tabs with weak cross-session visibility.
Chatminal targets one-window, keyboard-first session management with reliable scrollback and ANSI fidelity.

## Product Goals
1. Manage multiple local shell sessions concurrently.
2. Render terminal output with cursor styles, colors, and text attributes.
3. Keep UI responsive under typical interactive shell workloads.
4. Keep runtime safe with shell validation, bounded channels, and input-size limits.

## Non-Goals (Current Scope)
1. Remote protocols (SSH client mode).
2. Session restore across app restarts.
3. Plugin/extension system.
4. Full cross-platform parity beyond current Unix-oriented flow.

## Functional Requirements
| ID | Requirement | Status |
| --- | --- | --- |
| FR-01 | Create new terminal session from sidebar or shortcut. | Implemented |
| FR-02 | Switch active session from sidebar list. | Implemented |
| FR-03 | Close selected/active session and cleanup resources. | Implemented |
| FR-04 | Forward keyboard input bytes to active PTY writer channel. | Implemented |
| FR-05 | Parse/render ANSI output using wezterm terminal state. | Implemented |
| FR-06 | Support scrollback viewport via wheel and `Shift+PageUp/PageDown`. | Implemented |
| FR-07 | Resize PTY sessions on window size updates. | Implemented |
| FR-08 | Handle PTY EOF/read error and remove exited session from UI. | Implemented |

## Non-Functional Requirements
| ID | Requirement | Evidence in code |
| --- | --- | --- |
| NFR-01 | App should not terminate on broken pipe writes. | `main.rs` ignores broken-pipe signal. |
| NFR-02 | PTY input payload must be bounded. | `MAX_INPUT_BYTES = 65_536` in `SessionManager::send_input`. |
| NFR-03 | Shell path must be validated before spawn. | `/etc/shells` allowlist + canonicalization + executable bit checks. |
| NFR-04 | UI redraw should be generation-driven. | `terminal_generation` + canvas cache invalidation. |
| NFR-05 | Scroll position should remain stable while new output arrives. | `lines_added` from stable-row delta updates scroll offsets. |
| NFR-06 | Core behavior must be covered by automated tests. | `cargo test` passes 23 unit tests (2026-03-01). |

## Acceptance Criteria
1. `cargo run` starts app and creates an initial session.
2. `Alt+N` creates a session; `Alt+W` closes active session.
3. Scroll controls work on primary buffer output.
4. Cursor style changes (`block/underline/bar/hidden`) are rendered.
5. `cargo test` passes without failures.

## Technical Constraints
1. Rust toolchain requirement from manifest: `rust-version = 1.93`.
2. UI is coupled to Iced canvas rendering primitives.
3. Shell validation and signal handling are Unix-centric.
4. Session/grid state is in-memory only.

## Dependencies (Runtime)
- `iced`
- `portable-pty`
- `wezterm-term`
- `wezterm-surface`
- `tokio`
- `uuid`
- `indexmap`
- `serde`, `toml`, `dirs`
- `log`, `env_logger`, `libc`

## Risks and Mitigations
| Risk | Impact | Mitigation direction |
| --- | --- | --- |
| High PTY output rate vs UI event queue | Snapshot lag under pressure | Coalesced update retry exists; add throughput/load tests. |
| Unix-specific shell policy | Portability limits | Keep Linux-first docs; add platform abstraction phase. |
| No persistence boundary | State lost on restart/crash | Plan optional restore/import milestone later. |

## Change Notes
- 2026-03-01: Updated PDR to wezterm-based parser/runtime flow, stable-row `lines_added`, exited-event sender-thread model, and 23-test baseline.
