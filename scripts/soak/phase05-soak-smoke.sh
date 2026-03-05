#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
REPORT_PATH="${CHATMINAL_SOAK_REPORT:-/tmp/chatminal-phase05-soak-report-${RUN_ID}.json}"
SOAK_MODE="${CHATMINAL_SOAK_MODE:-pr}"
SOAK_DURATION_SECONDS="${CHATMINAL_SOAK_DURATION_SECONDS:-7200}"
SOAK_ITERATION_SLEEP_SECONDS="${CHATMINAL_SOAK_ITERATION_SLEEP_SECONDS:-5}"
MAX_ITERATIONS="${CHATMINAL_SOAK_MAX_ITERATIONS:-0}"
SOAK_WARMUP_ITERATIONS_RAW="${CHATMINAL_SOAK_WARMUP_ITERATIONS:-}"
SOAK_PR_ITERATIONS="${CHATMINAL_SOAK_PR_ITERATIONS:-2}"
SOAK_BENCH_SAMPLES="${CHATMINAL_SOAK_BENCH_SAMPLES:-40}"
SOAK_BENCH_WARMUP="${CHATMINAL_SOAK_BENCH_WARMUP:-8}"
SOAK_BENCH_TIMEOUT_MS="${CHATMINAL_SOAK_BENCH_TIMEOUT_MS:-2000}"
SOAK_BENCH_SHELL="${CHATMINAL_SOAK_BENCH_SHELL:-/bin/sh}"
SOAK_REQUIRE_BENCH_HARD_GATE="${CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE:-0}"

case "$SOAK_MODE" in
  pr|nightly) ;;
  *)
    echo "invalid CHATMINAL_SOAK_MODE='$SOAK_MODE' (expected: pr|nightly)" >&2
    exit 1
    ;;
esac

if [[ -n "$SOAK_WARMUP_ITERATIONS_RAW" ]]; then
  SOAK_WARMUP_ITERATIONS="$SOAK_WARMUP_ITERATIONS_RAW"
elif [[ "$SOAK_MODE" == "nightly" ]]; then
  SOAK_WARMUP_ITERATIONS=1
else
  SOAK_WARMUP_ITERATIONS=1
fi

if ! [[ "$SOAK_WARMUP_ITERATIONS" =~ ^[0-9]+$ ]]; then
  echo "invalid CHATMINAL_SOAK_WARMUP_ITERATIONS='$SOAK_WARMUP_ITERATIONS' (expected non-negative integer)" >&2
  exit 1
fi
if ! [[ "$SOAK_PR_ITERATIONS" =~ ^[0-9]+$ ]] || [[ "$SOAK_PR_ITERATIONS" -lt 1 ]]; then
  echo "invalid CHATMINAL_SOAK_PR_ITERATIONS='$SOAK_PR_ITERATIONS' (expected positive integer)" >&2
  exit 1
fi
if ! [[ "$SOAK_BENCH_SAMPLES" =~ ^[0-9]+$ ]] || [[ "$SOAK_BENCH_SAMPLES" -lt 1 ]]; then
  echo "invalid CHATMINAL_SOAK_BENCH_SAMPLES='$SOAK_BENCH_SAMPLES' (expected positive integer)" >&2
  exit 1
fi
if ! [[ "$SOAK_BENCH_WARMUP" =~ ^[0-9]+$ ]]; then
  echo "invalid CHATMINAL_SOAK_BENCH_WARMUP='$SOAK_BENCH_WARMUP' (expected non-negative integer)" >&2
  exit 1
fi
if ! [[ "$SOAK_BENCH_TIMEOUT_MS" =~ ^[0-9]+$ ]] || [[ "$SOAK_BENCH_TIMEOUT_MS" -lt 1 ]]; then
  echo "invalid CHATMINAL_SOAK_BENCH_TIMEOUT_MS='$SOAK_BENCH_TIMEOUT_MS' (expected positive integer)" >&2
  exit 1
fi
if [[ "$SOAK_REQUIRE_BENCH_HARD_GATE" != "0" && "$SOAK_REQUIRE_BENCH_HARD_GATE" != "1" ]]; then
  echo "invalid CHATMINAL_SOAK_REQUIRE_BENCH_HARD_GATE='$SOAK_REQUIRE_BENCH_HARD_GATE' (expected 0|1)" >&2
  exit 1
fi
if [[ "$SOAK_MODE" == "pr" ]] && (( SOAK_WARMUP_ITERATIONS >= SOAK_PR_ITERATIONS )); then
  SOAK_WARMUP_ITERATIONS=$((SOAK_PR_ITERATIONS - 1))
fi

extract_metric() {
  local raw_log="$1"
  local pattern="$2"
  local fallback="$3"
  local value
  value="$(rg -o "$pattern" "$raw_log" | tail -n 1 | sed -E "s/.*=//" || true)"
  if [[ -z "$value" ]]; then
    value="$fallback"
  fi
  printf '%s' "$value"
}

