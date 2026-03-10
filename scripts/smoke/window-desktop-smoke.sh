#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="$$"
DESKTOP_MOCK_LOG="/tmp/chatminal-desktop-mock-${RUN_ID}.log"
DESKTOP_MOCK_BIN="/tmp/chatminal-desktop-mock-${RUN_ID}.sh"

cleanup() {
  pkill -P $$ >/dev/null 2>&1 || true
  rm -f "$DESKTOP_MOCK_LOG" "$DESKTOP_MOCK_BIN"
}
trap cleanup EXIT

cat >"$DESKTOP_MOCK_BIN" <<EOF
#!/usr/bin/env bash
set -euo pipefail
{
  echo "argv:\$*"
  echo "sessions_sidebar:\${CHATMINAL_DESKTOP_SESSIONS_SIDEBAR:-}"
} >"$DESKTOP_MOCK_LOG"
EOF
chmod +x "$DESKTOP_MOCK_BIN"

cd "$ROOT_DIR"
session_id="desktop-smoke-session"

run_launcher_with_timeout() {
  if command -v timeout >/dev/null 2>&1; then
    timeout 20s "$@"
  elif command -v gtimeout >/dev/null 2>&1; then
    gtimeout 20s "$@"
  else
    "$@" &
    local cmd_pid=$!
    (
      sleep 20
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
  fi
}

CHATMINAL_DESKTOP_BIN="$DESKTOP_MOCK_BIN" \
CHATMINAL_SKIP_GUI_DISPLAY_CHECK=1 \
  run_launcher_with_timeout \
  cargo run --quiet --manifest-path apps/chatminal-app/Cargo.toml -- window-desktop "$session_id"

for _ in $(seq 1 30); do
  [[ -f "$DESKTOP_MOCK_LOG" ]] && break
  sleep 0.1
done

if [[ ! -f "$DESKTOP_MOCK_LOG" ]]; then
  echo "desktop mock log missing"
  exit 1
fi

mock_payload="$(cat "$DESKTOP_MOCK_LOG")"

if ! grep -q "argv:start -- " <<<"$mock_payload"; then
  echo "desktop start args missing"
  printf '%s\n' "$mock_payload"
  exit 1
fi

if ! grep -q "proxy-desktop-session" <<<"$mock_payload"; then
  echo "proxy command missing in desktop args"
  printf '%s\n' "$mock_payload"
  exit 1
fi

if ! grep -q "$session_id" <<<"$mock_payload"; then
  echo "session id not forwarded to desktop launcher"
  printf '%s\n' "$mock_payload"
  exit 1
fi

if ! grep -q "sessions_sidebar:1" <<<"$mock_payload"; then
  echo "sidebar env missing in desktop launcher"
  printf '%s\n' "$mock_payload"
  exit 1
fi

echo "window-desktop smoke passed"
