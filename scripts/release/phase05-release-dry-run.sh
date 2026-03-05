#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
OUTPUT_DIR="${CHATMINAL_RELEASE_OUTPUT_DIR:-/tmp/chatminal-release-dry-run-${RUN_ID}}"
REPORT_PATH="${CHATMINAL_RELEASE_REPORT:-${OUTPUT_DIR}/release-dry-run-report.json}"
ENDPOINT="/tmp/chatminald-release-dry-run-${RUN_ID}.sock"
DATA_DIR="/tmp/chatminal-release-dry-run-data-${RUN_ID}"

DAEMON_BIN_SRC="$ROOT_DIR/target/release/chatminald"
APP_BIN_SRC="$ROOT_DIR/target/release/chatminal-app"
DAEMON_BIN_OUT="$OUTPUT_DIR/chatminald"
APP_BIN_OUT="$OUTPUT_DIR/chatminal-app"
CHECKSUM_FILE="$OUTPUT_DIR/SHA256SUMS"
SMOKE_LOG="$OUTPUT_DIR/smoke.log"
CHECKSUM_TMP_FILE="$CHECKSUM_FILE.tmp"
CHECKSUM_TMP_NAME="$(basename "$CHECKSUM_TMP_FILE")"

status="failed"
failed_step=""
failure_reason=""
CHECKSUM_TOOL=""

cleanup() {
  if [[ -n "${DAEMON_PID:-}" ]]; then
    kill "$DAEMON_PID" >/dev/null 2>&1 || true
    wait "$DAEMON_PID" >/dev/null 2>&1 || true
  fi
  rm -f "$ENDPOINT"
  rm -f "$CHECKSUM_TMP_FILE"
}

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
  local report_dir
  local escaped_status
  local escaped_step
  local escaped_reason
  local escaped_output_dir
  local escaped_daemon_bin
  local escaped_app_bin
  local escaped_checksum
  local escaped_smoke_log

  report_dir="$(dirname "$target_path")"
  if ! mkdir -p "$report_dir" >/dev/null 2>&1; then
    target_path="/tmp/chatminal-release-dry-run-report-${RUN_ID}.json"
    mkdir -p "$(dirname "$target_path")"
  fi

  REPORT_PATH="$target_path"
  escaped_status="$(json_escape "$status")"
  escaped_step="$(json_escape "$failed_step")"
  escaped_reason="$(json_escape "$failure_reason")"
  escaped_output_dir="$(json_escape "$OUTPUT_DIR")"
  escaped_daemon_bin="$(json_escape "$DAEMON_BIN_OUT")"
  escaped_app_bin="$(json_escape "$APP_BIN_OUT")"
  escaped_checksum="$(json_escape "$CHECKSUM_FILE")"
  escaped_smoke_log="$(json_escape "$SMOKE_LOG")"

  cat >"$REPORT_PATH" <<JSON
{
  "type": "phase05_release_dry_run",
  "timestamp_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "$escaped_status",
  "failed_step": "$escaped_step",
  "failure_reason": "$escaped_reason",
  "artifacts": {
    "output_dir": "$escaped_output_dir",
    "daemon_binary": "$escaped_daemon_bin",
    "app_binary": "$escaped_app_bin",
    "checksums": "$escaped_checksum",
    "smoke_log": "$escaped_smoke_log"
  }
}
JSON
}

finalize() {
  local exit_code=$?
  if [[ $exit_code -eq 0 ]]; then
    status="passed"
    failed_step=""
    failure_reason=""
  elif [[ -z "$failure_reason" ]]; then
    failed_step="${failed_step:-runtime}"
    failure_reason="unexpected exit code: $exit_code"
  fi

  cleanup
  write_report

  if [[ $exit_code -eq 0 ]]; then
    echo "release dry-run report: $REPORT_PATH"
  else
    echo "release dry-run failed at step '$failed_step': $failure_reason" >&2
    echo "release dry-run report: $REPORT_PATH" >&2
  fi

  return "$exit_code"
}
trap finalize EXIT

fail_now() {
  failed_step="$1"
  failure_reason="$2"
  status="failed"
  exit 1
}

if ! mkdir -p "$OUTPUT_DIR" "$DATA_DIR"; then
  failed_step="bootstrap"
  fail_now "bootstrap" "failed to create output/data directories"
fi
if ! rm -f "$ENDPOINT"; then
  failed_step="bootstrap"
  fail_now "bootstrap" "failed to cleanup endpoint path: $ENDPOINT"
fi

set_checksum_tool() {
  if command -v sha256sum >/dev/null 2>&1; then
    CHECKSUM_TOOL="sha256sum"
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    CHECKSUM_TOOL="shasum -a 256"
    return
  fi
  fail_now "checksum_tool" "missing checksum command: need sha256sum or shasum"
}

failed_step="build_daemon_release"
if ! (
  cd "$ROOT_DIR"
  cargo build --release --manifest-path apps/chatminald/Cargo.toml
); then
  fail_now "build_daemon_release" "release build for daemon failed"
fi

failed_step="build_app_release"
if ! (
  cd "$ROOT_DIR"
  cargo build --release --manifest-path apps/chatminal-app/Cargo.toml
); then
  fail_now "build_app_release" "release build for app failed"
fi

failed_step="verify_artifacts"
if [[ ! -x "$DAEMON_BIN_SRC" ]]; then
  fail_now "build_daemon" "missing daemon release binary: $DAEMON_BIN_SRC"
fi
if [[ ! -x "$APP_BIN_SRC" ]]; then
  fail_now "build_app" "missing app release binary: $APP_BIN_SRC"
fi

failed_step="copy_artifacts"
if ! cp "$DAEMON_BIN_SRC" "$DAEMON_BIN_OUT"; then
  fail_now "copy_artifacts" "failed to copy daemon artifact"
fi
if ! cp "$APP_BIN_SRC" "$APP_BIN_OUT"; then
  fail_now "copy_artifacts" "failed to copy app artifact"
fi

failed_step="checksum_tool"
set_checksum_tool

failed_step="checksum_generate"
if ! (
  cd "$OUTPUT_DIR"
  case "$CHECKSUM_TOOL" in
    "sha256sum")
      sha256sum chatminald chatminal-app > "$CHECKSUM_TMP_NAME"
      ;;
    "shasum -a 256")
      shasum -a 256 chatminald chatminal-app > "$CHECKSUM_TMP_NAME"
      ;;
    *)
      fail_now "checksum_tool" "unsupported checksum command: $CHECKSUM_TOOL"
      ;;
  esac
); then
  fail_now "checksum_generate" "failed to generate checksum file"
fi
if ! mv "$CHECKSUM_TMP_FILE" "$CHECKSUM_FILE"; then
  fail_now "checksum_generate" "failed to finalize checksum file"
fi

failed_step="daemon_smoke_start"
CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" "$DAEMON_BIN_OUT" >"$SMOKE_LOG" 2>&1 &
DAEMON_PID=$!

failed_step="daemon_smoke_boot"
for _ in {1..120}; do
  if [[ -S "$ENDPOINT" ]]; then
    break
  fi
  sleep 0.1
done
if [[ ! -S "$ENDPOINT" ]]; then
  fail_now "daemon_smoke_boot" "daemon socket not ready: $ENDPOINT"
fi

failed_step="app_smoke_workspace"
if ! CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" "$APP_BIN_OUT" workspace >/dev/null; then
  fail_now "app_smoke_workspace" "release app failed to query workspace"
fi
