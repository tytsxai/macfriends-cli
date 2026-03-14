#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MACFRIENDS_BIN="${MACFRIENDS_BIN:-$ROOT_DIR/target/debug/macfriends}"
AGENT_DYLIB="$ROOT_DIR/native/agent/build/libmacfriends_agent.dylib"
AGENT_HOST="$ROOT_DIR/native/agent/build/macfriends-host"
ADAPTER_PATH="$ROOT_DIR/fixtures/adapter.wechat-macos-arm64.json"
WORK_DIR="/tmp/macfriends-smoke-$$"
HOME_DIR="$WORK_DIR/home"
APP_SUPPORT="$HOME_DIR/Library/Application Support/MacFriends"
SOCKET_PATH="$APP_SUPPORT/runtime/agent.sock"
AGENT_LOG="$APP_SUPPORT/logs/agent.log"

require_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "Missing required file: $path" >&2
    exit 1
  fi
}

assert_json() {
  local path="$1"
  local expression="$2"
  /usr/bin/python3 - "$path" "$expression" <<'PY'
import json
import sys

path = sys.argv[1]
expression = sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

context = {"data": data, "len": len, "any": any, "all": all}
if not eval(expression, {"__builtins__": {}}, context):
    raise SystemExit(
        f"Assertion failed: {expression}\nFile: {path}\nPayload:\n"
        + json.dumps(data, ensure_ascii=False, indent=2)
    )
PY
}

run_cli() {
  local name="$1"
  shift
  "$MACFRIENDS_BIN" --json "$@" > "$WORK_DIR/$name.json"
}

cleanup() {
  if [ -n "${HOST_PID:-}" ]; then
    kill "$HOST_PID" >/dev/null 2>&1 || true
    wait "$HOST_PID" >/dev/null 2>&1 || true
  fi
  rm -rf "$WORK_DIR"
}

require_file "$MACFRIENDS_BIN"
require_file "$AGENT_DYLIB"
require_file "$AGENT_HOST"
require_file "$ADAPTER_PATH"

rm -rf "$WORK_DIR"
mkdir -p "$APP_SUPPORT/runtime"
export HOME="$HOME_DIR"
export MACFRIENDS_AGENT_SOCKET="$SOCKET_PATH"
export MACFRIENDS_LOG_FILE="$AGENT_LOG"

trap cleanup EXIT

MACFRIENDS_ADAPTER_PATH="$ADAPTER_PATH" \
MACFRIENDS_ENABLE_FIXTURE=1 \
DYLD_INSERT_LIBRARIES="$AGENT_DYLIB" \
"$AGENT_HOST" >/dev/null 2>&1 &
HOST_PID=$!

for _ in $(seq 1 40); do
  if [ -S "$SOCKET_PATH" ]; then
    break
  fi
  sleep 0.25
done

if [ ! -S "$SOCKET_PATH" ]; then
  echo "Fixture agent socket did not become ready: $SOCKET_PATH" >&2
  if [ -f "$AGENT_LOG" ]; then
    cat "$AGENT_LOG" >&2
  fi
  exit 1
fi

run_cli doctor doctor
assert_json "$WORK_DIR/doctor.json" 'data["fixture_enabled"] is True'
assert_json "$WORK_DIR/doctor.json" 'data["runtime_ready"] is False'
assert_json "$WORK_DIR/doctor.json" 'len(data["release_blockers"]) >= 1'

run_cli attach attach
assert_json "$WORK_DIR/attach.json" 'data["fixture_enabled"] is True'
assert_json "$WORK_DIR/attach.json" 'data["primitive_resolution"]["profile"] == "fixture"'

run_cli profile profile
assert_json "$WORK_DIR/profile.json" 'data["wxid"] == "wxid_mock_macfriends"'

run_cli contacts contacts
assert_json "$WORK_DIR/contacts.json" 'len(data) >= 2'

run_cli scan scan --all
assert_json "$WORK_DIR/scan.json" 'data["mode"] == "fixture"'
assert_json "$WORK_DIR/scan.json" 'len(data["records"]) >= 2'

if "$MACFRIENDS_BIN" --json export --format json > "$WORK_DIR/export.json"; then
  echo "Fixture export unexpectedly succeeded" >&2
  cat "$WORK_DIR/export.json" >&2
  exit 1
fi
assert_json "$WORK_DIR/export.json" 'data["error_code"] == "production_scan_missing"'

run_cli detach detach
assert_json "$WORK_DIR/detach.json" 'data["message"] == "agent 已停止"'

for _ in $(seq 1 20); do
  if [ ! -S "$SOCKET_PATH" ]; then
    break
  fi
  sleep 0.25
done

if [ -S "$SOCKET_PATH" ]; then
  echo "Fixture socket still exists after detach" >&2
  exit 1
fi

echo "Fixture smoke passed"
