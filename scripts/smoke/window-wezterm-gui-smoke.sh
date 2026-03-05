#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="$$"
SOCKET="/tmp/chatminald-wezterm-gui-smoke-${RUN_ID}.sock"
DAEMON_LOG="/tmp/chatminald-wezterm-gui-smoke-${RUN_ID}.log"
WEZTERM_MOCK_LOG="/tmp/chatminal-wezterm-mock-${RUN_ID}.log"
WEZTERM_MOCK_BIN="/tmp/chatminal-wezterm-mock-${RUN_ID}.sh"

cleanup() {
  pkill -P $$ >/dev/null 2>&1 || true
  rm -f "$SOCKET" "$DAEMON_LOG" "$WEZTERM_MOCK_LOG" "$WEZTERM_MOCK_BIN"
}
trap cleanup EXIT

cat >"$WEZTERM_MOCK_BIN" <<EOF
#!/usr/bin/env bash
set -euo pipefail
{
  echo "argv:\$*"
  echo "endpoint:\${CHATMINAL_DAEMON_ENDPOINT:-}"
  echo "internal_proxy:\${CHATMINAL_INTERNAL_PROXY:-}"
} >"$WEZTERM_MOCK_LOG"
EOF
chmod +x "$WEZTERM_MOCK_BIN"

cd "$ROOT_DIR"

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
  cargo run --manifest-path apps/chatminald/Cargo.toml >"$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

for _ in $(seq 1 120); do
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

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
  cargo run --manifest-path apps/chatminal-app/Cargo.toml -- workspace >/dev/null

create_json="$(
  CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
    cargo run --quiet --manifest-path apps/chatminal-app/Cargo.toml -- create "wezterm-gui-smoke"
)"
session_id="$(printf '%s\n' "$create_json" | sed -n 's/.*"session_id":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"

if [[ -z "$session_id" ]]; then
  echo "failed to parse session_id from create response"
  printf '%s\n' "$create_json"
  exit 1
fi

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

CHATMINAL_DAEMON_ENDPOINT="$SOCKET" \
CHATMINAL_WEZTERM_BIN="$WEZTERM_MOCK_BIN" \
  run_launcher_with_timeout \
  cargo run --quiet --manifest-path apps/chatminal-app/Cargo.toml -- window-wezterm-gui "$session_id"

for _ in $(seq 1 30); do
  [[ -f "$WEZTERM_MOCK_LOG" ]] && break
  sleep 0.1
done

if [[ ! -f "$WEZTERM_MOCK_LOG" ]]; then
  echo "wezterm mock log missing"
  exit 1
fi

mock_payload="$(cat "$WEZTERM_MOCK_LOG")"

if ! grep -q "argv:start -- " <<<"$mock_payload"; then
  echo "wezterm start args missing"
  printf '%s\n' "$mock_payload"
  exit 1
fi

if ! grep -q "proxy-wezterm-session" <<<"$mock_payload"; then
  echo "proxy command missing in wezterm args"
  printf '%s\n' "$mock_payload"
  exit 1
fi

if ! grep -q "$session_id" <<<"$mock_payload"; then
  echo "session id not forwarded to wezterm launcher"
  printf '%s\n' "$mock_payload"
  exit 1
fi

if ! grep -q "endpoint:${SOCKET}" <<<"$mock_payload"; then
  echo "daemon endpoint env missing in wezterm launcher"
  printf '%s\n' "$mock_payload"
  exit 1
fi

if ! grep -q "internal_proxy:1" <<<"$mock_payload"; then
  echo "internal proxy env missing in wezterm launcher"
  printf '%s\n' "$mock_payload"
  exit 1
fi

kill "$DAEMON_PID" >/dev/null 2>&1 || true
wait "$DAEMON_PID" >/dev/null 2>&1 || true

echo "window-wezterm-gui smoke passed"
