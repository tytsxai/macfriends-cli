#!/usr/bin/env bash
set -euo pipefail
umask 077

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
APP_SUPPORT="$HOME/Library/Application Support/MacFriends"
BUNDLE_DIR="$APP_SUPPORT/bundle"
BUNDLE_BIN_DIR="$BUNDLE_DIR/bin"

require_file() {
  local path="$1"
  if [ ! -e "$path" ]; then
    echo "Missing required install asset: $path" >&2
    exit 1
  fi
}

backup_path() {
  local src="$1"
  local backup="$2"
  if [ ! -e "$src" ]; then
    return 0
  fi
  rm -rf "$backup"
  /usr/bin/ditto "$src" "$backup"
}

require_file "$ROOT_DIR/target/release/macfriends"
require_file "$ROOT_DIR/native/agent/build/libmacfriends_agent.dylib"
require_file "$ROOT_DIR/native/agent/build/macfriends-host"
require_file "$ROOT_DIR/fixtures/adapter.wechat-macos-arm64.json"

install -d -m 700 "$BIN_DIR" "$APP_SUPPORT" "$BUNDLE_DIR" "$BUNDLE_BIN_DIR"

backup_path "$BIN_DIR/macfriends" "$BIN_DIR/macfriends.previous"
backup_path "$BUNDLE_DIR" "$APP_SUPPORT/bundle.previous"

TMP_BIN="$(mktemp "$BIN_DIR/.macfriends.install.XXXXXX")"
TMP_BUNDLE="$(mktemp -d "$APP_SUPPORT/.bundle.install.XXXXXX")"
trap 'rm -f "$TMP_BIN"; rm -rf "$TMP_BUNDLE"' EXIT

install -m 755 "$ROOT_DIR/target/release/macfriends" "$TMP_BIN"
install -d -m 700 "$TMP_BUNDLE/bin"
install -m 755 "$ROOT_DIR/native/agent/build/libmacfriends_agent.dylib" "$TMP_BUNDLE/bin/libmacfriends_agent.dylib"
install -m 755 "$ROOT_DIR/native/agent/build/macfriends-host" "$TMP_BUNDLE/bin/macfriends-host"
install -m 644 "$ROOT_DIR/fixtures/adapter.wechat-macos-arm64.json" "$TMP_BUNDLE/adapter.wechat-macos-arm64.json"

mv "$TMP_BIN" "$BIN_DIR/macfriends"
rm -rf "$BUNDLE_DIR"
mv "$TMP_BUNDLE" "$BUNDLE_DIR"
trap - EXIT

echo "Installed macfriends to: $BIN_DIR/macfriends"
echo "Installed bundled assets to: $BUNDLE_DIR"
echo "Previous install backup: $BIN_DIR/macfriends.previous and $APP_SUPPORT/bundle.previous"
echo "Make sure $BIN_DIR is in your PATH."
