#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
ENDPOINT="${CHATMINAL_DAEMON_ENDPOINT:-/tmp/chatminald-phase05-fidelity-${RUN_ID}.sock}"
DATA_DIR="${CHATMINAL_DATA_DIR:-/tmp/chatminal-phase05-fidelity-${RUN_ID}}"
REPORT_PATH="${CHATMINAL_FIDELITY_REPORT:-/tmp/chatminal-phase05-fidelity-report-${RUN_ID}.json}"
PREVIEW_LINES="${CHATMINAL_PREVIEW_LINES:-400}"
PROFILE="${CHATMINAL_PROFILE:-dev}"
POLL_TIMEOUT_SECONDS="${CHATMINAL_FIDELITY_TIMEOUT_SECONDS:-8}"
POLL_INTERVAL_SECONDS="${CHATMINAL_FIDELITY_POLL_INTERVAL_SECONDS:-0.1}"

mkdir -p "$DATA_DIR"
rm -f "$ENDPOINT"

APP_ARGS=(run --manifest-path apps/chatminal-app/Cargo.toml --)
DAEMON_ARGS=(run --manifest-path apps/chatminald/Cargo.toml)
if [[ "$PROFILE" == "release" ]]; then
  APP_ARGS=(run --release --manifest-path apps/chatminal-app/Cargo.toml --)
  DAEMON_ARGS=(run --release --manifest-path apps/chatminald/Cargo.toml)
fi

TMP_DIR="$(mktemp -d "/tmp/chatminal-phase05-fidelity.${RUN_ID}.XXXXXX")"
DAEMON_LOG="$TMP_DIR/daemon.log"
CREATE_JSON="$TMP_DIR/create.json"
ACTIVATE_JSON="$TMP_DIR/activate.json"
INPUT_JSON="$TMP_DIR/input.json"
SNAPSHOT_JSON="$TMP_DIR/snapshot.json"
SNAPSHOT_ERR="$TMP_DIR/snapshot.err"

status="failed"
failed_step="bootstrap"
failure_reason=""
session_id=""
create_ok=false
activate_ok=false
input_ok=false
snapshot_ok=false
has_marker=false

finalize() {
  local exit_code=$?

  if [[ -n "${DAEMON_PID:-}" ]]; then
    kill "$DAEMON_PID" >/dev/null 2>&1 || true
    wait "$DAEMON_PID" >/dev/null 2>&1 || true
  fi
  rm -f "$ENDPOINT"

  if [[ -z "${failure_reason}" && "$status" != "passed" ]]; then
    failure_reason="script exited with code ${exit_code}"
  fi

  mkdir -p "$(dirname "$REPORT_PATH")" >/dev/null 2>&1 || true
  set +e
  cat >"$REPORT_PATH" <<JSON
{
  "type": "phase05_fidelity_smoke",
  "timestamp_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "endpoint": "$ENDPOINT",
  "data_dir": "$DATA_DIR",
  "session_id": "$session_id",
  "status": "$status",
  "failed_step": "$failed_step",
  "failure_reason": "$failure_reason",
  "checks": [
    { "id": "create_session", "pass": $create_ok },
    { "id": "activate_session", "pass": $activate_ok },
    { "id": "write_input", "pass": $input_ok },
    { "id": "snapshot_command", "pass": $snapshot_ok },
    { "id": "snapshot_contains_marker", "pass": $has_marker }
  ],
  "pass": $( [[ "$status" == "passed" ]] && echo "true" || echo "false" ),
  "artifacts": {
    "daemon_log": "$DAEMON_LOG",
    "create_json": "$CREATE_JSON",
    "activate_json": "$ACTIVATE_JSON",
    "input_json": "$INPUT_JSON",
    "snapshot_json": "$SNAPSHOT_JSON",
    "snapshot_err": "$SNAPSHOT_ERR"
  }
}
JSON
  echo "fidelity report: $REPORT_PATH"
  set -e

}
trap finalize EXIT

fail_now() {
  failed_step="$1"
  failure_reason="$2"
  exit 1
}

(
  cd "$ROOT_DIR"
  CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" cargo "${DAEMON_ARGS[@]}"
) >"$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

for _ in {1..120}; do
  if [[ -S "$ENDPOINT" ]]; then
    break
  fi
  sleep 0.1
done
if [[ ! -S "$ENDPOINT" ]]; then
  fail_now "daemon_boot" "daemon socket not ready: $ENDPOINT"
fi

if ! (
  cd "$ROOT_DIR"
  CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" cargo "${APP_ARGS[@]}" create phase05-fidelity
) >"$CREATE_JSON"; then
  fail_now "create_session" "create session command failed"
fi
create_ok=true

session_id="$(rg -o '"session_id":\s*"[^"]+"' "$CREATE_JSON" | head -n 1 | sed -E 's/.*"([^"]+)"/\1/')"
if [[ -z "$session_id" ]]; then
  fail_now "parse_session_id" "failed to parse session_id from create response"
fi

if ! (
  cd "$ROOT_DIR"
  CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" cargo "${APP_ARGS[@]}" activate "$session_id" 120 32 "$PREVIEW_LINES"
) >"$ACTIVATE_JSON"; then
  fail_now "activate_session" "activate session command failed"
fi
activate_ok=true

input_payload=$'printf "phase05-fidelity-ok\\n"\n'
if ! (
  cd "$ROOT_DIR"
  CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" cargo "${APP_ARGS[@]}" input "$session_id" "$input_payload"
) >"$INPUT_JSON"; then
  fail_now "write_input" "input command failed"
fi
input_ok=true

start_ts="$(date +%s)"
while true; do
  if (
    cd "$ROOT_DIR"
    CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" cargo "${APP_ARGS[@]}" snapshot "$session_id" "$PREVIEW_LINES"
  ) >"$SNAPSHOT_JSON" 2>"$SNAPSHOT_ERR"; then
    snapshot_ok=true
    if rg -q "phase05-fidelity-ok" "$SNAPSHOT_JSON"; then
      has_marker=true
      break
    fi
  fi

  now_ts="$(date +%s)"
  elapsed=$(( now_ts - start_ts ))
  if (( elapsed >= POLL_TIMEOUT_SECONDS )); then
    fail_now "snapshot_poll" "marker not found within ${POLL_TIMEOUT_SECONDS}s"
  fi
  sleep "$POLL_INTERVAL_SECONDS"
done

status="passed"
failed_step=""
failure_reason=""
