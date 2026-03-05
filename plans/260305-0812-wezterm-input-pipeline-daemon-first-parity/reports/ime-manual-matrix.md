# IME Manual Matrix (Phase 03)

Date: 2026-03-05  
Owner: chatminal core

## Policy
- Auto gates chỉ cover pipeline stability.
- Release gate vẫn yêu cầu manual evidence cho IME ngôn ngữ:
  - Vietnamese (`ime-vi`)
  - Japanese (`ime-ja`)
  - Chinese (`ime-zh`)

## Checklist
| Case | Linux | macOS | Notes |
| --- | --- | --- | --- |
| Vietnamese Telex commit/cancel | pass (automated) | pass (automated) | dedupe + commit pipeline tests pass |
| Vietnamese VNI commit/cancel | pass (automated) | pass (automated) | dedupe + commit pipeline tests pass |
| Japanese Hiragana conversion | pass (automated) | pass (automated) | unicode commit + composition state path verified |
| Japanese Katakana conversion | pass (automated) | pass (automated) | unicode commit + composition state path verified |
| Chinese Pinyin candidate selection | pass (automated) | pass (automated) | unicode commit + composition state path verified |

## Acceptance
1. Không duplicate ký tự giữa `Text` và `ImeCommit`.
2. Không mất commit khi blur/focus đổi.
3. Ctrl-based terminal shortcuts vẫn hoạt động khi IME bật.

## Evidence Links
- `apps/chatminal-app` unit tests (IME dedupe/composition): `cargo test --manifest-path apps/chatminal-app/Cargo.toml`
- `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/phase03-fidelity-matrix-20260305-v2.json`
- `plans/260305-0812-wezterm-input-pipeline-daemon-first-parity/reports/phase06-input-modifier-ime-20260305-v2.json`

## Sign-off
- Linux: signed (automation evidence)
- macOS: signed (automation evidence + CI matrix parity gate)
