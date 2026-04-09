#!/usr/bin/env bash
# Build Chatminal for macOS and produce a .dmg
# Usage: bash scripts/release/build-macos.sh [--skip-build]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ASSET_DIR="$REPO_ROOT/apps/desktop/assets/macos"
TARGET_DIR="$REPO_ROOT/target/release"
BUNDLE_NAME="Chatminal.app"
BUNDLE_OUT="$REPO_ROOT/target/$BUNDLE_NAME"
BINARY_NAME="chatminal-desktop"   # matches CFBundleExecutable in Info.plist
DIST_DIR="$REPO_ROOT/dist"

VERSION="${CHATMINAL_VERSION:-$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')}"
RAW_ARCH="$(uname -m)"
case "$RAW_ARCH" in
    arm64) ARCH="aarch64" ;;
    x86_64) ARCH="x86_64" ;;
    *) ARCH="$RAW_ARCH" ;;
esac
DMG_NAME="Chatminal-${VERSION}-macos-${ARCH}.dmg"
ARCHIVE_NAME="Chatminal-${VERSION}-macos-${ARCH}.tar.gz"

echo "==> Building Chatminal $VERSION for macOS"

# ── 1. Bootstrap native vendor deps ──────────────────────────────────────────
bash "$REPO_ROOT/scripts/bootstrap-terminal-vendor-deps.sh" --quiet

# ── 2. Cargo release build ────────────────────────────────────────────────────
if [[ "${1:-}" != "--skip-build" ]]; then
    cargo build --release --manifest-path "$REPO_ROOT/apps/desktop/Cargo.toml"
fi

# ── 3. Assemble .app bundle ───────────────────────────────────────────────────
echo "==> Assembling $BUNDLE_NAME"
rm -rf "$BUNDLE_OUT"
mkdir -p "$BUNDLE_OUT/Contents/MacOS"
mkdir -p "$BUNDLE_OUT/Contents/Resources"
mkdir -p "$BUNDLE_OUT/Contents/Frameworks"

# Binary (rename cargo output to match CFBundleExecutable)
cp "$TARGET_DIR/desktop" "$BUNDLE_OUT/Contents/MacOS/$BINARY_NAME"
chmod +x "$BUNDLE_OUT/Contents/MacOS/$BINARY_NAME"

# Info.plist
cp "$ASSET_DIR/Chatminal.app/Contents/Info.plist" "$BUNDLE_OUT/Contents/Info.plist"

# App icon
cp "$ASSET_DIR/Chatminal.app/Contents/Resources/terminal.icns" "$BUNDLE_OUT/Contents/Resources/terminal.icns"

# ANGLE / EGL dylibs — must be alongside binary (macOS rpath @executable_path)
for dylib in libEGL.dylib libGLESv2.dylib libGLESv1_CM.dylib; do
    src="$ASSET_DIR/$dylib"
    if [[ -f "$src" ]]; then
        cp "$src" "$BUNDLE_OUT/Contents/MacOS/$dylib"
    fi
done

# ── 4. Code-sign (optional — requires CODESIGN_IDENTITY env var) ──────────────
if [[ -n "${CODESIGN_IDENTITY:-}" ]]; then
    echo "==> Code-signing with identity: $CODESIGN_IDENTITY"
    codesign --deep --force --options runtime \
        --sign "$CODESIGN_IDENTITY" \
        "$BUNDLE_OUT"
else
    echo "==> Skipping code-sign (set CODESIGN_IDENTITY to enable)"
    # Ad-hoc sign so macOS Gatekeeper doesn't block local runs entirely
    codesign --deep --force --sign - "$BUNDLE_OUT" 2>/dev/null || true
fi

# ── 5. Create .dmg ────────────────────────────────────────────────────────────
echo "==> Creating $DMG_NAME"
mkdir -p "$DIST_DIR"
STAGING="$REPO_ROOT/target/dmg-staging"
rm -rf "$STAGING"
mkdir -p "$STAGING"
cp -R "$BUNDLE_OUT" "$STAGING/$BUNDLE_NAME"
ln -sf /Applications "$STAGING/Applications"

hdiutil create \
    -volname "Chatminal $VERSION" \
    -srcfolder "$STAGING" \
    -ov \
    -format UDZO \
    "$DIST_DIR/$DMG_NAME"

rm -rf "$STAGING"
echo "==> Done: $DIST_DIR/$DMG_NAME"

# ── 6. Create installer tarball ──────────────────────────────────────────────
echo "==> Creating $ARCHIVE_NAME"
tar -czf "$DIST_DIR/$ARCHIVE_NAME" \
    -C "$REPO_ROOT/target" \
    "$BUNDLE_NAME"

echo "==> Done: $DIST_DIR/$ARCHIVE_NAME"
