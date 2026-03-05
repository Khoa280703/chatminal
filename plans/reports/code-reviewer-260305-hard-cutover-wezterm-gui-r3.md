# Code Review - hard-cutover wezterm-gui r3

## High
1. Rollback/runbook docs still depend on old backend path that was removed in hard cutover, so incident rollback instructions are no longer executable as written.
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:48
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:49
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-04-session-profile-history-compatibility-and-rollout-guard.md:23
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-04-session-profile-history-compatibility-and-rollout-guard.md:31
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-06-rollout-rollback-and-windows-followup.md:23
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-06-rollout-rollback-and-windows-followup.md:64
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-06-rollout-rollback-and-windows-followup.md:74

## Medium
1. Plan status drift: phase map says Phase 01 completed, but phase-01 doc still `Pending` and TODO checklist still unchecked.
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:32
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-01-baseline-and-architecture-mapping.md:13
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-01-baseline-and-architecture-mapping.md:55

2. Release-gate chronology is inconsistent: doc says "must pass before default-on" while same plan already states default switched to `window-wezterm-gui`.
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:41
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:60
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:68

3. Phase-05 steps still instruct dual-backend command parameterization (`window-wezterm` vs `window-wezterm-gui`) although old command was removed by cutover.
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/phase-05-fidelity-and-performance-test-gates.md:48
- Refs: /home/khoa2807/working-sources/chatminal/plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/plan.md:48

## Low
1. Changelog still phrases `proxy-wezterm-session` as a normal added command; runtime now hides it from public command surface and only allows it under internal env gate.
- Refs: /home/khoa2807/working-sources/chatminal/docs/project-changelog.md:7
- Refs: /home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:40
- Refs: /home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:78
- Refs: /home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:81
- Refs: /home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/main.rs:237

## Unresolved Questions
1. Rollback expectation now should be binary rollback (revert/redeploy) only, or do you still want an in-place runtime kill-switch path after hard cutover?
