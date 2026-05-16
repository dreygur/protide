#!/usr/bin/env bash
# Build a distributable macOS DMG (must run on macOS).
# Usage: ./packaging/build-mac.sh [--target <triple>] [--sign <identity>]
#   --target   e.g. aarch64-apple-darwin or x86_64-apple-darwin
#   --sign     codesign identity (default: ad-hoc "-")
# Output: target/dist/protide-<version>-<arch>-mac.dmg
set -euo pipefail

if [[ "$(uname)" != "Darwin" ]]; then
  echo "Error: this script must run on macOS." >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# ── options ────────────────────────────────────────────────────────────────
TARGET=""
SIGN_IDENTITY="-"   # ad-hoc; pass your Developer ID to notarize
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target) TARGET="$2"; shift 2 ;;
    --sign)   SIGN_IDENTITY="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# ── version / arch ─────────────────────────────────────────────────────────
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

if [[ -n "$TARGET" ]]; then
  case "$TARGET" in
    x86_64*)  ARCH="x86_64" ;;
    aarch64*) ARCH="arm64" ;;
    *) ARCH="$TARGET" ;;
  esac
else
  ARCH=$(uname -m)   # arm64 on Apple Silicon, x86_64 on Intel
fi

OUTPUT_NAME="protide-${VERSION}-${ARCH}-mac"
echo "Building ${OUTPUT_NAME} ..."

# ── compile ────────────────────────────────────────────────────────────────
CARGO_ARGS=(build -p protide --release)
[[ -n "$TARGET" ]] && CARGO_ARGS+=(--target "$TARGET")
cargo "${CARGO_ARGS[@]}"

BINARY_PATH="target/${TARGET:+${TARGET}/}release/protide"
BINARY_PATH="${BINARY_PATH//\/\//\/}"

# ── .app bundle ────────────────────────────────────────────────────────────
DIST="target/dist"
APP="$DIST/Protide.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BINARY_PATH" "$APP/Contents/MacOS/protide"
chmod +x "$APP/Contents/MacOS/protide"

# Convert PNG → ICNS (sips + iconutil are macOS built-ins)
ICON_PNG="crates/protide/assets/protide-logo.png"
ICONSET="$DIST/protide.iconset"
rm -rf "$ICONSET"; mkdir -p "$ICONSET"
for SIZE in 16 32 64 128 256 512; do
  sips -z $SIZE $SIZE "$ICON_PNG" --out "$ICONSET/icon_${SIZE}x${SIZE}.png"     >/dev/null
  DOUBLE=$((SIZE * 2))
  [[ $DOUBLE -le 1024 ]] && \
    sips -z $DOUBLE $DOUBLE "$ICON_PNG" --out "$ICONSET/icon_${SIZE}x${SIZE}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$ICONSET"

# Info.plist
cat > "$APP/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>              <string>Protide</string>
    <key>CFBundleDisplayName</key>       <string>Protide</string>
    <key>CFBundleIdentifier</key>        <string>com.protide.app</string>
    <key>CFBundleVersion</key>           <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleExecutable</key>        <string>protide</string>
    <key>CFBundleIconFile</key>          <string>AppIcon</string>
    <key>CFBundlePackageType</key>       <string>APPL</string>
    <key>LSApplicationCategoryType</key> <string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key>   <true/>
    <key>LSMinimumSystemVersion</key>    <string>12.0</string>
</dict>
</plist>
PLIST

# ── code sign ──────────────────────────────────────────────────────────────
echo "Code-signing with identity: ${SIGN_IDENTITY}"
codesign --deep --force --sign "$SIGN_IDENTITY" "$APP"

# ── DMG ────────────────────────────────────────────────────────────────────
# Staging dir with the .app + /Applications symlink for drag-to-install UI
DMG_STAGING="$DIST/dmg-staging"
rm -rf "$DMG_STAGING"; mkdir -p "$DMG_STAGING"
cp -R "$APP" "$DMG_STAGING/Protide.app"
ln -s /Applications "$DMG_STAGING/Applications"

DMG_PATH="$DIST/${OUTPUT_NAME}.dmg"
hdiutil create \
  -volname "Protide ${VERSION}" \
  -srcfolder "$DMG_STAGING" \
  -ov -format UDZO \
  "$DMG_PATH"

rm -rf "$DMG_STAGING"

echo ""
echo "✓  $DMG_PATH"
echo ""
echo "Note: ad-hoc signed DMGs will show a Gatekeeper warning on first launch."
echo "Pass --sign 'Developer ID Application: Your Name (TEAMID)' to sign properly."
