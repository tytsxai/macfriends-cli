#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/crates/cli/Cargo.toml" | head -1)"
PACKAGE_NAME="macfriends-${VERSION}-macos-arm64"
DIST_DIR="$ROOT_DIR/dist/$PACKAGE_NAME"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/bin" "$DIST_DIR/bundle/bin" "$DIST_DIR/docs"

cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"
make -C "$ROOT_DIR/native/agent" artifacts

cp "$ROOT_DIR/target/release/macfriends" "$DIST_DIR/bin/"
cp "$ROOT_DIR/native/agent/build/libmacfriends_agent.dylib" "$DIST_DIR/bundle/bin/"
cp "$ROOT_DIR/native/agent/build/macfriends-host" "$DIST_DIR/bundle/bin/"
cp "$ROOT_DIR/fixtures/adapter.wechat-macos-arm64.json" "$DIST_DIR/bundle/"
cp "$ROOT_DIR/README.md" "$DIST_DIR/"
cp "$ROOT_DIR/README.en.md" "$DIST_DIR/"
cp "$ROOT_DIR/LICENSE" "$DIST_DIR/"
cp "$ROOT_DIR/CHANGELOG.md" "$DIST_DIR/"
cp "$ROOT_DIR/scripts/install.sh" "$DIST_DIR/"
cp -R "$ROOT_DIR/docs/." "$DIST_DIR/docs/"

(
  cd "$ROOT_DIR/dist"
  tar -czf "$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"
)

echo "Packaged: $ROOT_DIR/dist/$PACKAGE_NAME.tar.gz"