extract_numeric_or_zero() {
  local value="$1"
  if [[ "$value" == "null" ]]; then
    printf '0'
  else
    printf '%s' "$value"
  fi
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  printf '%s' "$value"
}

iterations_json=""
iteration_count=0
pass_count=0
fail_count=0
evaluated_iterations=0
max_p95_ms=0
max_p99_ms=0
max_daemon_peak_mb=0
max_app_peak_mb=0
max_total_peak_mb=0
overall_hard_gate=true

start_epoch="$(date +%s)"
start_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

should_continue() {
  if [[ "$SOAK_MODE" == "pr" ]]; then
    [[ "$iteration_count" -lt "$SOAK_PR_ITERATIONS" ]]
    return
  fi

  local now_epoch
  now_epoch="$(date +%s)"
  local elapsed
  elapsed=$((now_epoch - start_epoch))
  if (( elapsed >= SOAK_DURATION_SECONDS )); then
    return 1
  fi
  if (( MAX_ITERATIONS > 0 && iteration_count >= MAX_ITERATIONS )); then
    return 1
  fi
  return 0
}

while should_continue; do
  iteration_count=$((iteration_count + 1))
  raw_log="$(mktemp "/tmp/chatminal-phase05-soak-log.${RUN_ID}.${iteration_count}.XXXXXX")"
  iteration_started_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  iteration_start_epoch="$(date +%s)"

  set +e
  (
    cd "$ROOT_DIR"
    CHATMINAL_BENCH_ENFORCE_HARD_GATE="$SOAK_REQUIRE_BENCH_HARD_GATE" \
    CHATMINAL_BENCH_SAMPLES="$SOAK_BENCH_SAMPLES" \
    CHATMINAL_BENCH_WARMUP="$SOAK_BENCH_WARMUP" \
    CHATMINAL_BENCH_TIMEOUT_MS="$SOAK_BENCH_TIMEOUT_MS" \
    CHATMINAL_BENCH_SHELL="$SOAK_BENCH_SHELL" \
    bash scripts/bench/phase02-rtt-memory-gate.sh
  ) | tee "$raw_log"
  bench_exit=${PIPESTATUS[0]}
  set -e

  iteration_end_epoch="$(date +%s)"
  iteration_duration_seconds=$((iteration_end_epoch - iteration_start_epoch))

  p95_ms="$(extract_metric "$raw_log" 'p95_ms=[0-9.]+' 'null')"
  p99_ms="$(extract_metric "$raw_log" 'p99_ms=[0-9.]+' 'null')"
  daemon_peak_mb="$(extract_metric "$raw_log" 'daemon_peak_mb=[0-9.]+' 'null')"
  app_peak_mb="$(extract_metric "$raw_log" 'app_peak_mb=[0-9.]+' 'null')"
  total_peak_mb="$(extract_metric "$raw_log" 'total_peak_mb=[0-9.]+' 'null')"

  soft_fail_detected=false
  if rg -q "SOFT-FAIL" "$raw_log"; then
    soft_fail_detected=true
  fi

  pass_hard_gate=false
  if [[ "$SOAK_REQUIRE_BENCH_HARD_GATE" == "1" ]]; then
    if [[ $bench_exit -eq 0 && "$soft_fail_detected" != "true" ]]; then
      pass_hard_gate=true
    fi
  elif [[ $bench_exit -eq 0 ]]; then
    pass_hard_gate=true
  fi

  counts_for_gate=true
  if (( iteration_count <= SOAK_WARMUP_ITERATIONS )); then
    counts_for_gate=false
  fi

  if [[ "$counts_for_gate" == "true" ]]; then
    evaluated_iterations=$((evaluated_iterations + 1))
    if [[ "$pass_hard_gate" == "true" ]]; then
      pass_count=$((pass_count + 1))
    else
      fail_count=$((fail_count + 1))
      overall_hard_gate=false
    fi
  fi

  max_p95_ms="$(awk -v left="$max_p95_ms" -v right="$(extract_numeric_or_zero "$p95_ms")" 'BEGIN { if (right > left) printf "%s", right; else printf "%s", left }')"
  max_p99_ms="$(awk -v left="$max_p99_ms" -v right="$(extract_numeric_or_zero "$p99_ms")" 'BEGIN { if (right > left) printf "%s", right; else printf "%s", left }')"
  max_daemon_peak_mb="$(awk -v left="$max_daemon_peak_mb" -v right="$(extract_numeric_or_zero "$daemon_peak_mb")" 'BEGIN { if (right > left) printf "%s", right; else printf "%s", left }')"
  max_app_peak_mb="$(awk -v left="$max_app_peak_mb" -v right="$(extract_numeric_or_zero "$app_peak_mb")" 'BEGIN { if (right > left) printf "%s", right; else printf "%s", left }')"
  max_total_peak_mb="$(awk -v left="$max_total_peak_mb" -v right="$(extract_numeric_or_zero "$total_peak_mb")" 'BEGIN { if (right > left) printf "%s", right; else printf "%s", left }')"

  if [[ -n "$iterations_json" ]]; then
    iterations_json+=","
  fi
  iterations_json+=$'\n'
  iterations_json+="    {\"index\":${iteration_count},\"started_utc\":\"${iteration_started_utc}\",\"duration_seconds\":${iteration_duration_seconds},\"bench_exit\":${bench_exit},\"soft_fail_detected\":${soft_fail_detected},\"pass_hard_gate\":${pass_hard_gate},\"counts_for_gate\":${counts_for_gate},\"metrics\":{\"p95_ms\":${p95_ms},\"p99_ms\":${p99_ms},\"daemon_peak_mb\":${daemon_peak_mb},\"app_peak_mb\":${app_peak_mb},\"total_peak_mb\":${total_peak_mb}},\"raw_log\":\"$(json_escape "$raw_log")\"}"

  if [[ "$SOAK_MODE" == "nightly" ]]; then
    if ! should_continue; then
      break
    fi
    sleep "$SOAK_ITERATION_SLEEP_SECONDS"
  fi
