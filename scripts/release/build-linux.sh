#!/usr/bin/env bash
# Build Chatminal for Linux and produce a .tar.gz
# Usage: bash scripts/release/build-linux.sh [--skip-build]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="$REPO_ROOT/target/release"
DIST_DIR="$REPO_ROOT/dist"

VERSION="${CHATMINAL_VERSION:-$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')}"
RAW_ARCH="$(uname -m)"
case "$RAW_ARCH" in
    arm64) ARCH="aarch64" ;;
    amd64) ARCH="x86_64" ;;
    x86_64|aarch64) ARCH="$RAW_ARCH" ;;
    *) ARCH="$RAW_ARCH" ;;
esac
ARCHIVE_NAME="Chatminal-${VERSION}-linux-${ARCH}.tar.gz"
STAGING_DIR="$REPO_ROOT/target/linux-staging/chatminal-${VERSION}"

echo "==> Building Chatminal $VERSION for Linux ($ARCH)"

# ── 1. System native deps (CI path) ──────────────────────────────────────────
if command -v apt-get &>/dev/null && [[ "${CI:-}" == "true" ]]; then
    sudo apt-get update -qq
    sudo apt-get install -y -qq \
        libwayland-dev \
        libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
        libxcb-image0-dev libxcb-util0-dev \
        libxkbcommon-dev libxkbcommon-x11-dev \
        libx11-dev libx11-xcb-dev \
        libfontconfig1-dev libfreetype-dev \
        pkg-config
fi

# ── 2. Bootstrap native vendor deps ──────────────────────────────────────────
bash "$REPO_ROOT/scripts/bootstrap-terminal-vendor-deps.sh" --quiet

# ── 3. Cargo release build ────────────────────────────────────────────────────
if [[ "${1:-}" != "--skip-build" ]]; then
    cargo build --release --manifest-path "$REPO_ROOT/apps/desktop/Cargo.toml"
fi

# ── 4. Stage files ────────────────────────────────────────────────────────────
echo "==> Staging files"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR/bin"
cp "$TARGET_DIR/desktop" "$STAGING_DIR/bin/chatminal"
chmod +x "$STAGING_DIR/bin/chatminal"

# Desktop entry for application launchers
cat > "$STAGING_DIR/chatminal.desktop" << 'EOF'
[Desktop Entry]
Name=Chatminal
Comment=A modern terminal emulator
Exec=chatminal
Icon=chatminal
Type=Application
Categories=System;TerminalEmulator;
StartupNotify=false
EOF

# Copy icon if available
ICON_SRC="$REPO_ROOT/apps/desktop/assets/icon/terminal.png"
if [[ -f "$ICON_SRC" ]]; then
    mkdir -p "$STAGING_DIR/share/icons/hicolor/256x256/apps"
    cp "$ICON_SRC" "$STAGING_DIR/share/icons/hicolor/256x256/apps/chatminal.png"
fi

# ── 5. Package .tar.gz ────────────────────────────────────────────────────────
echo "==> Creating $ARCHIVE_NAME"
mkdir -p "$DIST_DIR"
tar -czf "$DIST_DIR/$ARCHIVE_NAME" \
    -C "$(dirname "$STAGING_DIR")" \
    "$(basename "$STAGING_DIR")"

rm -rf "$(dirname "$STAGING_DIR")"
echo "==> Done: $DIST_DIR/$ARCHIVE_NAME"
