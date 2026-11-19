#!/usr/bin/env bash
# Renders Casks/protide.rb for the release .dmg that CI published for the
# workspace version. Run it after the release workflow finishes, then commit
# the cask so `brew upgrade` picks the release up.
#
#   packaging/macos/cask.sh   -> Casks/protide.rb
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR/../.."
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
URL="https://github.com/dreygur/protide/releases/download/v$VERSION/protide-v$VERSION-macos-arm64.dmg"

SUM="$(curl -fsSL "$URL" | shasum -a 256 | awk '{print $1}')"
mkdir -p "$ROOT/Casks"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@SHA256@/$SUM/g" "$SCRIPT_DIR/protide.rb.in" >"$ROOT/Casks/protide.rb"
echo "Casks/protide.rb: $VERSION (sha256 $SUM)"