done

elapsed_seconds="$(( $(date +%s) - start_epoch ))"

status="passed"
failed_step=""
failure_reason=""
if [[ "$evaluated_iterations" -eq 0 ]]; then
  status="failed"
  failed_step="soak_iterations"
  failure_reason="no evaluated soak iterations (all warmup)"
elif [[ "$overall_hard_gate" != "true" ]]; then
  status="failed"
  failed_step="soak_iterations"
  failure_reason="one or more soak iterations failed hard gate"
fi

write_report() {
  local target_path="$REPORT_PATH"
  local fallback_path="/tmp/chatminal-phase05-soak-report-${RUN_ID}.json"
  local wrote=false

  for candidate in "$target_path" "$fallback_path"; do
    mkdir -p "$(dirname "$candidate")" >/dev/null 2>&1 || true
    set +e
    (
      cat >"$candidate" <<JSON
{
  "type": "phase05_soak_smoke",
  "timestamp_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "$status",
  "failed_step": "$failed_step",
  "failure_reason": "$(json_escape "$failure_reason")",
  "started_utc": "$start_utc",
  "mode": "$SOAK_MODE",
  "elapsed_seconds": $elapsed_seconds,
  "configured_duration_seconds": $SOAK_DURATION_SECONDS,
  "warmup_iterations": $SOAK_WARMUP_ITERATIONS,
  "pr_iterations": $SOAK_PR_ITERATIONS,
  "bench_profile": {
    "samples": $SOAK_BENCH_SAMPLES,
    "warmup": $SOAK_BENCH_WARMUP,
    "timeout_ms": $SOAK_BENCH_TIMEOUT_MS,
    "shell": "$(json_escape "$SOAK_BENCH_SHELL")",
    "require_hard_gate": $SOAK_REQUIRE_BENCH_HARD_GATE
  },
  "evaluated_iterations": $evaluated_iterations,
  "iteration_sleep_seconds": $SOAK_ITERATION_SLEEP_SECONDS,
  "iterations_total": $iteration_count,
  "pass_count": $pass_count,
  "fail_count": $fail_count,
  "hard_gate_enforced": $SOAK_REQUIRE_BENCH_HARD_GATE,
  "pass_hard_gate": $overall_hard_gate,
  "max_metrics": {
    "p95_ms": $max_p95_ms,
    "p99_ms": $max_p99_ms,
    "daemon_peak_mb": $max_daemon_peak_mb,
    "app_peak_mb": $max_app_peak_mb,
    "total_peak_mb": $max_total_peak_mb
  },
  "iterations": [${iterations_json}
  ],
  "metrics": {
    "p95_ms": $max_p95_ms,
    "p99_ms": $max_p99_ms,
    "daemon_peak_mb": $max_daemon_peak_mb,
    "app_peak_mb": $max_app_peak_mb,
    "total_peak_mb": $max_total_peak_mb
  },
  "artifacts": {
    "report_path": "$(json_escape "$candidate")"
  }
}
JSON
    ) >/dev/null 2>&1
    local write_code=$?
    set -e
    if [[ $write_code -eq 0 ]]; then
      REPORT_PATH="$candidate"
      wrote=true
      break
    fi
  done

  if [[ "$wrote" != "true" ]]; then
    echo "warning: failed to write soak report to '$target_path' and fallback '$fallback_path'" >&2
  fi
}

write_report
echo "soak report: $REPORT_PATH"
if [[ "$status" != "passed" ]]; then
  exit 1
fi
