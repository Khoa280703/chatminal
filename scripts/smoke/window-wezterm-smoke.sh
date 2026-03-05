#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SOCKET="/tmp/chatminald-smoke-$$.sock"
DAEMON_LOG="/tmp/chatminald-smoke-$$.log"

cleanup() {
  pkill -P $$ || true
  rm -f "$SOCKET" "$DAEMON_LOG"
}
trap cleanup EXIT

if ! command -v xvfb-run >/dev/null 2>&1; then
  echo "xvfb-run is required for window smoke test"
  exit 1
fi

cd "$ROOT_DIR"

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" cargo run --manifest-path apps/chatminald/Cargo.toml >"$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

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

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" cargo run --manifest-path apps/chatminal-app/Cargo.toml -- workspace >/dev/null

set +e
timeout 6s xvfb-run -a \
  env CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
  cargo run --manifest-path apps/chatminal-app/Cargo.toml -- window-wezterm 200 120 32 >/tmp/chatminal-window-smoke.log 2>&1
WINDOW_EXIT=$?
set -e

if [[ "$WINDOW_EXIT" -ne 0 && "$WINDOW_EXIT" -ne 124 ]]; then
  echo "window smoke failed with exit code: $WINDOW_EXIT"
  tail -n 80 /tmp/chatminal-window-smoke.log || true
  exit 1
fi

kill "$DAEMON_PID" 2>/dev/null || true
wait "$DAEMON_PID" 2>/dev/null || true

echo "window smoke passed"
