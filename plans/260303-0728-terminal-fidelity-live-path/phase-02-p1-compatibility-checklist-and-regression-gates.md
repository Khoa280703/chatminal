# Phase 02 - P1 Compatibility Checklist and Regression Gates

## Context Links
- Plan: [plan.md](plan.md)
- Prev: [phase-01-p0-live-path-fidelity-and-behavior-baseline.md](phase-01-p0-live-path-fidelity-and-behavior-baseline.md)
- Next: [phase-03-p2-daemon-ownership-safe-slice.md](phase-03-p2-daemon-ownership-safe-slice.md)

## Overview
- Priority: P2
- Status: pending
- Effort: 8h
- Goal: Add practical, repeatable compatibility checks for common terminal apps and edge rendering.

## Key Insights
- Current repo has no real terminal compatibility checklist for TUIs.
- Fidelity regressions often appear in alt-screen, unicode width, and resize handling.
- Need checklist that can run today without waiting for daemon cutover.

## Requirements
- Checklist must be executable with clear pass/fail criteria.
- Cover: `vim`, `btop`, `fzf`, `less`, `nano`, unicode width/combining, and resize behavior.
- Include preflight package checks and deterministic commands.
- Keep this as release gate for terminal-fidelity work.

## Architecture
- Add a dedicated section in docs for compatibility runbook.
- Use one profile/session in Chatminal and run the exact command script manually.
- Record result matrix in markdown table (`PASS/FAIL`, notes, environment).

## Related Code Files
- Modify `/home/khoa2807/working-sources/chatminal/README.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/code-standards.md`
- Modify `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`

## Implementation Steps
1. Add preflight command block:
   - `command -v vim btop fzf less nano || true`
   - document fallback install notes.
2. Add compatibility checklist commands and expected outcomes:
   - `vim`: insert/edit/save/quit in alt-screen.
   - `btop`: refresh, keyboard nav, quit.
   - `fzf`: interactive filter and select.
   - `less`: paging/search.
   - `nano`: edit/save unicode line.
   - unicode: combining, CJK, emoji column alignment.
   - resize: rapid window resize + `stty size` + app redraw check.
3. Add a result template table in docs so QA can report execution quickly.
4. Mark checklist as required verification for fidelity-related PRs.

## Todo List
- [ ] Add practical compatibility runbook to README and deployment guide.
- [ ] Add explicit pass/fail criteria for each app scenario.
- [ ] Add regression gate note in code standards.
- [ ] Log first baseline results in changelog after execution.

## Success Criteria
- Team can execute full checklist in one session with no ambiguity.
- Each target app has clear input steps and a binary pass/fail rule.
- Unicode and resize checks catch misalignment and redraw regressions.
- Checklist is linked from top-level docs.

## Risk Assessment
- Risk: environment missing TUI tools.
- Mitigation: include preflight + package install notes and allow partial run with marked skips.
- Risk: manual checklist drift over time.
- Mitigation: keep command blocks copy/paste-ready and version in changelog.

## Security Considerations
- Checklist commands must avoid destructive file/system operations.
- Use temp files under user home or `/tmp` only.

## Next Steps
- After P1 baseline is green, run P2 daemon-safe slice with same checklist.

## Unresolved Questions
1. Should checklist evidence include screenshot capture requirement for every app or only failures?
