#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUN_ID="${RUN_ID:-$$}"
ENDPOINT="${CHATMINAL_DAEMON_ENDPOINT:-/tmp/chatminald-phase03-fidelity-${RUN_ID}.sock}"
DATA_DIR="${CHATMINAL_DATA_DIR:-/tmp/chatminal-phase03-fidelity-${RUN_ID}}"
REPORT_PATH="${CHATMINAL_FIDELITY_MATRIX_REPORT:-/tmp/chatminal-phase03-fidelity-matrix-report-${RUN_ID}.json}"
PREVIEW_LINES="${CHATMINAL_PREVIEW_LINES:-1200}"
PROFILE="${CHATMINAL_PROFILE:-dev}"
POLL_TIMEOUT_SECONDS="${CHATMINAL_FIDELITY_TIMEOUT_SECONDS:-10}"
POLL_INTERVAL_SECONDS="${CHATMINAL_FIDELITY_POLL_INTERVAL_SECONDS:-0.1}"
STRICT_MODE="${CHATMINAL_FIDELITY_STRICT:-1}"
TIMEOUT_BIN="${CHATMINAL_TIMEOUT_BIN:-}"
REQUIRED_CASES="${CHATMINAL_FIDELITY_REQUIRED_CASES:-bash-prompt,ctrl-c,ctrl-c-burst,ctrl-z,unicode,stress-paste,resize,reconnect}"
CONTROL_SIGNAL_DELAY_SECONDS="${CHATMINAL_FIDELITY_CONTROL_SIGNAL_DELAY_SECONDS:-0.12}"

APP_ARGS=(run --manifest-path apps/chatminal-app/Cargo.toml --)
DAEMON_ARGS=(run --manifest-path apps/chatminald/Cargo.toml)
if [[ "$PROFILE" == "release" ]]; then
  APP_ARGS=(run --release --manifest-path apps/chatminal-app/Cargo.toml --)
  DAEMON_ARGS=(run --release --manifest-path apps/chatminald/Cargo.toml)
fi

TMP_DIR="$(mktemp -d "/tmp/chatminal-phase03-fidelity.${RUN_ID}.XXXXXX")"
DAEMON_LOG="$TMP_DIR/daemon.log"
CREATE_JSON="$TMP_DIR/create.json"
ACTIVATE_JSON="$TMP_DIR/activate.json"

status="failed"
failed_step="bootstrap"
failure_reason=""
session_id=""
overall_fail_count=0
skip_count=0
required_skip_count=0
checks_json=""

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  printf '%s' "$value"
}

append_check() {
  local id="$1"
  local case_status="$2"
  local pass="$3"
  local note="$4"
  local escaped_note
  escaped_note="$(json_escape "$note")"
  if [[ -n "$checks_json" ]]; then
    checks_json+=","
  fi
  checks_json+=$'\n'
  checks_json+="    {\"id\":\"${id}\",\"status\":\"${case_status}\",\"pass\":${pass},\"note\":\"${escaped_note}\"}"
}

is_required_case() {
  local id="$1"
  local list=",${REQUIRED_CASES},"
  [[ "$list" == *",$id,"* ]]
}

cleanup() {
  if [[ -n "${DAEMON_PID:-}" ]]; then
    kill "$DAEMON_PID" >/dev/null 2>&1 || true
    wait "$DAEMON_PID" >/dev/null 2>&1 || true
  fi
  rm -f "$ENDPOINT"
}

