# Rollback Drill Log (Phase 06)

Date: 2026-03-05  
Owner: chatminal core

## Objective
Diễn tập rollback runtime input pipeline từ `wezterm` sang `legacy` mà không đổi daemon/store contract.

## Drill Steps
1. Start daemon với endpoint tạm.
2. Tạo session test bằng `chatminal-app create`.
3. Chạy attach mode với `CHATMINAL_INPUT_PIPELINE_MODE=wezterm` (timeout cưỡng bức để test start path).
4. Chạy attach mode với `CHATMINAL_INPUT_PIPELINE_MODE=legacy` (timeout cưỡng bức để test rollback path).
5. So sánh exit status và kiểm tra không crash ngay lúc boot attach.

## Command Evidence
```text
phase06 killswitch attach verify passed:
wezterm_exit=124 legacy_exit=124
session_id=c2db4680-2017-4eae-879e-bcff3bb6a531

phase06 killswitch verify passed:
wezterm_exit=124 legacy_exit=124
session_id=cfe6b01a-5f02-4af2-825b-8d0402efdaad

phase06 killswitch verify passed:
wezterm_exit=124 legacy_exit=124
session_id=6176a30a-3004-4ea0-8074-db68b8cd606d

phase06 killswitch verify passed:
wezterm_exit=124 legacy_exit=124
session_id=d359625b-b179-4293-a15c-72c5c251e9a6
```

## Notes (2026-03-05 update)
- Verify script giờ build binary trước khi attach check (không qua `cargo run`) để tránh nhiễu compile-time.
- Fallback path không còn trả `124` cứng khi attach thoát sớm.
- Timeout-based pass (`124`) chỉ được chấp nhận khi transcript attach không rỗng và daemon vẫn trả `workspace` sau mỗi mode check.

## Result
- Rollback runtime path khả dụng:
  - `wezterm` mode attach startup OK.
  - `legacy` mode attach startup OK.
- Không cần migration DB/protocol để rollback mode.

## Rollback Runtime Command (Operator)
```bash
export CHATMINAL_INPUT_PIPELINE_MODE=legacy
```
Restart `chatminal-app`.

## Restore New Pipeline
```bash
export CHATMINAL_INPUT_PIPELINE_MODE=wezterm
```
Restart `chatminal-app`.
