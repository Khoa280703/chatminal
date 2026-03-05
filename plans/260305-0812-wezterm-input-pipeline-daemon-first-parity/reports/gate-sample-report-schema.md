# Gate Report Schema (Phase 05)

## Mục tiêu
- Chuẩn hóa field JSON cho các script gate (`fidelity`, `soak`, `release dry-run`) để CI và tooling dễ parse.
- Dùng cùng key names cho mọi report thay vì mỗi script tự đặt kiểu khác nhau.

## Common Envelope
```json
{
  "type": "string",
  "timestamp_utc": "YYYY-MM-DDTHH:mm:ssZ",
  "status": "passed|failed|passed_with_warnings",
  "failed_step": "string",
  "failure_reason": "string",
  "artifacts": {}
}
```

## Fidelity Matrix Report
```json
{
  "type": "phase03_fidelity_matrix_smoke",
  "status": "passed|failed|passed_with_warnings",
  "fail_count": 0,
  "skip_count": 0,
  "required_skip_count": 0,
  "required_cases": "csv",
  "checks": [
    {
      "id": "ctrl-c",
      "status": "pass|fail|skip",
      "pass": true,
      "note": "string"
    }
  ],
  "artifacts": {
    "tmp_dir": "path",
    "daemon_log": "path"
  }
}
```

## Soak Report
```json
{
  "type": "phase05_soak_smoke",
  "mode": "pr|nightly",
  "elapsed_seconds": 0,
  "warmup_iterations": 1,
  "pr_iterations": 2,
  "evaluated_iterations": 1,
  "iterations_total": 0,
  "bench_profile": {
    "samples": 40,
    "warmup": 8,
    "timeout_ms": 2000,
    "shell": "/bin/sh",
    "require_hard_gate": 0
  },
  "pass_count": 0,
  "fail_count": 0,
  "pass_hard_gate": true,
  "max_metrics": {
    "p95_ms": 0,
    "p99_ms": 0,
    "daemon_peak_mb": 0,
    "app_peak_mb": 0,
    "total_peak_mb": 0
  },
  "iterations": [
    {
      "index": 1,
      "bench_exit": 0,
      "pass_hard_gate": true,
      "metrics": {
        "p95_ms": 0,
        "p99_ms": 0,
        "daemon_peak_mb": 0,
        "app_peak_mb": 0,
        "total_peak_mb": 0
      },
      "raw_log": "path"
    }
  ]
}
```

## Validation Rules
1. `type` bắt buộc, dùng đúng constant theo script.
2. `timestamp_utc` bắt buộc, UTC RFC3339 `Z`.
3. `status=failed` thì `failure_reason` không được rỗng.
4. Các numeric fields phải là number JSON, không phải string.
5. Paths trong `artifacts` nên là absolute path để dễ debug runner.
