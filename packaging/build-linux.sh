#!/usr/bin/env bash
# Build a distributable Linux tarball.
# Usage: ./packaging/build-linux.sh [--target <triple>]
# Output: target/dist/protide-<version>-<arch>-linux.tar.gz
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# ── options ────────────────────────────────────────────────────────────────
TARGET=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target) TARGET="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

# ── version ────────────────────────────────────────────────────────────────
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

# ── arch ───────────────────────────────────────────────────────────────────
if [[ -n "$TARGET" ]]; then
  case "$TARGET" in
    x86_64*)  ARCH="x86_64" ;;
    aarch64*) ARCH="aarch64" ;;
    *) ARCH="$TARGET" ;;
  esac
else
  ARCH=$(uname -m)
fi

OUTPUT_NAME="protide-${VERSION}-${ARCH}-linux"
echo "Building ${OUTPUT_NAME} ..."

# ── compile ────────────────────────────────────────────────────────────────
CARGO_ARGS=(build -p protide --release)
[[ -n "$TARGET" ]] && CARGO_ARGS+=(--target "$TARGET")
cargo "${CARGO_ARGS[@]}"

BINARY_PATH="target/${TARGET:+${TARGET}/}release/protide"
BINARY_PATH="${BINARY_PATH//\/\//\/}"   # collapse double slashes

# ── stage ──────────────────────────────────────────────────────────────────
STAGING="target/dist/${OUTPUT_NAME}"
rm -rf "$STAGING"
mkdir -p "$STAGING/assets"

cp "$BINARY_PATH"                              "$STAGING/protide"
cp packaging/protide.desktop                   "$STAGING/protide.desktop"
cp crates/protide/assets/protide-logo.png      "$STAGING/assets/protide-logo.png"

chmod +x "$STAGING/protide"

# embed a quick per-user install helper inside the tarball
cat > "$STAGING/install.sh" << 'INNER'
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${1:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"
cp "$SCRIPT_DIR/protide" "$INSTALL_DIR/protide"
echo "Installed protide → $INSTALL_DIR/protide"

ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$ICON_DIR" "$DESKTOP_DIR"
cp "$SCRIPT_DIR/assets/protide-logo.png" "$ICON_DIR/protide.png"
sed "s|^Exec=.*|Exec=$INSTALL_DIR/protide|" "$SCRIPT_DIR/protide.desktop" \
  > "$DESKTOP_DIR/protide.desktop"
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
echo "Desktop entry and icon installed."
INNER
chmod +x "$STAGING/install.sh"

# ── tarball ────────────────────────────────────────────────────────────────
mkdir -p target/dist
(cd target/dist && tar czf "${OUTPUT_NAME}.tar.gz" "${OUTPUT_NAME}")

echo ""
echo "✓  target/dist/${OUTPUT_NAME}.tar.gz"
