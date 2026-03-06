#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
SOCKET="${CHATMINAL_DAEMON_ENDPOINT:-/tmp/chatminald-phase08-${RUN_ID}.sock}"
DATA_DIR="/tmp/chatminal-phase08-${RUN_ID}"
DAEMON_LOG="${CHATMINAL_PHASE08_DAEMON_LOG:-/tmp/chatminald-phase08-${RUN_ID}.log}"
WINDOW_LOG="${CHATMINAL_PHASE08_WINDOW_LOG:-/tmp/chatminal-window-phase08-${RUN_ID}.log}"
READY_MARKER="${CHATMINAL_PHASE08_READY_MARKER:-/tmp/chatminal-window-phase08-ready-${RUN_ID}.txt}"

cleanup() {
  pkill -P $$ >/dev/null 2>&1 || true
  rm -f "$SOCKET" "$DAEMON_LOG" "$WINDOW_LOG" "$READY_MARKER"
  case "$DATA_DIR" in
    /tmp/chatminal-phase08-*)
      rm -rf "$DATA_DIR"
      ;;
  esac
}
trap cleanup EXIT

wait_for_socket() {
  local socket_path="$1"
  for _ in $(seq 1 100); do
    if [[ -S "$socket_path" ]]; then
      return 0
    fi
    sleep 0.1
  done
  return 1
}

resolve_timeout_bin() {
  if command -v timeout >/dev/null 2>&1; then
    echo "timeout"
    return
  fi
  if command -v gtimeout >/dev/null 2>&1; then
    echo "gtimeout"
    return
  fi
  echo ""
}

TIMEOUT_BIN="$(resolve_timeout_bin)"

run_with_timeout() {
  if [[ -n "$TIMEOUT_BIN" ]]; then
    "$TIMEOUT_BIN" 8s "$@"
    return $?
  fi
  "$@" &
  local cmd_pid=$!
  (
    sleep 8
    if kill -0 "$cmd_pid" >/dev/null 2>&1; then
      kill "$cmd_pid" >/dev/null 2>&1 || true
    fi
  ) &
  local guard_pid=$!
  wait "$cmd_pid"
  local status=$?
  kill "$guard_pid" >/dev/null 2>&1 || true
  wait "$guard_pid" >/dev/null 2>&1 || true
  return "$status"
}

cd "$ROOT_DIR"
cargo build --manifest-path apps/chatminald/Cargo.toml >/dev/null
cargo build --manifest-path apps/chatminal-app/Cargo.toml >/dev/null

DAEMON_BIN="$ROOT_DIR/target/debug/chatminald"
APP_BIN="$ROOT_DIR/target/debug/chatminal-app"

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" "$DAEMON_BIN" >"$DAEMON_LOG" 2>&1 &

if ! wait_for_socket "$SOCKET"; then
  echo "phase08 verify failed: daemon socket not ready at $SOCKET"
  tail -n 120 "$DAEMON_LOG" || true
  exit 1
fi

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "phase08 verify: skip runtime check on non-Linux host"
  exit 0
fi

if ! command -v xvfb-run >/dev/null 2>&1; then
  echo "phase08 verify failed: xvfb-run missing for headless native window check"
  exit 1
fi

set +e
run_with_timeout \
  xvfb-run -a env \
    CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
    CHATMINAL_DATA_DIR="$DATA_DIR" \
    CHATMINAL_LEGACY_WINDOW_READY_FILE="$READY_MARKER" \
    "$APP_BIN" window >"$WINDOW_LOG" 2>&1
window_exit=$?
set -e

if [[ "$window_exit" -ne 0 && "$window_exit" -ne 124 && "$window_exit" -ne 137 && "$window_exit" -ne 143 ]]; then
  echo "phase08 verify failed: window launch error exit=$window_exit"
  tail -n 120 "$WINDOW_LOG" || true
  exit 1
fi

if [[ ! -f "$READY_MARKER" ]]; then
  echo "phase08 verify failed: native window has no ready marker"
  tail -n 120 "$WINDOW_LOG" || true
  exit 1
fi

echo "phase08 native window verify passed"
