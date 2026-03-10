#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEFAULT_WORK_DIR="$(mktemp -d /tmp/chatminal-bench-XXXXXX)"
WORK_DIR="${CHATMINAL_BENCH_WORKDIR:-$DEFAULT_WORK_DIR}"
SOCKET="${CHATMINAL_BENCH_SOCKET:-$WORK_DIR/chatminald.sock}"
DATA_DIR="${CHATMINAL_BENCH_DATA_DIR:-$WORK_DIR/data}"
DAEMON_LOG="$WORK_DIR/daemon.log"
APP_LOG="$WORK_DIR/app.log"

SAMPLES="${CHATMINAL_BENCH_SAMPLES:-80}"
WARMUP="${CHATMINAL_BENCH_WARMUP:-15}"
TIMEOUT_MS="${CHATMINAL_BENCH_TIMEOUT_MS:-2000}"
COLS="${CHATMINAL_BENCH_COLS:-120}"
ROWS="${CHATMINAL_BENCH_ROWS:-32}"
ENFORCE_HARD_GATE="${CHATMINAL_BENCH_ENFORCE_HARD_GATE:-1}"
BUILD_PROFILE="${CHATMINAL_BENCH_PROFILE:-release}"
BENCH_SHELL="${CHATMINAL_BENCH_SHELL:-/bin/sh}"
BENCH_MAX_SECONDS="${CHATMINAL_BENCH_MAX_SECONDS:-180}"
SAMPLE_INTERVAL_SECONDS="${CHATMINAL_BENCH_SAMPLE_INTERVAL_SECONDS:-0.02}"

DAEMON_HARD_FAIL_KB=$((160 * 1024))
APP_HARD_FAIL_KB=$((220 * 1024))
TOTAL_HARD_FAIL_KB=$((350 * 1024))
P95_HARD_FAIL_MS="50.0"

mkdir -p "$WORK_DIR"

DAEMON_PID=""
APP_PID=""

cleanup() {
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill "$APP_PID" 2>/dev/null || true
  fi
  if [[ -n "$DAEMON_PID" ]] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi
  rm -f "$SOCKET"
  if [[ -z "${CHATMINAL_BENCH_WORKDIR:-}" ]]; then
    rm -rf "$WORK_DIR"
  fi
}
trap cleanup EXIT

rss_kb() {
  local pid="$1"
  if [[ -r "/proc/$pid/status" ]]; then
    awk '/VmRSS:/ { print $2; exit }' "/proc/$pid/status" 2>/dev/null || echo 0
    return
  fi
  ps -o rss= -p "$pid" 2>/dev/null | awk '{print $1+0}' || echo 0
}

rss_tree_kb() {
  local root="$1"
  local total=0
  local value=0
  value="$(rss_kb "$root")"
  total=$((total + value))

  while read -r child; do
    [[ -z "$child" ]] && continue
    value="$(rss_tree_kb "$child")"
    total=$((total + value))
  done < <(ps -o pid= --ppid "$root" 2>/dev/null | awk '{print $1}')

  echo "$total"
}

extract_field() {
  local line="$1"
  local key="$2"
  awk -v key="$key" '{
    for (i=1; i<=NF; i++) {
      split($i, pair, "=");
      if (pair[1] == key) {
        print pair[2];
        exit;
      }
    }
  }' <<<"$line"
}

is_decimal() {
  [[ "$1" =~ ^[0-9]+([.][0-9]+)?$ ]]
}

echo "[bench] build binaries..."
if [[ "$BUILD_PROFILE" == "release" ]]; then
  cargo build --release --manifest-path "$ROOT_DIR/apps/chatminald/Cargo.toml" >/dev/null
  cargo build --release --manifest-path "$ROOT_DIR/apps/chatminal-app/Cargo.toml" >/dev/null
  DAEMON_BIN="$ROOT_DIR/target/release/chatminald"
  APP_BIN="$ROOT_DIR/target/release/chatminal-app"
else
  cargo build --manifest-path "$ROOT_DIR/apps/chatminald/Cargo.toml" >/dev/null
  cargo build --manifest-path "$ROOT_DIR/apps/chatminal-app/Cargo.toml" >/dev/null
  DAEMON_BIN="$ROOT_DIR/target/debug/chatminald"
  APP_BIN="$ROOT_DIR/target/debug/chatminal-app"
fi

echo "[bench] start daemon..."
CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" CHATMINAL_DEFAULT_SHELL="$BENCH_SHELL" "$DAEMON_BIN" >"$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

ready=0
for _ in $(seq 1 100); do
  if [[ -S "$SOCKET" ]] && CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" "$APP_BIN" workspace >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 0.1
done

if [[ "$ready" -ne 1 ]]; then
  echo "[bench] daemon did not become ready"
  tail -n 120 "$DAEMON_LOG" || true
  exit 1
fi

echo "[bench] run RTT benchmark..."
TIMEOUT_BIN="${CHATMINAL_TIMEOUT_BIN:-}"
if [[ -z "$TIMEOUT_BIN" ]]; then
  if command -v timeout >/dev/null 2>&1; then
    TIMEOUT_BIN="timeout"
  elif command -v gtimeout >/dev/null 2>&1; then
    TIMEOUT_BIN="gtimeout"
  fi
fi

if [[ -n "$TIMEOUT_BIN" ]]; then
  "$TIMEOUT_BIN" "${BENCH_MAX_SECONDS}s" env CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" \
    "$APP_BIN" bench-rtt "$SAMPLES" "$WARMUP" "$TIMEOUT_MS" "$COLS" "$ROWS" >"$APP_LOG" 2>&1 &
