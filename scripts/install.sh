#!/usr/bin/env bash
set -euo pipefail

REPO_SLUG="${CHATMINAL_REPO:-Khoa280703/chatminal}"
BIN_DIR="${CHATMINAL_BIN_DIR:-$HOME/.local/bin}"
INSTALL_ROOT="${CHATMINAL_INSTALL_DIR:-$HOME/.local/share/chatminal}"
MACOS_APP_DIR="${CHATMINAL_APP_DIR:-$HOME/Applications}"
DRY_RUN="${CHATMINAL_INSTALL_DRY_RUN:-0}"
BOOTSTRAPPED="${CHATMINAL_INSTALL_BOOTSTRAPPED:-0}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

say() {
  printf '==> %s\n' "$1"
}

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

resolve_version() {
  if [[ -n "${CHATMINAL_VERSION:-}" ]]; then
    printf '%s\n' "$CHATMINAL_VERSION"
    return
  fi

  curl -fsSL "https://api.github.com/repos/${REPO_SLUG}/releases/latest" \
    | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1
}

normalize_os() {
  case "$(uname -s)" in
    Darwin) printf 'macos\n' ;;
    Linux) printf 'linux\n' ;;
    *) fail "unsupported operating system: $(uname -s)" ;;
  esac
}

normalize_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf 'x86_64\n' ;;
    arm64|aarch64) printf 'aarch64\n' ;;
    *) fail "unsupported architecture: $(uname -m)" ;;
  esac
}

assert_supported_target() {
  local os="$1"
  local arch="$2"

  case "$os/$arch" in
    macos/aarch64|macos/x86_64|linux/x86_64) ;;
    linux/*) fail "curl|bash installer currently supports linux/x86_64 only" ;;
    *) fail "curl|bash installer does not support $os/$arch yet" ;;
  esac
}

compute_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
    return
  fi
  fail "missing sha256 tool (need sha256sum or shasum)"
}

verify_checksum() {
  local file="$1"
  local checksums="$2"
  local asset_name
  local expected
  local actual

  asset_name="$(basename "$file")"
  expected="$(awk -v name="$asset_name" '$2 == name { print $1 }' "$checksums")"
  [[ -n "$expected" ]] || fail "checksum not found for $asset_name"

  actual="$(compute_sha256 "$file")"
  [[ "$actual" == "$expected" ]] || fail "checksum mismatch for $asset_name"
}

install_linux() {
  local archive="$1"
  local version="$2"
  local target_dir="$INSTALL_ROOT/$version"
  local current_link="$INSTALL_ROOT/current"
  local desktop_src="$target_dir/chatminal.desktop"
  local desktop_dst="$HOME/.local/share/applications/chatminal.desktop"
  local icon_src="$target_dir/share/icons/hicolor/256x256/apps/chatminal.png"
  local icon_dst="$HOME/.local/share/icons/hicolor/256x256/apps/chatminal.png"

  say "Installing Linux archive into $target_dir"
  mkdir -p "$INSTALL_ROOT" "$BIN_DIR"
  rm -rf "$target_dir"
  mkdir -p "$target_dir"
  tar -xzf "$archive" -C "$target_dir" --strip-components=1

  ln -sfn "$target_dir" "$current_link"
  ln -sfn "$current_link/bin/chatminal" "$BIN_DIR/chatminal"

  if [[ -f "$desktop_src" ]]; then
    mkdir -p "$(dirname "$desktop_dst")"
    sed "s|^Exec=.*$|Exec=$BIN_DIR/chatminal|" "$desktop_src" >"$desktop_dst"
  fi

  if [[ -f "$icon_src" ]]; then
    mkdir -p "$(dirname "$icon_dst")"
    cp "$icon_src" "$icon_dst"
  fi

  say "Installed chatminal to $BIN_DIR/chatminal"
}

install_macos() {
  local archive="$1"
  local version="$2"
  local staging_dir="$TMP_DIR/macos"
  local app_path="$MACOS_APP_DIR/Chatminal.app"
  local version_dir="$INSTALL_ROOT/$version"
  local current_link="$INSTALL_ROOT/current"

  say "Installing macOS app into $app_path"
  mkdir -p "$MACOS_APP_DIR" "$INSTALL_ROOT" "$BIN_DIR" "$staging_dir"
  tar -xzf "$archive" -C "$staging_dir"

  rm -rf "$app_path"
  mv "$staging_dir/Chatminal.app" "$app_path"

  rm -rf "$version_dir"
  mkdir -p "$version_dir"
  ln -sfn "$app_path" "$version_dir/Chatminal.app"
  ln -sfn "$version_dir" "$current_link"
  ln -sfn "$app_path/Contents/MacOS/chatminal-desktop" "$BIN_DIR/chatminal"

  say "Installed Chatminal.app to $MACOS_APP_DIR"
  say "Installed chatminal launcher to $BIN_DIR/chatminal"
}

bootstrap_release_installer() {
  local version="$1"
  local release_url="$2"
  local release_installer_path="$TMP_DIR/install.sh"

  curl -fsSL "$release_url/install.sh" -o "$release_installer_path"
  chmod +x "$release_installer_path"

  env \
    CHATMINAL_REPO="$REPO_SLUG" \
    CHATMINAL_VERSION="$version" \
    CHATMINAL_BIN_DIR="$BIN_DIR" \
    CHATMINAL_INSTALL_DIR="$INSTALL_ROOT" \
    CHATMINAL_APP_DIR="$MACOS_APP_DIR" \
    CHATMINAL_INSTALL_DRY_RUN="$DRY_RUN" \
    CHATMINAL_INSTALL_BOOTSTRAPPED=1 \
    bash "$release_installer_path"
}

main() {
  local version
  local os
  local arch
  local asset_name
  local release_url
  local archive_path
  local checksums_path

  need_cmd curl
  need_cmd tar

  version="$(resolve_version)"
  [[ -n "$version" ]] || fail "failed to resolve release version"

  os="$(normalize_os)"
  arch="$(normalize_arch)"
  assert_supported_target "$os" "$arch"
  asset_name="Chatminal-${version}-${os}-${arch}.tar.gz"
  release_url="https://github.com/${REPO_SLUG}/releases/download/${version}"
  archive_path="$TMP_DIR/$asset_name"
  checksums_path="$TMP_DIR/SHA256SUMS"

  say "Preparing Chatminal $version for $os/$arch"
  if [[ "$DRY_RUN" == "1" ]]; then
    printf 'version=%s\nos=%s\narch=%s\nasset=%s\nurl=%s/%s\ninstaller=%s/install.sh\n' \
      "$version" "$os" "$arch" "$asset_name" "$release_url" "$asset_name" "$release_url"
    return
  fi

  if [[ "$BOOTSTRAPPED" != "1" ]]; then
    say "Loading release installer for $version"
    bootstrap_release_installer "$version" "$release_url"
    return
  fi

  say "Downloading $asset_name"
  curl -fL "$release_url/$asset_name" -o "$archive_path"
  curl -fsSL "$release_url/SHA256SUMS" -o "$checksums_path"
  verify_checksum "$archive_path" "$checksums_path"

  case "$os" in
    linux) install_linux "$archive_path" "$version" ;;
    macos) install_macos "$archive_path" "$version" ;;
  esac

  if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    printf '\nAdd %s to your PATH, for example:\n' "$BIN_DIR"
    printf '  export PATH="%s:$PATH"\n' "$BIN_DIR"
  fi

  printf '\nDone.\n'
}

main "$@"