write_report() {
  local target_path="$REPORT_PATH"
  local fallback_path="/tmp/chatminal-phase03-fidelity-matrix-report-${RUN_ID}.json"
  local wrote=false

  for candidate in "$target_path" "$fallback_path"; do
    mkdir -p "$(dirname "$candidate")" >/dev/null 2>&1 || true
    set +e
    (
      cat >"$candidate" <<JSON
{
  "type": "phase03_fidelity_matrix_smoke",
  "timestamp_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "$status",
  "failed_step": "$failed_step",
  "failure_reason": "$(json_escape "$failure_reason")",
  "endpoint": "$(json_escape "$ENDPOINT")",
  "data_dir": "$(json_escape "$DATA_DIR")",
	  "session_id": "$(json_escape "$session_id")",
	  "checks": [${checks_json}
	  ],
	  "fail_count": $overall_fail_count,
	  "skip_count": $skip_count,
	  "required_skip_count": $required_skip_count,
	  "required_cases": "$(json_escape "$REQUIRED_CASES")",
	  "artifacts": {
    "tmp_dir": "$TMP_DIR",
    "daemon_log": "$DAEMON_LOG",
    "create_json": "$CREATE_JSON",
    "activate_json": "$ACTIVATE_JSON"
  }
}
JSON
    ) >/dev/null 2>&1
    local write_code=$?
    set -e
    if [[ $write_code -eq 0 ]]; then
      REPORT_PATH="$candidate"
      wrote=true
      break
    fi
  done

  if [[ "$wrote" != "true" ]]; then
    echo "error: failed to write fidelity matrix report to '$target_path' and fallback '$fallback_path'" >&2
    return 1
  fi
  return 0
}

finalize() {
  local exit_code=$?
  if [[ $exit_code -eq 0 ]]; then
    if [[ "$status" == "failed" ]]; then
      status="passed"
      failed_step=""
      failure_reason=""
    fi
  elif [[ -z "$failure_reason" ]]; then
    failure_reason="script exited with code ${exit_code}"
  fi
  cleanup
  if ! write_report && [[ $exit_code -eq 0 ]]; then
    exit_code=1
    failed_step="write_report"
    failure_reason="failed to write fidelity matrix report artifact"
  fi
  if [[ $exit_code -eq 0 ]]; then
    echo "phase03 fidelity matrix report: $REPORT_PATH"
  else
    echo "phase03 fidelity matrix failed at '$failed_step': $failure_reason" >&2
    echo "phase03 fidelity matrix report: $REPORT_PATH" >&2
  fi
  return "$exit_code"
}
trap finalize EXIT

fail_now() {
  failed_step="$1"
  failure_reason="$2"
  exit 1
}

resolve_timeout_bin() {
  if [[ -n "$TIMEOUT_BIN" ]]; then
    return
  fi
  if command -v timeout >/dev/null 2>&1; then
    TIMEOUT_BIN="timeout"
    return
  fi
  if command -v gtimeout >/dev/null 2>&1; then
    TIMEOUT_BIN="gtimeout"
    return
  fi
}

run_app() {
  (
    cd "$ROOT_DIR"
    CHATMINAL_DAEMON_ENDPOINT="$ENDPOINT" CHATMINAL_DATA_DIR="$DATA_DIR" cargo "${APP_ARGS[@]}" "$@"
  )
}

wait_for_marker_with_limit() {
  local id="$1"
  local marker_pass="$2"
  local marker_fail="$3"
  local marker_skip="$4"
  local pass_deadline_seconds="${5:-}"
  local snapshot_file="$TMP_DIR/snapshot-${id}.json"
  local snapshot_err="$TMP_DIR/snapshot-${id}.err"
  local start_ts
  start_ts="$(date +%s)"

  while true; do
    if run_app snapshot "$session_id" "$PREVIEW_LINES" >"$snapshot_file" 2>"$snapshot_err"; then
      if rg -q "$marker_pass" "$snapshot_file"; then
        local now_ts
        now_ts="$(date +%s)"
        local elapsed_seconds=$((now_ts - start_ts))
        if [[ -n "$pass_deadline_seconds" ]] && (( elapsed_seconds > pass_deadline_seconds )); then
          append_check "$id" "fail" "false" "marker found after ${elapsed_seconds}s (deadline ${pass_deadline_seconds}s)"
          overall_fail_count=$((overall_fail_count + 1))
          return 0
        fi
        append_check "$id" "pass" "true" "marker found: pass"
        return 0
      fi
      if rg -q "$marker_skip" "$snapshot_file"; then
        append_check "$id" "skip" "true" "tool not installed on host"
        skip_count=$((skip_count + 1))
        if is_required_case "$id"; then
          required_skip_count=$((required_skip_count + 1))
        fi
        return 0
      fi
      if rg -q "$marker_fail" "$snapshot_file"; then
        append_check "$id" "fail" "false" "command returned failure marker"
        overall_fail_count=$((overall_fail_count + 1))
        return 0
      fi
    fi

    local now_ts
    now_ts="$(date +%s)"
    if (( now_ts - start_ts >= POLL_TIMEOUT_SECONDS )); then
      append_check "$id" "fail" "false" "timeout waiting marker"
      overall_fail_count=$((overall_fail_count + 1))
      return 0
    fi
    sleep "$POLL_INTERVAL_SECONDS"
  done
}

