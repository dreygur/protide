#!/usr/bin/env bash
set -euo pipefail

REPO="${PROTIDE_REPO:-dreygur/protide}"
VERSION="${PROTIDE_VERSION:-latest}"
INSTALL_DIR="${PROTIDE_INSTALL_DIR:-/usr/local/bin}"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()  { printf "${GREEN}%s${NC}\n" "$*"; }
warn()  { printf "${YELLOW}%s${NC}\n" "$*"; }
error() { printf "${RED}%s${NC}\n" "$*" >&2; }

ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')

case "$ARCH" in
  x86_64)  ARCH="x86_64" ;;
  aarch64|arm64) ARCH="aarch64" ;;
  *) error "Unsupported architecture: $ARCH"; exit 1 ;;
esac

if [ "$OS" != "linux" ]; then
  error "Unsupported OS: $OS (only Linux is supported for now)"
  exit 1
fi

if [ "$VERSION" = "latest" ]; then
  info "Fetching latest release info..."
  API_URL="https://api.github.com/repos/$REPO/releases/latest"
  VERSION=$(curl -sL "$API_URL" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": "\(.*\)",/\1/')
  if [ -z "$VERSION" ]; then
    error "Failed to determine latest version from $API_URL"
    exit 1
  fi
  info "Latest version: $VERSION"
fi

ARCHIVE="protide-${VERSION}-${ARCH}-linux"
ARCHIVE_URL="https://github.com/$REPO/releases/download/$VERSION/${ARCHIVE}.tar.gz"

TMPDIR=$(mktemp -d)
cleanup() { rm -rf "$TMPDIR"; }
trap cleanup EXIT

info "Downloading $ARCHIVE_URL ..."
curl -sL "$ARCHIVE_URL" | tar xz -C "$TMPDIR"

BINARY="$TMPDIR/$ARCHIVE/protide"
if [ ! -f "$BINARY" ]; then
  error "Binary not found in archive (expected $ARCHIVE/protide)"
  exit 1
fi

if [ "$INSTALL_DIR" = "/usr/local/bin" ] && [ ! -w "$INSTALL_DIR" ]; then
  warn "No write permission to $INSTALL_DIR — using sudo"
  chmod +x "$BINARY"
  sudo mv "$BINARY" "$INSTALL_DIR/protide"
  info "Installed protide to $INSTALL_DIR/protide"
else
  mkdir -p "$INSTALL_DIR"
  chmod +x "$BINARY"
  mv "$BINARY" "$INSTALL_DIR/protide"
  info "Installed protide to $INSTALL_DIR/protide"
fi

ASSETS_DIR="$TMPDIR/$ARCHIVE/assets"
if [ -d "$ASSETS_DIR" ]; then
  ICON_DIR="${HOME}/.local/share/icons/hicolor/256x256/apps"
  DESKTOP_DIR="${HOME}/.local/share/applications"
  mkdir -p "$ICON_DIR" "$DESKTOP_DIR"

  if [ -f "$ASSETS_DIR/protide-logo.png" ]; then
    cp "$ASSETS_DIR/protide-logo.png" "$ICON_DIR/protide.png"
    info "Installed icon → $ICON_DIR/protide.png"
  fi

  if [ -f "$TMPDIR/$ARCHIVE/protide.desktop" ]; then
    BIN_PATH="$INSTALL_DIR/protide"
    sed "s|^Exec=.*|Exec=$BIN_PATH|" "$TMPDIR/$ARCHIVE/protide.desktop" \
      > "$DESKTOP_DIR/protide.desktop"
    info "Installed .desktop → $DESKTOP_DIR/protide.desktop"
  fi

  update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
  gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
fi

if ! command -v protide &>/dev/null; then
  warn "Make sure $INSTALL_DIR is in your PATH:"
  echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi

info "Protide $VERSION installed successfully!"
