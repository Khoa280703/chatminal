#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
SOCKET="${CHATMINAL_DAEMON_ENDPOINT:-/tmp/chatminald-phase06-killswitch-${RUN_ID}.sock}"
DAEMON_LOG="${CHATMINAL_PHASE06_DAEMON_LOG:-/tmp/chatminald-phase06-killswitch-${RUN_ID}.log}"
DATA_DIR="${CHATMINAL_DATA_DIR:-/tmp/chatminal-phase06-killswitch-${RUN_ID}}"
ATTACH_TIMEOUT_SECONDS="${CHATMINAL_PHASE06_ATTACH_TIMEOUT_SECONDS:-5}"

resolve_timeout_bin() {
  if command -v timeout >/dev/null 2>&1; then
    echo "timeout"
    return
  fi
  if command -v gtimeout >/dev/null 2>&1; then
    echo "gtimeout"
    return
  fi
  if command -v perl >/dev/null 2>&1; then
    echo "perl"
    return
  fi
  echo ""
}

TIMEOUT_BIN="$(resolve_timeout_bin)"
DESKTOP_TRANSCRIPT=""
LEGACY_TRANSCRIPT=""

cleanup() {
  pkill -P $$ >/dev/null 2>&1 || true
  wait >/dev/null 2>&1 || true
  rm -f "$SOCKET" "$DAEMON_LOG"
  rm -rf "$DATA_DIR"
  rm -f "/tmp/chatminal-phase06-script-${RUN_ID}-"*.log
  rm -f "/tmp/chatminal-phase06-attach-${RUN_ID}-"*.log
}
trap cleanup EXIT

run_attach_with_pty() {
  local mode="$1"
  local session_id="$2"
  local transcript="/tmp/chatminal-phase06-attach-${RUN_ID}-${mode}.log"
  case "$mode" in
    desktop)
      DESKTOP_TRANSCRIPT="$transcript"
      ;;
    legacy)
      LEGACY_TRANSCRIPT="$transcript"
      ;;
  esac
  local cmd="CHATMINAL_INPUT_PIPELINE_MODE=${mode} CHATMINAL_DAEMON_ENDPOINT=${SOCKET} ${APP_BIN} attach ${session_id} 120 32 200"
  local exit_code=0
  local ret_code=0
  local had_errexit=0
  if [[ "$-" == *e* ]]; then
    had_errexit=1
  fi

  set +e
  if [[ -n "$TIMEOUT_BIN" ]]; then
    case "$TIMEOUT_BIN" in
      timeout|gtimeout)
        if script -h 2>&1 | rg -q -- "-c"; then
          "$TIMEOUT_BIN" "${ATTACH_TIMEOUT_SECONDS}s" script -qfec "$cmd" "$transcript" >/dev/null 2>&1
        else
          "$TIMEOUT_BIN" "${ATTACH_TIMEOUT_SECONDS}s" script -qF "$transcript" bash -lc "$cmd" >/dev/null 2>&1
        fi
        exit_code=$?
        ;;
      perl)
        if script -h 2>&1 | rg -q -- "-c"; then
          {
            perl -e 'alarm shift; exec @ARGV' \
              "$ATTACH_TIMEOUT_SECONDS" \
              script -qfec "$cmd" "$transcript" >/dev/null 2>&1
          } 2>/dev/null
        else
          {
            perl -e 'alarm shift; exec @ARGV' \
              "$ATTACH_TIMEOUT_SECONDS" \
              script -qF "$transcript" bash -lc "$cmd" >/dev/null 2>&1
          } 2>/dev/null
        fi
        exit_code=$?
        if [[ "$exit_code" -eq 142 ]]; then
          exit_code=124
        fi
        ;;
    esac
  else
    script -qF "$transcript" bash -lc "$cmd" >/dev/null 2>&1 &
    local pid=$!
    local deadline=$(( $(date +%s) + ATTACH_TIMEOUT_SECONDS ))

    while kill -0 "$pid" >/dev/null 2>&1; do
      if (( $(date +%s) >= deadline )); then
        kill "$pid" >/dev/null 2>&1 || true
        sleep 0.2
        kill -KILL "$pid" >/dev/null 2>&1 || true
        wait "$pid" >/dev/null 2>&1 || true
        exit_code=124
        break
      fi
      sleep 0.1
    done

    if [[ "$exit_code" -ne 124 ]]; then
      wait "$pid"
      exit_code=$?
    fi
  fi

  ret_code="$exit_code"

  if [[ "$had_errexit" -eq 1 ]]; then
    set -e
  fi
  return "$ret_code"
}

transcript_has_attach_ready_banner() {
  local transcript="$1"
  if [[ -z "$transcript" || ! -f "$transcript" ]]; then
    return 1
  fi
  for _ in $(seq 1 10); do
    [[ -s "$transcript" ]] && break
    sleep 0.1
  done
  if [[ ! -s "$transcript" ]]; then
    return 1
  fi
  tr -d '\r' <"$transcript" | rg -q "Attached " && return 0
  return 1
}

assert_workspace_alive() {
  CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" "$APP_BIN" workspace >/dev/null
}

cd "$ROOT_DIR"
cargo build --manifest-path apps/chatminald/Cargo.toml >/dev/null
cargo build --manifest-path apps/chatminal-app/Cargo.toml >/dev/null

DAEMON_BIN="$ROOT_DIR/target/debug/chatminald"
APP_BIN="$ROOT_DIR/target/debug/chatminal-app"

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" "$DAEMON_BIN" >"$DAEMON_LOG" 2>&1 &

for _ in $(seq 1 80); do
  if [[ -S "$SOCKET" ]]; then
    break
  fi
  sleep 0.1
done

if [[ ! -S "$SOCKET" ]]; then
  echo "daemon socket not ready: $SOCKET"
  tail -n 80 "$DAEMON_LOG" || true
  exit 1
fi

create_json="$(CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" "$APP_BIN" create phase06-killswitch-test)"
session_id="$(printf '%s' "$create_json" | rg -o '"session_id":\s*"[^"]+"' | head -n 1 | sed -E 's/.*"([^"]+)"/\1/')"
if [[ -z "$session_id" ]]; then
  echo "failed to parse session_id from create response"
  echo "$create_json"
  exit 1
fi

set +e
run_attach_with_pty "desktop" "$session_id"
desktop_exit=$?
assert_workspace_alive || desktop_exit=126
run_attach_with_pty "legacy" "$session_id"
legacy_exit=$?
assert_workspace_alive || legacy_exit=126
set -e

if [[ "$desktop_exit" -ne 0 && "$desktop_exit" -ne 124 ]]; then
  echo "desktop mode attach startup failed: exit=$desktop_exit"
  exit 1
fi
if [[ "$legacy_exit" -ne 0 && "$legacy_exit" -ne 124 ]]; then
  echo "legacy mode attach startup failed: exit=$legacy_exit"
  exit 1
fi

if ! transcript_has_attach_ready_banner "$DESKTOP_TRANSCRIPT"; then
  echo "desktop mode attach did not reach ready banner"
  [[ -n "$DESKTOP_TRANSCRIPT" ]] && tail -n 120 "$DESKTOP_TRANSCRIPT" || true
  exit 1
fi
if ! transcript_has_attach_ready_banner "$LEGACY_TRANSCRIPT"; then
  echo "legacy mode attach did not reach ready banner"
  [[ -n "$LEGACY_TRANSCRIPT" ]] && tail -n 120 "$LEGACY_TRANSCRIPT" || true
  exit 1
fi

echo "phase06 killswitch verify passed: session_id=$session_id desktop_exit=$desktop_exit legacy_exit=$legacy_exit"
