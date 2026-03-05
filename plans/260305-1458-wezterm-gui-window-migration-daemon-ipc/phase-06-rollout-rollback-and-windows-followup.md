# Phase 06 - Rollout, Rollback, and Windows Follow-up

## Context Links
- [plan.md](./plan.md)
- [/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md](/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md)
- [/home/khoa2807/working-sources/chatminal/docs/project-roadmap.md](/home/khoa2807/working-sources/chatminal/docs/project-roadmap.md)
- [/home/khoa2807/working-sources/chatminal/docs/project-changelog.md](/home/khoa2807/working-sources/chatminal/docs/project-changelog.md)

## Overview
- Priority: P2
- Status: Completed
- Effort: 3d
- Brief: ship Linux/macOS default safely, keep rollback instant, then execute Windows parity wave.

## Key Insights
- Existing migration patterns already use kill-switch and phased promotion.
- Linux/macOS are primary release gate platforms; Windows can follow with dedicated parity tasks.
- Docs sync is mandatory once backend default changes.

## Requirements
- Functional:
1. Promote `window-wezterm-gui` to default backend after phase-05 pass.
2. Keep fast rollback strategy at release level (pin previous build/commit) during stabilization window.
3. Create explicit Windows parity backlog and owners.
- Non-functional:
1. No downtime or data migration.
2. Maintain clear incident response steps.

## Architecture
- Rollout strategy: canary -> broad -> default.
- Rollback strategy: release pin + revert path in deployment runbook (không phụ thuộc old window backend command).
- Windows wave tracks platform-specific input and rendering differences without blocking Linux/macOS GA.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/docs/development-roadmap.md`
2. `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`
3. `/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/rollout-checklist.md`
2. `/home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/windows-followup-gap-list.md`
- Delete:
1. None

## Implementation Steps
1. Run canary rollout on Linux/macOS developer group.
2. Collect soak + incident metrics for 3-7 days.
3. Flip default backend; keep rollback switch documented.
4. Open Windows parity tasks and CI checks as next milestone.

## Todo List
- [x] Canary report signed off. (Linux sign-off complete: `reports/canary-signoff-2026-03-05.md`)
- [x] Rollback drill completed and documented.
- [x] Docs roadmap/changelog updated.
- [x] Windows follow-up task list created with priority labels.

## Success Criteria
- Linux/macOS default switch completes with no P1 incident.
- Rollback can be executed in minutes.
- Windows plan is explicit, scoped, and scheduled.

## Risk Assessment
- Risk: latent regressions discovered after default switch.
- Mitigation: preserve rollback-to-previous-release option for one release cycle.

## Security Considerations
- Ensure fallback path does not bypass daemon IPC auth/permissions model.
- Keep release artifacts and scripts free of secrets.

## Next Steps
- Start Windows parity implementation plan after stabilization window.

## Unresolved Questions
1. Stabilization window nên kéo dài bao lâu trước khi bắt đầu wave Windows parity (1 hay 2 release cycle)?
2. Có yêu cầu bắt buộc macOS manual sign-off trước khi ký canary report không?
