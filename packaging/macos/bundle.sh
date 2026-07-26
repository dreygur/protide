#!/usr/bin/env bash
# Assemble target/release/protide into a macOS .app bundle. Finder, the Dock and
# the app switcher only read an icon from a bundle (Info.plist CFBundleIconFile
# -> Resources/protide.icns), so a bare binary can never have one.
# Usage: ./bundle.sh [output-app-path]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/../.."
BINARY="$ROOT/target/release/protide"
LOGO="$ROOT/crates/protide/assets/protide-logo.png"
APP="${1:-$ROOT/target/Protide.app}"

[[ -f "$BINARY" ]] || { echo "No release binary at $BINARY - run 'make release' first" >&2; exit 1; }
[[ -f "$LOGO" ]] || { echo "No logo at $LOGO" >&2; exit 1; }

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

# iconutil only accepts this exact set of names/sizes
ICONSET="$(mktemp -d)/protide.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$LOGO" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    sips -z "$((size * 2))" "$((size * 2))" "$LOGO" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/protide.icns"

cp "$BINARY" "$APP/Contents/MacOS/protide"
sed "s|__VERSION__|$VERSION|g" "$SCRIPT_DIR/Info.plist" > "$APP/Contents/Info.plist"

# Ad-hoc signature: unsigned bundles get killed by Gatekeeper on Apple Silicon
codesign --force --sign - "$APP" 2>/dev/null || echo "Warning: codesign failed - bundle is unsigned"

# Finder caches icons per bundle mtime; touching it forces a re-read
touch "$APP"

echo "Built $APP (version $VERSION)"
echo "Run it with: open '$APP'   (drag it to /Applications to keep it in the Dock)"
