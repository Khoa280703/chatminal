# Phase 06 - Migration Rollout, Rollback, Test Checklist

## Context Links
- [plan.md](./plan.md)
- [phase-05-quality-gates-fidelity-matrix-soak.md](./phase-05-quality-gates-fidelity-matrix-soak.md)
- [/home/khoa2807/working-sources/chatminal/docs/release-checklist.md](/home/khoa2807/working-sources/chatminal/docs/release-checklist.md)
- [/home/khoa2807/working-sources/chatminal/Makefile](/home/khoa2807/working-sources/chatminal/Makefile)
- [/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md](/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md)

## Overview
- Priority: P1
- Status: Completed
- Effort: 5d
- Brief: migration không phá runtime đang chạy, có rollback nhanh, có checklist test rõ trước/sau cutover.

## Key Insights
- Input pipeline là hot path, rollout big-bang dễ gây regression khó rollback.
- Cần dual-path có thể bật/tắt runtime mà không thay protocol/store ngay.
- Rollback tốt phải rehearsal trước release, không chỉ viết docs.

## Requirements
- Functional:
1. Rollout theo từng wave (canary -> partial -> default-on) trên Linux/macOS.
2. Có kill-switch rõ (`legacy` mode) khi phát hiện regression.
3. Có checklist test trước/sau từng wave.
- Non-functional:
1. Không downtime daemon bắt buộc cho toàn user base khi rollback.
2. Rollback hoàn tất < 15 phút với release artifact gần nhất.

## Architecture
- Migration strategy:
1. Phase A: ship code with dual path, default `wezterm`.
2. Phase B: internal canary Linux/macOS với kill-switch `legacy`.
3. Phase C: giữ `legacy` fallback 1-2 release cycles trước khi đánh giá loại bỏ.
- Rollback strategy:
1. Runtime rollback: set env/config về `legacy`, restart app.
2. Release rollback: redeploy artifact/tag stable trước đó.
3. Data safety: protocol/store backward-compatible nên không cần DB migration rollback.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/apps/chatminal-app/src/config.rs`
2. `/home/khoa2807/working-sources/chatminal/README.md`
3. `/home/khoa2807/working-sources/chatminal/docs/deployment-guide.md`
4. `/home/khoa2807/working-sources/chatminal/docs/release-checklist.md`
5. `/home/khoa2807/working-sources/chatminal/docs/project-changelog.md`
6. `/home/khoa2807/working-sources/chatminal/docs/project-roadmap.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/rollback-drill-log.md`
2. `/home/khoa2807/working-sources/chatminal/plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/migration-wave-checklist.md`
- Delete:
1. None

## Implementation Steps
1. Thêm config flag chọn pipeline mode và docs hướng dẫn vận hành.
2. Chạy wave canary Linux/macOS với gate suite đầy đủ.
3. Ghi nhận regression, quyết định promote hoặc rollback.
4. Diễn tập rollback runtime + rollback release.
5. Chốt release notes và remove blockers trước default-on.

## Todo List
- [x] Kill-switch được verify hoạt động trên Linux/macOS.
- [x] Rollback drill có timestamp, owner, evidence.
- [x] Checklist test trước/sau rollout được ký xác nhận.
- [x] Docs roadmap/changelog sync với trạng thái rollout.
- [x] Promote gate: 2h soak nightly pass (Linux/macOS artifacts).
- [x] Promote gate: manual IME evidence signed (`ime-vi`, `ime-ja`, `ime-zh`).

## Success Criteria
- Migration không gây outage hoặc mất session data.
- Rollback có thể thực thi nhanh, deterministic.
- Test checklist pass 2 vòng liên tiếp trước release rộng.

## Risk Assessment
- Risk: rollback chỉ đổi app config nhưng daemon behavior vẫn mismatch.
- Mitigation: giữ daemon contract backward-compatible, kiểm thử rollback end-to-end.
- Risk: team quên chạy manual IME checklist khi gate auto pass.
- Mitigation: release checklist bắt buộc mục manual sign-off.

## Security Considerations
- Không bật debug input logs ở production mặc định.
- Rollback/release scripts kiểm tra secret leakage trước publish artifact.

## Next Steps
- Phase 07: Windows parity follow-up sau khi Linux/macOS ổn định.

## Test Checklist
1. `cargo check --workspace`
2. `cargo test --manifest-path crates/chatminal-protocol/Cargo.toml`
3. `cargo test --manifest-path crates/chatminal-store/Cargo.toml`
4. `cargo test --manifest-path apps/chatminald/Cargo.toml`
5. `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
6. `bash scripts/bench/phase02-rtt-memory-gate.sh`
7. `CHATMINAL_FIDELITY_STRICT=1 bash scripts/fidelity/phase03-fidelity-matrix-smoke.sh`
8. `bash scripts/fidelity/phase05-fidelity-smoke.sh`
9. `bash scripts/soak/phase05-soak-smoke.sh`
10. `bash scripts/release/phase05-release-dry-run.sh`
11. Manual matrix: Ctrl+C/modifiers + IME (vi/ja/zh) trên Linux + macOS

## Progress Sync (2026-03-05)
- Final validation run confirm `phase06-killswitch-verify.sh` pass on current HEAD.
- Open severity for this phase scope: `Critical=0`, `High=0`.

## Decisions Locked
1. Giữ `legacy` mode tối thiểu 2 release cycles sau khi `wezterm` path default-on, rồi mới đánh giá loại bỏ.
