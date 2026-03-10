#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
DESKTOP_MOCK_LOG="${CHATMINAL_PHASE08_DESKTOP_MOCK_LOG:-/tmp/chatminal-phase08-desktop-mock-${RUN_ID}.log}"
DESKTOP_MOCK_BIN="${CHATMINAL_PHASE08_DESKTOP_MOCK_BIN:-/tmp/chatminal-phase08-desktop-mock-${RUN_ID}.sh}"

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
  rm -f "$DESKTOP_MOCK_LOG" "$DESKTOP_MOCK_BIN"
}
trap cleanup EXIT

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
cargo build --manifest-path apps/chatminal-app/Cargo.toml >/dev/null

APP_BIN="$ROOT_DIR/target/debug/chatminal-app"
session_id="phase08-killswitch-test-session"

cat >"$DESKTOP_MOCK_BIN" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
{
  echo "argv:$*"
  echo "sessions_sidebar:${CHATMINAL_DESKTOP_SESSIONS_SIDEBAR:-}"
} >"${CHATMINAL_PHASE08_DESKTOP_MOCK_LOG:?missing mock log path}"
EOF
chmod +x "$DESKTOP_MOCK_BIN"

CHATMINAL_DESKTOP_BIN="$DESKTOP_MOCK_BIN" \
CHATMINAL_PHASE08_DESKTOP_MOCK_LOG="$DESKTOP_MOCK_LOG" \
"$APP_BIN" window-desktop "$session_id"

for _ in $(seq 1 40); do
  [[ -f "$DESKTOP_MOCK_LOG" ]] && break
  sleep 0.05
done

if [[ ! -f "$DESKTOP_MOCK_LOG" ]]; then
  echo "phase08 killswitch verify failed: desktop mock log missing"
  exit 1
fi

mock_payload="$(cat "$DESKTOP_MOCK_LOG")"
if ! rg -q "argv:start -- .*proxy-desktop-session" <<<"$mock_payload"; then
  echo "phase08 killswitch verify failed: desktop backend did not invoke runtime bootstrap command"
  echo "$mock_payload"
  exit 1
fi
if ! rg -q "sessions_sidebar:1" <<<"$mock_payload"; then
  echo "phase08 killswitch verify failed: sidebar env missing"
  echo "$mock_payload"
  exit 1
fi
if ! rg -q "argv:start -- chatminal-runtime proxy-desktop-session ${session_id}" <<<"$mock_payload"; then
  echo "phase08 killswitch verify failed: session id was not forwarded to runtime bootstrap command"
  echo "$mock_payload"
  exit 1
fi

echo "phase08 desktop killswitch verify passed: session_id=$session_id"
