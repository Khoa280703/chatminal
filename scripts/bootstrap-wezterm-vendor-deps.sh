#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDOR_DIR="${CHATMINAL_WEZTERM_VENDOR_DEPS_DIR:-$ROOT_DIR/vendor/wezterm-deps}"
QUIET=0

for arg in "$@"; do
  case "$arg" in
    --quiet) QUIET=1 ;;
    *)
      echo "Usage: $(basename "$0") [--quiet]" >&2
      exit 1
      ;;
  esac
done

log() {
  if [[ "$QUIET" -eq 0 ]]; then
    printf '%s\n' "$*"
  fi
}

ensure_checkout() {
  local name="$1"
  local rel_dir="$2"
  local sentinel="$3"
  local url="$4"
  local target_dir="$VENDOR_DIR/$rel_dir"
  local sentinel_path="$target_dir/$sentinel"

  if [[ -f "$sentinel_path" ]]; then
    log "chatminal wezterm vendor dep ready: $name"
    return 0
  fi

  if [[ -d "$target_dir" ]] && find "$target_dir" -mindepth 1 -maxdepth 1 | read -r _; then
    echo "chatminal wezterm vendor dep incomplete: $target_dir (missing $sentinel)" >&2
    exit 1
  fi

  log "hydrating chatminal wezterm vendor dep: $name"
  rm -rf "$target_dir"
  mkdir -p "$(dirname "$target_dir")"

  local temp_dir
  temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/chatminal-wezterm-dep-XXXXXX")"
  git clone --depth 1 "$url" "$temp_dir" >/dev/null 2>&1
  rm -rf "$temp_dir/.git"
  mv "$temp_dir" "$target_dir"

  if [[ ! -f "$sentinel_path" ]]; then
    echo "chatminal wezterm vendor dep hydrate failed: $name (missing $sentinel)" >&2
    exit 1
  fi
}

ensure_checkout "zlib" "freetype/zlib" "adler32.c" "https://github.com/madler/zlib.git"
ensure_checkout "libpng" "freetype/libpng" "png.c" "https://github.com/glennrp/libpng.git"
ensure_checkout "freetype2" "freetype/freetype2" "include/freetype/freetype.h" "https://github.com/freetype/freetype2.git"
ensure_checkout "harfbuzz" "harfbuzz/harfbuzz" "src/harfbuzz.cc" "https://github.com/harfbuzz/harfbuzz.git"