wait_for_marker() {
  wait_for_marker_with_limit "$1" "$2" "$3" "$4"
}

run_case_payload() {
  local id="$1"
  local payload="$2"
  local marker_key
  marker_key="$(echo "$id" | tr '[:lower:]-' '[:upper:]_')"
  local marker_pass="__CHM_${marker_key}_PASS__"
  local marker_fail="__CHM_${marker_key}_FAIL__"
  local marker_skip="__CHM_${marker_key}_SKIP__"
  local input_json="$TMP_DIR/input-${id}.json"

  if ! run_app input "$session_id" "$payload" >"$input_json"; then
    append_check "$id" "fail" "false" "input command failed"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi

  wait_for_marker "$id" "$marker_pass" "$marker_fail" "$marker_skip"
}

run_case_ctrl_c() {
  local id="ctrl-c"
  local marker_pass="__CHM_CTRL_C_PASS__"
  local marker_fail="__CHM_CTRL_C_FAIL__"
  local marker_skip="__CHM_CTRL_C_SKIP__"

  if ! run_app input "$session_id" $'cat >/dev/null\n' >"$TMP_DIR/input-${id}-start.json"; then
    append_check "$id" "fail" "false" "failed to start foreground cat process"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi
  sleep "$CONTROL_SIGNAL_DELAY_SECONDS"
  if ! run_app input "$session_id" $'\003' >"$TMP_DIR/input-${id}-sigint.json"; then
    append_check "$id" "fail" "false" "failed to send ctrl-c signal"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi
  sleep "$CONTROL_SIGNAL_DELAY_SECONDS"
  if ! run_app input "$session_id" $'printf "__CHM_CTRL_C_%s__\\n" "PASS"\n' >"$TMP_DIR/input-${id}-marker.json"; then
    append_check "$id" "fail" "false" "failed to send ctrl-c marker command"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi

  wait_for_marker "$id" "$marker_pass" "$marker_fail" "$marker_skip"
}

run_case_ctrl_c_burst() {
  local id="ctrl-c-burst"
  local marker_pass="__CHM_CTRL_C_BURST_PASS__"
  local marker_fail="__CHM_CTRL_C_BURST_FAIL__"
  local marker_skip="__CHM_CTRL_C_BURST_SKIP__"

  if ! run_app input "$session_id" $'cat >/dev/null\n' >"$TMP_DIR/input-${id}-start.json"; then
    append_check "$id" "fail" "false" "failed to start foreground cat process"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi
  sleep "$CONTROL_SIGNAL_DELAY_SECONDS"

  for idx in {1..6}; do
    if ! run_app input "$session_id" $'\003' >"$TMP_DIR/input-${id}-sigint-${idx}.json"; then
      append_check "$id" "fail" "false" "failed to send ctrl-c burst signal #${idx}"
      overall_fail_count=$((overall_fail_count + 1))
      return
    fi
    sleep 0.03
  done

  if ! run_app input "$session_id" $'printf "__CHM_CTRL_C_BURST_%s__\\n" "PASS"\n' >"$TMP_DIR/input-${id}-marker.json"; then
    append_check "$id" "fail" "false" "failed to send ctrl-c burst marker command"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi

  wait_for_marker "$id" "$marker_pass" "$marker_fail" "$marker_skip"
}

run_case_stress_paste() {
  local id="stress-paste"
  local marker_pass="__CHM_STRESS_PASTE_PASS__"
  local marker_fail="__CHM_STRESS_PASTE_FAIL__"
  local marker_skip="__CHM_STRESS_PASTE_SKIP__"
  local payload=$'if command -v seq >/dev/null 2>&1; then\n  for i in $(seq 1 400); do\n    printf "chm-stress-line-%04d\\n" "$i"\n  done\n  printf "__CHM_STRESS_PASTE_%s__\\n" "PASS"\nelse\n  printf "__CHM_STRESS_PASTE_%s__\\n" "SKIP"\nfi\n'

  if ! run_app input "$session_id" "$payload" >"$TMP_DIR/input-${id}.json"; then
    append_check "$id" "fail" "false" "failed to send stress payload"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi

  wait_for_marker "$id" "$marker_pass" "$marker_fail" "$marker_skip"
}

