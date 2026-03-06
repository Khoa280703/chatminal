#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
SOCKET="${CHATMINAL_DAEMON_ENDPOINT:-/tmp/chatminald-phase08-killswitch-${RUN_ID}.sock}"
DATA_DIR="${CHATMINAL_DATA_DIR:-/tmp/chatminal-phase08-killswitch-${RUN_ID}}"
DAEMON_LOG="${CHATMINAL_PHASE08_DAEMON_LOG:-/tmp/chatminald-phase08-killswitch-${RUN_ID}.log}"
WEZTERM_MOCK_LOG="${CHATMINAL_PHASE08_WEZTERM_MOCK_LOG:-/tmp/chatminal-phase08-wezterm-mock-${RUN_ID}.log}"
WEZTERM_MOCK_BIN="${CHATMINAL_PHASE08_WEZTERM_MOCK_BIN:-/tmp/chatminal-phase08-wezterm-mock-${RUN_ID}.sh}"
LEGACY_LOG="${CHATMINAL_PHASE08_LEGACY_LOG:-/tmp/chatminal-phase08-legacy-${RUN_ID}.log}"
LEGACY_READY_MARKER="${CHATMINAL_PHASE08_LEGACY_READY_MARKER:-/tmp/chatminal-phase08-legacy-ready-${RUN_ID}.txt}"
REQUIRE_LEGACY_HEADLESS="${CHATMINAL_PHASE08_REQUIRE_LEGACY_HEADLESS:-0}"

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

cleanup() {
  pkill -P $$ >/dev/null 2>&1 || true
  rm -f "$SOCKET" "$DAEMON_LOG" "$WEZTERM_MOCK_LOG" "$WEZTERM_MOCK_BIN" "$LEGACY_LOG" "$LEGACY_READY_MARKER"
  rm -rf "$DATA_DIR"
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
  echo "phase08 killswitch verify failed: daemon socket not ready at $SOCKET"
  tail -n 120 "$DAEMON_LOG" || true
  exit 1
fi

create_json="$(CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" "$APP_BIN" create phase08-killswitch-test)"
session_id="$(printf '%s' "$create_json" | rg -o '"session_id":\s*"[^"]+"' | head -n 1 | sed -E 's/.*"([^"]+)"/\1/')"
if [[ -z "$session_id" ]]; then
  echo "phase08 killswitch verify failed: cannot parse session_id"
  echo "$create_json"
  exit 1
fi

cat >"$WEZTERM_MOCK_BIN" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
{
  echo "argv:$*"
  echo "endpoint:${CHATMINAL_DAEMON_ENDPOINT:-}"
  echo "internal_proxy:${CHATMINAL_INTERNAL_PROXY:-}"
} >"${CHATMINAL_PHASE08_WEZTERM_MOCK_LOG:?missing mock log path}"
EOF
chmod +x "$WEZTERM_MOCK_BIN"

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
CHATMINAL_DATA_DIR="$DATA_DIR" \
CHATMINAL_WINDOW_BACKEND="wezterm-gui" \
CHATMINAL_WEZTERM_BIN="$WEZTERM_MOCK_BIN" \
CHATMINAL_PHASE08_WEZTERM_MOCK_LOG="$WEZTERM_MOCK_LOG" \
"$APP_BIN" window-wezterm-gui "$session_id"

for _ in $(seq 1 40); do
  [[ -f "$WEZTERM_MOCK_LOG" ]] && break
  sleep 0.05
done

if [[ ! -f "$WEZTERM_MOCK_LOG" ]]; then
  echo "phase08 killswitch verify failed: wezterm mock log missing"
  exit 1
fi

mock_payload="$(cat "$WEZTERM_MOCK_LOG")"
if ! rg -q "argv:start -- .*proxy-wezterm-session" <<<"$mock_payload"; then
  echo "phase08 killswitch verify failed: wezterm backend did not invoke proxy launcher"
  echo "$mock_payload"
  exit 1
fi
if ! rg -q "internal_proxy:1" <<<"$mock_payload"; then
  echo "phase08 killswitch verify failed: internal proxy flag missing"
  echo "$mock_payload"
  exit 1
fi

if [[ "$(uname -s)" == "Linux" ]]; then
  if ! command -v xvfb-run >/dev/null 2>&1; then
    if [[ "$REQUIRE_LEGACY_HEADLESS" == "1" ]]; then
      echo "phase08 killswitch verify failed: xvfb-run missing for legacy backend check"
      exit 1
    fi
    echo "phase08 killswitch: skip legacy backend runtime check (xvfb-run not installed)"
    echo "phase08 wezterm-gui killswitch verify passed: session_id=$session_id"
    exit 0
  fi

  set +e
  run_with_timeout \
    xvfb-run -a env \
      CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
      CHATMINAL_DATA_DIR="$DATA_DIR" \
      CHATMINAL_WINDOW_BACKEND="legacy" \
      CHATMINAL_LEGACY_WINDOW_READY_FILE="$LEGACY_READY_MARKER" \
      "$APP_BIN" window-wezterm-gui "$session_id" >"$LEGACY_LOG" 2>&1
  legacy_exit=$?
  set -e

  if [[ "$legacy_exit" -ne 0 && "$legacy_exit" -ne 124 ]]; then
    echo "phase08 killswitch verify failed: legacy backend launch error exit=$legacy_exit"
    tail -n 120 "$LEGACY_LOG" || true
    exit 1
  fi

  if [[ ! -f "$LEGACY_READY_MARKER" ]]; then
    echo "phase08 killswitch verify failed: legacy backend has no ready marker"
    tail -n 120 "$LEGACY_LOG" || true
    exit 1
  fi
else
  echo "phase08 killswitch: skip legacy GUI runtime verification on non-Linux host"
fi

echo "phase08 wezterm-gui killswitch verify passed: session_id=$session_id"
