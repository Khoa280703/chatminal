# Project Overview and PDR

Last updated: 2026-03-02  
Version: 0.2.1-doc-sync

## Product Overview
Chatminal is a local desktop terminal workspace with profile-scoped sessions.

Active runtime:
- Desktop host: `Tauri v2` (`src-tauri/`)
- Backend: `Rust + portable-pty`
- Frontend: `Svelte 5 + xterm.js` (`frontend/`)

Legacy runtime note:
- Root `Cargo.toml` + `src/` (Iced) remain buildable but are not default runtime flow.

## Goals
1. Manage multiple profiles and multiple terminal sessions per profile.
2. Stream PTY output to UI with stable session/event contracts.
3. Keep reconnect predictable with disconnected previews and explicit activation.
4. Persist workspace/session metadata and scrollback with retention controls.
5. Keep shell execution constrained by validation and bounded IO.

## Non-Goals (Current)
1. Remote SSH orchestration.
2. Cross-device sync.
3. Plugin/extension ecosystem.

## Functional Requirements
| ID | Requirement | Status |
| --- | --- | --- |
| FR-01 | Provide workspace bootstrap via `load_workspace` (profiles, sessions, active IDs). | Implemented |
| FR-02 | Provide profile lifecycle commands (`list_profiles`, `create_profile`, `switch_profile`, `rename_profile`, `delete_profile`). | Implemented |
| FR-03 | Provide session lifecycle commands (`list_sessions`, `create_session`, `activate_session`, `rename_session`, `close_session`). | Implemented |
| FR-04 | Bridge terminal IO (`write_input`, `resize_session`, `get_session_snapshot`). | Implemented |
| FR-05 | Emit runtime events (`pty/output`, `pty/exited`, `pty/error`). | Implemented |
| FR-06 | Persist session metadata (`name`, `cwd`, `status`, `persist_history`, `last_seq`). | Implemented |
| FR-07 | Persist and trim scrollback by line cap and TTL. | Implemented |
| FR-08 | Support lazy reconnect for disconnected sessions on activation/input path. | Implemented |
| FR-09 | Track active workspace state keys per profile. | Implemented |

## Non-Functional Requirements
| ID | Requirement | Evidence |
| --- | --- | --- |
| NFR-01 | Input payload must be bounded. | Input-size guard in `src-tauri/src/service.rs` |
| NFR-02 | Input queue must be bounded and non-blocking on hot path. | Bounded queue + `try_send` in `src-tauri/src/service.rs` |
| NFR-03 | Snapshot output must be bounded. | Snapshot-size guard in `src-tauri/src/service.rs` |
| NFR-04 | Shell path must be validated before spawn. | `/etc/shells` + canonicalization + executable checks |
| NFR-05 | Persistence writes must not block PTY reader path. | history queue + batch writer worker (`50ms`, batch `128`) |
| NFR-06 | Session status cleanup should be deterministic on exit/disconnect. | cleanup worker updates session state and emits exit event |
| NFR-07 | CWD state must remain current across long-running sessions. | CWD sync worker (`500ms`) persists updates |

## Acceptance Criteria
1. `npm --prefix frontend install` succeeds.
2. `npx --prefix frontend tauri dev` launches app and `load_workspace` returns a coherent state.
3. Profile lifecycle operations work end-to-end from UI.
4. Session lifecycle operations work end-to-end from UI.
5. Output is streamed via `pty/output`; errors/exits surface via `pty/error` and `pty/exited`.
6. Restart restores profile/session workspace with disconnected preview content.
7. Activating a disconnected session reconnects it and resumes live output.
8. New session default `cwd` is home (`~`) when available (fallback `/` only if home resolution fails).

## Technical Constraints
1. Rust toolchain `1.93+`, Node.js/npm required.
2. GUI desktop context required (macOS or Linux desktop with display).
3. Unix shell policy enforced through `/etc/shells`.
4. SQLite persistence uses local filesystem data/config directories.

## Risks and Mitigations
| Risk | Impact | Mitigation |
| --- | --- | --- |
| Invalid shell config | Session spawn failure | Multi-candidate shell fallback + strict shell validation + troubleshooting docs |
| Queue pressure under heavy output | Input lag or dropped writes | bounded queue + backpressure error surface |
| Persistence failures | Missing restore/history | runtime fallback when persistence unavailable + warning logs |
| Docs drift during runtime evolution | Integration confusion | required docs sync checklist in `docs/code-standards.md` |

## Requirement Change Notes
- 2026-03-02: Added explicit profile lifecycle requirements and active-state key semantics.
- 2026-03-02: Clarified lazy reconnect path and default `cwd` behavior.
- 2026-03-02: Updated constraints and acceptance criteria to current Tauri runtime.
