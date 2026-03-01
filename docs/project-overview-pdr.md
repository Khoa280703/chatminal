# Project Overview and PDR

Last updated: 2026-03-01
Version: 0.1.0

## Project Overview
Chatminal is a desktop terminal workspace that lets users run and switch multiple local shell sessions in one GUI window.
Current implementation is Rust + Iced for rendering and portable-pty for shell process management.

## Problem Statement
Standard terminal tabs split workflow across multiple windows or apps.
Chatminal targets a single-window, keyboard-first workflow with explicit session list, fast switching, and readable scrollback.

## Goals
1. Run multiple local shell sessions concurrently.
2. Render ANSI terminal output with colors, styles, cursor, scrollback.
3. Keep interaction low-latency under normal developer shell usage.
4. Keep behavior safe by validating shell execution path and bounding risky inputs.

## Non-Goals (Current Scope)
1. Remote terminal protocols (SSH client mode).
2. Session persistence across app restart.
3. Plugin system and external extensions.
4. Multi-platform parity beyond current Unix-oriented implementation.

## Functional Requirements
| ID | Requirement | Status |
| --- | --- | --- |
| FR-01 | User can create a new session from sidebar and keyboard shortcut. | Implemented |
| FR-02 | User can switch active session from sidebar. | Implemented |
| FR-03 | User can close active/selected session. | Implemented |
| FR-04 | Keyboard input is forwarded to active PTY session. | Implemented |
| FR-05 | ANSI text/styles/colors render in terminal view. | Implemented |
| FR-06 | Scrollback review is available by wheel and Shift+PageUp/PageDown. | Implemented |
| FR-07 | Active sessions resize with window changes. | Implemented |
| FR-08 | Session exit triggers cleanup and UI removal. | Implemented |

## Non-Functional Requirements
| ID | Requirement | Current Evidence |
| --- | --- | --- |
| NFR-01 | App should not terminate on PTY write after peer exit. | `main.rs` ignores broken-pipe signal at startup. |
| NFR-02 | App should reject oversized input payloads to PTY channel. | `MAX_INPUT_BYTES = 65_536` enforced in `SessionManager::send_input`. |
| NFR-03 | Shell launch should accept only valid system shells. | Validation via `/etc/shells`, canonical path, executable bit check. |
| NFR-04 | Rendering should avoid full redraw when no state change. | Canvas cache + generation invalidation in terminal canvas state. |
| NFR-05 | Code should include automated tests for core state behavior. | `cargo test` passes with 13 unit tests. |
| NFR-06 | Runtime numeric settings should stay in safe bounds. | `Config::normalized()` clamps scrollback/font/sidebar values before use. |

## Acceptance Criteria
1. Running `cargo run` launches window with at least one created session.
2. `Alt+N` creates session; `Alt+W` closes active session.
3. `Shift+PageUp` and wheel scrolling move viewport upward when scrollback exists.
4. ANSI output from commands like `ls --color=auto` renders expected color variants.
5. `cargo test` passes without failures.

## Technical Constraints
1. Rust toolchain pinned by manifest requirement: `rust-version = 1.93`.
2. Rendering stack is coupled to Iced canvas model.
3. PTY behavior and `/etc/shells` validation assume Unix-like host.
4. No persistence layer exists; all session state is in-memory.

## Dependencies
- `iced`
- `portable-pty`
- `vte`
- `tokio`
- `uuid`
- `indexmap`
- `serde`
- `toml`
- `dirs`
- `log`, `env_logger`
- `libc`

## Risks and Mitigations
| Risk | Impact | Mitigation |
| --- | --- | --- |
| High-volume output can outpace UI updates | Lag or dropped updates | Keep channel bounded in design, add stress test/bench in next phase. |
| Linux-specific assumptions in shell path handling | Portability limits | Document Linux-first support and add platform abstraction phase. |
| No persistent sessions | Data loss on app exit/crash | Add optional restore design in future milestone. |

## Requirement Change Log
- 2026-03-01: Initial PDR created from implemented codebase.
- 2026-03-01: Updated NFR evidence for config clamp hardening and expanded unit-test baseline.