run_case_ctrl_z() {
  local id="ctrl-z"
  local marker_pass="__CHM_CTRL_Z_PASS__"
  local marker_fail="__CHM_CTRL_Z_FAIL__"
  local marker_skip="__CHM_CTRL_Z_SKIP__"

  if ! run_app input "$session_id" $'sleep 30\n' >"$TMP_DIR/input-${id}-start.json"; then
    append_check "$id" "fail" "false" "failed to start foreground sleep process"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi
  sleep "$CONTROL_SIGNAL_DELAY_SECONDS"
  if ! run_app input "$session_id" $'\032' >"$TMP_DIR/input-${id}-sigtstp.json"; then
    append_check "$id" "fail" "false" "failed to send ctrl-z signal"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi
  sleep "$CONTROL_SIGNAL_DELAY_SECONDS"

  local payload=$'if jobs | grep -q "Stopped"; then\n  printf "__CHM_CTRL_Z_%s__\\n" "PASS"\nelse\n  printf "__CHM_CTRL_Z_%s__\\n" "FAIL"\nfi\nkill %1 >/dev/null 2>&1 || true\n'
  if ! run_app input "$session_id" "$payload" >"$TMP_DIR/input-${id}-marker.json"; then
    append_check "$id" "fail" "false" "failed to send ctrl-z marker command"
    overall_fail_count=$((overall_fail_count + 1))
    return
  fi

  wait_for_marker "$id" "$marker_pass" "$marker_fail" "$marker_skip"
}

mk_payload_timeout_tui() {
  local binary="$1"
  local run_cmd="$2"
  local marker="$3"
  if [[ -z "$TIMEOUT_BIN" ]]; then
    cat <<EOF
printf "__CHM_${marker}_%s__\\n" "SKIP"
EOF
    return
  fi
  cat <<EOF
if command -v ${binary} >/dev/null 2>&1; then
  ${TIMEOUT_BIN} 1 bash -lc '${run_cmd}' >/dev/tty 2>/dev/tty < /dev/tty
  rc=\$?
  if [[ \$rc -eq 0 || \$rc -eq 124 ]]; then
    printf "__CHM_${marker}_%s__\\n" "PASS"
  else
    printf "__CHM_${marker}_%s__\\n" "FAIL"
  fi
else
  printf "__CHM_${marker}_%s__\\n" "SKIP"
fi
EOF
}

mk_payload_version_check() {
  local binary="$1"
  local marker="$2"
  cat <<EOF
if command -v ${binary} >/dev/null 2>&1; then
  ${binary} --version >/dev/null 2>&1
  rc=\$?
  if [[ \$rc -eq 0 ]]; then
    printf "__CHM_${marker}_%s__\\n" "PASS"
  else
    printf "__CHM_${marker}_%s__\\n" "FAIL"
  fi
else
  printf "__CHM_${marker}_%s__\\n" "SKIP"
fi
EOF
}

mkdir -p "$DATA_DIR"
rm -f "$ENDPOINT"
resolve_timeout_bin

failed_step="daemon_boot"
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

failed_step="create_session"
if ! run_app create phase03-fidelity-matrix >"$CREATE_JSON"; then
  fail_now "create_session" "create session failed"
fi
session_id="$(rg -o '"session_id":\s*"[^"]+"' "$CREATE_JSON" | head -n 1 | sed -E 's/.*"([^"]+)"/\1/')"
if [[ -z "$session_id" ]]; then
  fail_now "parse_session_id" "failed to parse session_id"
fi

failed_step="activate_session"
if ! run_app activate "$session_id" 120 32 "$PREVIEW_LINES" >"$ACTIVATE_JSON"; then
  fail_now "activate_session" "activate session failed"
fi

