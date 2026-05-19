#!/usr/bin/env bash
# Install Protide's .desktop entry and icon for the current user's taskbar/dock.
# Usage: ./install-desktop.sh [path-to-protide-binary]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# tar.gz layout: binary is a sibling of this script; source-tree layout: target/release/protide
BINARY="${1:-$(which protide 2>/dev/null || ls "$SCRIPT_DIR/protide" "$SCRIPT_DIR/../target/release/protide" 2>/dev/null | head -1)}"

ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
DESKTOP_DIR="$HOME/.local/share/applications"

mkdir -p "$ICON_DIR" "$DESKTOP_DIR"

# tar.gz layout: assets/ is a sibling of this script; source-tree layout: crates/protide/assets/
ICON_SRC="$SCRIPT_DIR/assets/protide-logo.png"
[[ -f "$ICON_SRC" ]] || ICON_SRC="$SCRIPT_DIR/../crates/protide/assets/protide-logo.png"
if [[ -f "$ICON_SRC" ]]; then
    cp "$ICON_SRC" "$ICON_DIR/protide.png"
    echo "Installed icon → $ICON_DIR/protide.png"
else
    echo "Warning: icon not found at $ICON_SRC — skipping icon install"
fi

# Install .desktop file with the resolved binary path
RESOLVED_BINARY="$(realpath "$BINARY" 2>/dev/null || echo "$BINARY")"
sed "s|^Exec=.*|Exec=$RESOLVED_BINARY|" "$SCRIPT_DIR/protide.desktop" \
    > "$DESKTOP_DIR/protide.desktop"
echo "Installed .desktop → $DESKTOP_DIR/protide.desktop"

# Refresh caches
update-desktop-database "$DESKTOP_DIR" 2>/dev/null && echo "Desktop database refreshed" || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null && echo "Icon cache refreshed" || true

echo ""
echo "Done. Protide should now appear in your application launcher."
echo "If the icon doesn't appear immediately, try logging out and back in."