else
  CHATMINAL_DAEMON_ENDPOINT="$SOCKET" CHATMINAL_DATA_DIR="$DATA_DIR" \
    "$APP_BIN" bench-rtt "$SAMPLES" "$WARMUP" "$TIMEOUT_MS" "$COLS" "$ROWS" >"$APP_LOG" 2>&1 &
fi
APP_PID=$!

daemon_peak_kb=0
app_peak_kb=0
total_peak_kb=0
timed_out=0
start_epoch="$(date +%s)"

while kill -0 "$APP_PID" 2>/dev/null; do
  now_epoch="$(date +%s)"
  if (( now_epoch - start_epoch >= BENCH_MAX_SECONDS )); then
    timed_out=1
    kill "$APP_PID" 2>/dev/null || true
    break
  fi

  daemon_rss="$(rss_tree_kb "$DAEMON_PID")"
  app_rss="$(rss_tree_kb "$APP_PID")"
  total_rss=$((daemon_rss + app_rss))

  (( daemon_rss > daemon_peak_kb )) && daemon_peak_kb="$daemon_rss"
  (( app_rss > app_peak_kb )) && app_peak_kb="$app_rss"
  (( total_rss > total_peak_kb )) && total_peak_kb="$total_rss"
  sleep "$SAMPLE_INTERVAL_SECONDS"
done

set +e
wait "$APP_PID"
APP_EXIT="$?"
set -e
APP_PID=""

if [[ "$timed_out" -eq 1 ]]; then
  echo "[bench] benchmark exceeded CHATMINAL_BENCH_MAX_SECONDS=$BENCH_MAX_SECONDS"
  tail -n 120 "$APP_LOG" || true
  exit 1
fi

if [[ "$APP_EXIT" -ne 0 ]]; then
  echo "[bench] app benchmark failed (exit=$APP_EXIT)"
  tail -n 120 "$APP_LOG" || true
  exit 1
fi

summary_line="$(grep '^RTT_BENCH ' "$APP_LOG" | tail -n 1 || true)"
if [[ -z "$summary_line" ]]; then
  echo "[bench] missing RTT_BENCH summary output"
  tail -n 120 "$APP_LOG" || true
  exit 1
fi

p95_ms="$(extract_field "$summary_line" "p95_ms")"
p99_ms="$(extract_field "$summary_line" "p99_ms")"
pass_targets="$(extract_field "$summary_line" "pass_targets")"
pass_fail_gate="$(extract_field "$summary_line" "pass_fail_gate")"

if [[ -z "$p95_ms" || -z "$p99_ms" || -z "$pass_targets" || -z "$pass_fail_gate" ]]; then
  echo "[bench] summary missing required fields"
  echo "[bench] summary: $summary_line"
  exit 1
fi

if ! is_decimal "$p95_ms" || ! is_decimal "$p99_ms"; then
  echo "[bench] invalid numeric fields in summary"
  echo "[bench] p95_ms='$p95_ms' p99_ms='$p99_ms'"
  exit 1
fi

if [[ "$pass_targets" != "true" && "$pass_targets" != "false" ]]; then
  echo "[bench] invalid pass_targets value: '$pass_targets'"
  exit 1
fi
if [[ "$pass_fail_gate" != "true" && "$pass_fail_gate" != "false" ]]; then
  echo "[bench] invalid pass_fail_gate value: '$pass_fail_gate'"
  exit 1
fi

daemon_peak_mb="$(awk -v value="$daemon_peak_kb" 'BEGIN { printf "%.1f", value / 1024.0 }')"
app_peak_mb="$(awk -v value="$app_peak_kb" 'BEGIN { printf "%.1f", value / 1024.0 }')"
total_peak_mb="$(awk -v value="$total_peak_kb" 'BEGIN { printf "%.1f", value / 1024.0 }')"

echo "[bench] summary: $summary_line"
echo "[bench] daemon_peak_mb=$daemon_peak_mb app_peak_mb=$app_peak_mb total_peak_mb=$total_peak_mb"

hard_fail=0
if [[ "$pass_fail_gate" != "true" ]]; then
  echo "[bench][FAIL] app-level fail gate reported pass_fail_gate=$pass_fail_gate"
  hard_fail=1
fi
if awk -v value="$p95_ms" -v gate="$P95_HARD_FAIL_MS" 'BEGIN { exit !(value > gate) }'; then
  echo "[bench][FAIL] p95_ms=$p95_ms > $P95_HARD_FAIL_MS"
  hard_fail=1
fi
if (( daemon_peak_kb > DAEMON_HARD_FAIL_KB )); then
  echo "[bench][FAIL] daemon RSS ${daemon_peak_mb}MB > 160MB"
  hard_fail=1
fi
if (( app_peak_kb > APP_HARD_FAIL_KB )); then
  echo "[bench][FAIL] app RSS ${app_peak_mb}MB > 220MB"
  hard_fail=1
fi
if (( total_peak_kb > TOTAL_HARD_FAIL_KB )); then
  echo "[bench][FAIL] total RSS ${total_peak_mb}MB > 350MB"
  hard_fail=1
fi

echo "[bench] targets: pass_targets=$pass_targets pass_fail_gate=$pass_fail_gate p99_ms=$p99_ms"
echo "[bench] logs: daemon=$DAEMON_LOG app=$APP_LOG profile=$BUILD_PROFILE"

if [[ "$hard_fail" -ne 0 ]] && [[ "$ENFORCE_HARD_GATE" == "1" ]]; then
  exit 1
fi

if [[ "$hard_fail" -ne 0 ]]; then
  echo "[bench] SOFT-FAIL (hard gates bypassed by CHATMINAL_BENCH_ENFORCE_HARD_GATE=$ENFORCE_HARD_GATE)"
  exit 0
fi

echo "[bench] PASS (hard gates)"