run_case_payload "bash-prompt" $'printf "__CHM_BASH_PROMPT_%s__\\n" "PASS"\n'
run_case_ctrl_c
run_case_ctrl_c_burst
run_case_ctrl_z
run_case_payload "alt-backspace" $'\033\177printf "__CHM_ALT_BACKSPACE_%s__\\n" "PASS"\n'
run_case_payload "meta-shortcuts-macos" $'\033b\033fprintf "__CHM_META_SHORTCUTS_MACOS_%s__\\n" "PASS"\n'

run_case_payload "vim" "$(mk_payload_timeout_tui "vim" "vim -Nu NONE -n +qall!" "VIM")"$'\n'
run_case_payload "nvim" "$(mk_payload_timeout_tui "nvim" "nvim +qall!" "NVIM")"$'\n'
run_case_payload "tmux" "$(cat <<'EOF'
if command -v tmux >/dev/null 2>&1; then
  tmux -L chatminal-smoke -f /dev/null new-session -d 'exit 0' >/dev/null 2>&1
  rc=$?
  tmux -L chatminal-smoke kill-server >/dev/null 2>&1 || true
  if [[ $rc -eq 0 ]]; then
    printf "__CHM_TMUX_%s__\n" "PASS"
  else
    printf "__CHM_TMUX_%s__\n" "FAIL"
  fi
else
  printf "__CHM_TMUX_%s__\n" "SKIP"
fi
EOF
)"$'\n'
run_case_payload "htop" "$(mk_payload_version_check "htop" "HTOP")"$'\n'
run_case_payload "btop" "$(mk_payload_version_check "btop" "BTOP")"$'\n'
run_case_payload "lazygit" "$(mk_payload_version_check "lazygit" "LAZYGIT")"$'\n'

if [[ -n "$TIMEOUT_BIN" ]]; then
  run_case_payload "fzf" $'if command -v fzf >/dev/null 2>&1; then\n  printf "a\\nb\\n" | '"${TIMEOUT_BIN}"$' 1 fzf --filter a >/dev/null 2>&1\n  rc=$?\n  if [[ $rc -eq 0 || $rc -eq 124 ]]; then\n    printf "__CHM_FZF_%s__\\n" "PASS"\n  else\n    printf "__CHM_FZF_%s__\\n" "FAIL"\n  fi\nelse\n  printf "__CHM_FZF_%s__\\n" "SKIP"\nfi\n'
else
  run_case_payload "fzf" $'printf "__CHM_FZF_%s__\\n" "SKIP"\n'
fi

run_case_payload "unicode" $'printf "Tiếng Việt: ă â ê ô ơ ư đ\\n"\nprintf "__CHM_UNICODE_%s__\\n" "PASS"\n'

failed_step="resize_session"
if run_app resize "$session_id" 140 40 >"$TMP_DIR/resize.json"; then
  run_case_payload "resize" $'printf "__CHM_RESIZE_%s__\\n" "PASS"\n'
else
  append_check "resize" "fail" "false" "resize command failed"
  overall_fail_count=$((overall_fail_count + 1))
fi

failed_step="reconnect_session"
if run_app activate "$session_id" 120 32 "$PREVIEW_LINES" >"$TMP_DIR/reconnect-activate.json"; then
  run_case_payload "reconnect" $'printf "__CHM_RECONNECT_%s__\\n" "PASS"\n'
else
  append_check "reconnect" "fail" "false" "re-activate session failed"
  overall_fail_count=$((overall_fail_count + 1))
fi

run_case_stress_paste

if (( overall_fail_count > 0 )); then
  if [[ "$STRICT_MODE" == "1" ]]; then
    fail_now "matrix_checks" "one or more matrix checks failed"
  fi
  status="passed_with_warnings"
  failed_step=""
  failure_reason="non-strict mode: matrix has warnings (${overall_fail_count} failed checks)"
fi

if (( required_skip_count > 0 )); then
  if [[ "$STRICT_MODE" == "1" ]]; then
    fail_now "matrix_coverage" "required fidelity cases were skipped"
  fi
  status="passed_with_warnings"
  failed_step=""
  failure_reason="non-strict mode: required fidelity cases skipped (${required_skip_count})"
fi
