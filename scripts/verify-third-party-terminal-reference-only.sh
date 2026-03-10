#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

check_no_root_path_deps() {
  local matches
  matches="$(rg -n 'path\s*=\s*".*third_party/terminal-engine-reference' Cargo.toml apps crates -g 'Cargo.toml' || true)"
  if [[ -n "$matches" ]]; then
    echo "third_party/terminal-engine-reference reference-only check failed: found active Cargo path dependency" >&2
    printf '%s\n' "$matches" >&2
    exit 1
  fi
}

check_no_runtime_shell_refs() {
  local matches
  matches="$(rg -n 'manifest-path third_party/terminal-engine-reference|cd third_party/terminal-engine-reference|cargo .*(third_party/terminal-engine-reference)' Makefile scripts apps crates \
    -g 'Makefile' \
    -g '*.sh' \
    -g '*.rs' \
    -g '!scripts/verify-third-party-terminal-reference-only.sh' \
    || true)"
  if [[ -n "$matches" ]]; then
    echo "third_party/terminal-engine-reference reference-only check failed: found active runtime/build shell reference" >&2
    printf '%s\n' "$matches" >&2
    exit 1
  fi
}

check_no_root_path_deps
check_no_runtime_shell_refs

echo "third_party/terminal-engine-reference is reference-only for active Chatminal build/runtime"
