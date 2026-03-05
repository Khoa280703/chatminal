# Phase 05 - Quality Gates, Fidelity Matrix, Soak

## Context Links
- [plan.md](./plan.md)
- [phase-03-linux-macos-ime-composition-path.md](./phase-03-linux-macos-ime-composition-path.md)
- [phase-04-daemon-contract-and-runtime-stability.md](./phase-04-daemon-contract-and-runtime-stability.md)
- [/home/khoa2807/working-sources/chatminal/scripts/bench/phase02-rtt-memory-gate.sh](/home/khoa2807/working-sources/chatminal/scripts/bench/phase02-rtt-memory-gate.sh)
- [/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase03-fidelity-matrix-smoke.sh](/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase03-fidelity-matrix-smoke.sh)
- [/home/khoa2807/working-sources/chatminal/scripts/soak/phase05-soak-smoke.sh](/home/khoa2807/working-sources/chatminal/scripts/soak/phase05-soak-smoke.sh)
- [/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md](/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md)

## Overview
- Priority: P1
- Status: Completed
- Effort: 6d
- Brief: nâng gate suite để chặn regression cho input pipeline mới trước rollout.

## Key Insights
- Gate hiện có tốt cho RTT/RSS baseline, nhưng chưa bắt đủ IME/modifier scenarios.
- Cần fidelity matrix required cases rõ cho Linux/macOS trước khi default-on.
- Soak hiện ngắn, cần soak dài hơn để bắt leak/deadlock/input drop dài hạn.
- Soak gate nên tập trung stability (crash/freeze/input-loss); RTT hard-gate đã được chặn riêng ở phase-02 benchmark.

## Requirements
- Functional:
1. Gate tự động cho RTT/memory/fidelity/soak có JSON artifacts.
2. Fidelity matrix required cases gồm Ctrl+C/modifier/IME bắt buộc.
3. CI Linux+macOS chạy đủ required gate trước merge.
- Non-functional:
1. Thời gian CI giữ dưới 45 phút cho pipeline chuẩn.
2. Soak dài có thể chạy nightly nếu không phù hợp PR gate.

## Architecture
- Gate tiers:
1. PR gate: compile/tests + RTT/memory + fidelity required subset.
2. Nightly gate: full matrix + 2h soak + extended IME manual evidence.
- Soak semantics:
1. `pr` mode chạy 2 iterations (1 warmup + 1 evaluated) để giảm cold-start flake.
2. `nightly` mode mặc định warmup 1 iteration trước khi tính pass/fail.
3. `CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE` mặc định `0`; khi cần strict latency trong soak thì bật `1`.
- Fidelity required matrix tối thiểu:
1. `bash-prompt`, `unicode`, `resize`, `reconnect`
2. `ctrl-c`, `ctrl-z`, `alt-backspace`, `meta-shortcuts-macos`
3. `ime-vi`, `ime-ja`, `ime-zh`
- Artifact contract:
1. JSON report thống nhất field names để dễ diff trend.

## Related Code Files
- Modify:
1. `/home/khoa2807/working-sources/chatminal/scripts/bench/phase02-rtt-memory-gate.sh`
2. `/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase03-fidelity-matrix-smoke.sh`
3. `/home/khoa2807/working-sources/chatminal/scripts/soak/phase05-soak-smoke.sh`
4. `/home/khoa2807/working-sources/chatminal/.github/workflows/rewrite-quality-gates.yml`
5. `/home/khoa2807/working-sources/chatminal/docs/terminal-fidelity-matrix.md`
6. `/home/khoa2807/working-sources/chatminal/docs/release-checklist.md`
- Create:
1. `/home/khoa2807/working-sources/chatminal/scripts/fidelity/phase06-input-modifier-ime-smoke.sh`
2. `/home/khoa2807/working-sources/chatminal/plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/gate-sample-report-schema.md`
- Delete:
1. None

## Implementation Steps
1. Bổ sung benchmark profile cho input-heavy scenarios (modifier bursts + IME commits).
2. Mở rộng fidelity matrix smoke để cover required input/IME cases.
3. Nâng soak script thành 2 mode: PR-short và nightly-long (2h).
4. Cập nhật workflow matrix Linux/macOS cho required cases.
5. Chuẩn hóa output JSON để so trend qua thời gian.

## Todo List
- [x] RTT gate giữ nguyên threshold: p95<=30ms, p99<=60ms, hard fail p95>45ms.
- [x] Memory gate: target daemon<=120MB, app<=180MB, total<=300MB; hard fail daemon>160MB, app>220MB, total>350MB.
- [x] Fidelity required skip = 0 cho Linux/macOS.
- [x] Soak 2h pass không crash/freeze/input loss.

## Success Criteria
- Gate suite mới bắt được regression đã biết của Ctrl+C/modifiers/IME.
- CI artifacts đủ để truy vết fail root-cause nhanh.
- Hai vòng gate liên tiếp pass trước rollout default-on.

## Risk Assessment
- Risk: CI time tăng mạnh.
- Mitigation: tách PR vs nightly gate; required subset cho PR.
- Risk: host runner không có đầy đủ IME deps.
- Mitigation: IME auto cases tối giản + manual matrix bắt buộc trước release.

## Security Considerations
- Script gate không chạy lệnh phá hoại hệ thống; chỉ thao tác session test riêng.
- Artifact không chứa nội dung terminal nhạy cảm ngoài markers test.

## Next Steps
- Phase 06: rollout dần + rollback diễn tập.

## Decisions Locked
1. Wave đầu không dùng dedicated self-hosted IME nightly runner; chạy trên runner hiện có + manual matrix bắt buộc trước release.
