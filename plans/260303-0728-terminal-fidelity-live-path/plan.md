---
title: "Terminal Fidelity Hardening - Live Path First"
description: "Execution-first plan to make Chatminal behave closer to a traditional terminal without breaking current runtime."
status: pending
priority: P1
effort: 24h
branch: main
tags: [terminal, fidelity, runtime, tauri, xterm, daemon]
created: 2026-03-03
---

# Terminal Fidelity Plan

## Objective
- Bring runtime behavior closer to native terminal behavior.
- Prioritize live stream path, reduce unnecessary reset/hydrate/snapshot.
- Remove or make explicit any command-level intervention.
- Add practical compatibility checklist for real terminal programs.
- Keep daemon ownership migration incremental and safe.

## P0/P1/P2 Execution Checklist
| Priority | Scope | Status | Effort | Detail |
|---|---|---|---|---|
| P0 | Live path fidelity + behavior normalization | pending | 10h | [Phase 01](phase-01-p0-live-path-fidelity-and-behavior-baseline.md) |
| P1 | Practical compatibility checklist + regression gates | pending | 8h | [Phase 02](phase-02-p1-compatibility-checklist-and-regression-gates.md) |
| P2 | Daemon ownership safe slice with fallback | pending | 6h | [Phase 03](phase-03-p2-daemon-ownership-safe-slice.md) |

## Ship Order
1. Ship P0 first, no daemon dependency.
2. Ship P1 checklist/gates immediately after P0 for confidence on TUIs.
3. Ship P2 safe daemon slice only after P0/P1 pass.

## Files Expected To Change (Consolidated)
- `/home/khoa2807/working-sources/chatminal/frontend/src/App.svelte`
- `/home/khoa2807/working-sources/chatminal/frontend/src/lib/types.ts`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/config.rs`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/models.rs`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/main.rs`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/service.rs`
- `/home/khoa2807/working-sources/chatminal/src-tauri/src/runtime_backend.rs`
- `/home/khoa2807/working-sources/chatminal/README.md`
- `/home/khoa2807/working-sources/chatminal/docs/system-architecture.md`
- `/home/khoa2807/working-sources/chatminal/docs/code-standards.md`
- `/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md`
- `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Verify Gate (Release Blocking)
1. `cargo test --manifest-path src-tauri/Cargo.toml` passes.
2. `npm --prefix frontend run build` passes.
3. Live path no unexpected reset while session is running and active.
4. Compatibility checklist fully green for `vim/btop/fzf/less/nano/unicode/resize`.
5. `CHATMINAL_RUNTIME_BACKEND=daemon` with daemon unavailable still works via safe fallback.

## Unresolved Questions
1. P0 default for command interception: remove fully now or keep behind hidden setting default `off`?
2. OS validation scope for compatibility checklist: Linux only first, or Linux + macOS in same sprint?
