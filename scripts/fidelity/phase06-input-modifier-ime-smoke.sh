#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
REPORT_PATH="${CHATMINAL_PHASE06_INPUT_IME_REPORT:-/tmp/chatminal-phase06-input-modifier-ime-report-${RUN_ID}.json}"
MATRIX_REPORT_PATH="${CHATMINAL_FIDELITY_MATRIX_REPORT:-/tmp/chatminal-phase03-fidelity-matrix-report-${RUN_ID}.json}"
REQUIRED_CASES="${CHATMINAL_PHASE06_REQUIRED_CASES:-bash-prompt,ctrl-c,ctrl-c-burst,ctrl-z,unicode,stress-paste,resize,reconnect}"
MANUAL_EVIDENCE_PATH="${CHATMINAL_IME_MANUAL_EVIDENCE_PATH:-plans/260305-1458-wezterm-gui-window-migration-daemon-ipc/reports/ime-manual-evidence.md}"
REQUIRE_MANUAL_SIGNOFF="${CHATMINAL_PHASE06_REQUIRE_MANUAL_IME_SIGNOFF:-0}"

status="failed"
failed_step="phase03_matrix"
failure_reason=""
matrix_exit=1
matrix_status="unknown"
matrix_fail_count=0
matrix_skip_count=0
matrix_required_skip_count=0
manual_signoff_status="pending"
manual_signoff_required=true

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  printf '%s' "$value"
}

write_report() {
  local target_path="$REPORT_PATH"
  local fallback_path="/tmp/chatminal-phase06-input-modifier-ime-report-${RUN_ID}.json"
  local wrote=false

  for candidate in "$target_path" "$fallback_path"; do
    mkdir -p "$(dirname "$candidate")" >/dev/null 2>&1 || true
    set +e
    (
      cat >"$candidate" <<JSON
{
  "type": "phase06_input_modifier_ime_smoke",
  "timestamp_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "$status",
  "failed_step": "$failed_step",
  "failure_reason": "$(json_escape "$failure_reason")",
  "required_cases": "$(json_escape "$REQUIRED_CASES")",
  "matrix_report_path": "$(json_escape "$MATRIX_REPORT_PATH")",
  "manual_evidence_path": "$(json_escape "$MANUAL_EVIDENCE_PATH")",
  "manual_signoff_status": "$(json_escape "$manual_signoff_status")",
  "manual_signoff_required": $manual_signoff_required,
  "matrix": {
    "exit_code": $matrix_exit,
    "status": "$(json_escape "$matrix_status")",
    "fail_count": $matrix_fail_count,
    "skip_count": $matrix_skip_count,
    "required_skip_count": $matrix_required_skip_count
  },
  "ime_manual_evidence_required": true,
  "ime_manual_cases": [
    "ime-vi",
    "ime-ja",
    "ime-zh"
  ],
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
    echo "error: failed to write phase06 report to '$target_path' and fallback '$fallback_path'" >&2
    return 1
  fi
  return 0
}

set +e
(
  cd "$ROOT_DIR"
  CHATMINAL_FIDELITY_MATRIX_REPORT="$MATRIX_REPORT_PATH" \
  CHATMINAL_FIDELITY_STRICT=1 \
  CHATMINAL_FIDELITY_REQUIRED_CASES="$REQUIRED_CASES" \
  bash scripts/fidelity/phase03-fidelity-matrix-smoke.sh
)
matrix_exit=$?
set -e

if [[ -f "$MATRIX_REPORT_PATH" ]]; then
  matrix_status="$(rg -o '"status":\s*"[^"]+"' "$MATRIX_REPORT_PATH" | head -n 1 | sed -E 's/.*"([^"]+)"/\1/' || true)"
  matrix_fail_count="$(rg -o '"fail_count":\s*[0-9]+' "$MATRIX_REPORT_PATH" | head -n 1 | sed -E 's/.*:\s*//' || echo 0)"
  matrix_skip_count="$(rg -o '"skip_count":\s*[0-9]+' "$MATRIX_REPORT_PATH" | head -n 1 | sed -E 's/.*:\s*//' || echo 0)"
  matrix_required_skip_count="$(rg -o '"required_skip_count":\s*[0-9]+' "$MATRIX_REPORT_PATH" | head -n 1 | sed -E 's/.*:\s*//' || echo 0)"
fi

if [[ -f "$MANUAL_EVIDENCE_PATH" ]]; then
  owner_line="$(rg '^- Owner:' "$MANUAL_EVIDENCE_PATH" | tail -n 1 || true)"
  date_line="$(rg '^- Date:' "$MANUAL_EVIDENCE_PATH" | tail -n 1 || true)"
  if [[ -n "$owner_line" && -n "$date_line" && "$owner_line" != *pending* && "$date_line" != *pending* ]]; then
    manual_signoff_status="signed_off"
  fi
fi

script_exit_code=0
if [[ $matrix_exit -eq 0 ]]; then
  if [[ "$manual_signoff_status" == "signed_off" ]]; then
    status="passed"
    failed_step=""
    failure_reason=""
  else
    status="passed_manual_pending"
    failed_step=""
    failure_reason="manual IME sign-off pending"
    if [[ "$REQUIRE_MANUAL_SIGNOFF" == "1" ]]; then
      status="failed"
      failed_step="manual_ime_signoff"
      failure_reason="manual IME sign-off required but still pending"
      script_exit_code=2
    fi
  fi
else
  status="failed"
  failed_step="phase03_matrix"
  failure_reason="phase03 fidelity matrix strict mode failed"
  script_exit_code="$matrix_exit"
fi

write_report || script_exit_code=1
echo "phase06 input+modifier+ime smoke report: $REPORT_PATH"

if [[ $script_exit_code -ne 0 ]]; then
  exit "$script_exit_code"
fi
