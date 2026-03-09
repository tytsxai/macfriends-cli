#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
APP_SUPPORT="$HOME/Library/Application Support/MacFriends"
BUNDLE_DIR="$APP_SUPPORT/bundle"
BUNDLE_BIN_DIR="$BUNDLE_DIR/bin"

mkdir -p "$BIN_DIR" "$BUNDLE_BIN_DIR"

backup_path() {
  local src="$1"
  local backup="$2"
  if [ ! -e "$src" ]; then
    return 0
  fi
  rm -rf "$backup"
  cp -R "$src" "$backup"
}

backup_path "$BIN_DIR/macfriends" "$BIN_DIR/macfriends.previous"
backup_path "$BUNDLE_DIR" "$APP_SUPPORT/bundle.previous"

install -m 755 "$ROOT_DIR/target/release/macfriends" "$BIN_DIR/macfriends"
install -m 755 "$ROOT_DIR/native/agent/build/libmacfriends_agent.dylib" "$BUNDLE_BIN_DIR/libmacfriends_agent.dylib"
install -m 755 "$ROOT_DIR/native/agent/build/macfriends-host" "$BUNDLE_BIN_DIR/macfriends-host"
install -m 644 "$ROOT_DIR/fixtures/adapter.wechat-macos-arm64.json" "$BUNDLE_DIR/adapter.wechat-macos-arm64.json"

echo "Installed macfriends to: $BIN_DIR/macfriends"
echo "Installed bundled assets to: $BUNDLE_DIR"
echo "Previous install backup: $BIN_DIR/macfriends.previous and $APP_SUPPORT/bundle.previous"
echo "Make sure $BIN_DIR is in your PATH."
