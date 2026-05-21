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
  if [ -n "${WEB_PID:-}" ]; then
    kill "$WEB_PID" >/dev/null 2>&1 || true
    wait "$WEB_PID" >/dev/null 2>&1 || true
  fi
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

run_cli status status
assert_json "$WORK_DIR/status.json" 'data["lifecycle"] == "running_blocked"'
assert_json "$WORK_DIR/status.json" 'data["lifecycle_label"] == "已启动但未满足生产条件"'
assert_json "$WORK_DIR/status.json" 'data["supported_wechat_version"] == "4.1.8"'
assert_json "$WORK_DIR/status.json" '"compatibility_warnings" in data'
assert_json "$WORK_DIR/status.json" 'data["fixture_enabled"] is True'
assert_json "$WORK_DIR/status.json" 'len(data["next_actions"]) >= 1'
run_cli status_zh 状态
assert_json "$WORK_DIR/status_zh.json" 'data["fixture_enabled"] is True'

"$MACFRIENDS_BIN" serve --addr 127.0.0.1:0 > "$WORK_DIR/web.log" 2>&1 &
WEB_PID=$!
for _ in $(seq 1 40); do
  WEB_URL="$(sed -n 's/^MacFriends 本地控制台: //p' "$WORK_DIR/web.log" | head -1)"
  if [ -n "$WEB_URL" ]; then
    break
  fi
  sleep 0.25
done
if [ -z "${WEB_URL:-}" ]; then
  echo "Web console did not start" >&2
  cat "$WORK_DIR/web.log" >&2 || true
  exit 1
fi
/usr/bin/curl -fsS "$WEB_URL/api/health" > "$WORK_DIR/web-health.json"
assert_json "$WORK_DIR/web-health.json" 'data["ok"] is True'
/usr/bin/curl -fsS "$WEB_URL/" > "$WORK_DIR/web-index.html"
if ! /usr/bin/grep -q "MacFriends 本地控制台" "$WORK_DIR/web-index.html"; then
  echo "Web index is not Chinese-first" >&2
  exit 1
fi
/usr/bin/curl -fsS -X OPTIONS "$WEB_URL/api/status" > "$WORK_DIR/web-options.txt"
/usr/bin/curl -fsS "$WEB_URL/api/status" > "$WORK_DIR/web-status.json"
assert_json "$WORK_DIR/web-status.json" 'data["ok"] is True'
assert_json "$WORK_DIR/web-status.json" 'data["data"]["fixture_enabled"] is True'
assert_json "$WORK_DIR/web-status.json" 'data["data"]["lifecycle_label"] == "已启动但未满足生产条件"'
/usr/bin/curl -fsS "$WEB_URL/api/compatibility" > "$WORK_DIR/web-compatibility.json"
assert_json "$WORK_DIR/web-compatibility.json" 'data["ok"] is True'
assert_json "$WORK_DIR/web-compatibility.json" 'data["data"]["supported_wechat_version"] == "4.1.8"'
/usr/bin/curl -fsS "$WEB_URL/api/doctor" > "$WORK_DIR/web-doctor.json"
assert_json "$WORK_DIR/web-doctor.json" 'data["ok"] is True'
/usr/bin/curl -fsS "$WEB_URL/api/attach" > "$WORK_DIR/web-attach.json"
assert_json "$WORK_DIR/web-attach.json" 'data["ok"] is True'
/usr/bin/curl -fsS "$WEB_URL/api/profile" > "$WORK_DIR/web-profile.json"
assert_json "$WORK_DIR/web-profile.json" 'data["ok"] is True'
/usr/bin/curl -fsS "$WEB_URL/api/contacts" > "$WORK_DIR/web-contacts.json"
assert_json "$WORK_DIR/web-contacts.json" 'data["ok"] is True'
/usr/bin/curl -fsS "$WEB_URL/api/logs?kind=cli&lines=20" > "$WORK_DIR/web-cli-log.json"
assert_json "$WORK_DIR/web-cli-log.json" 'data["ok"] is True'
/usr/bin/curl -fsS "$WEB_URL/api/logs?kind=agent&lines=20" > "$WORK_DIR/web-agent-log.json"
assert_json "$WORK_DIR/web-agent-log.json" 'data["ok"] is True'
/usr/bin/curl -fsS -X POST -H 'content-type: application/json' -d '{"all":true}' "$WEB_URL/api/scan" > "$WORK_DIR/web-scan.json"
assert_json "$WORK_DIR/web-scan.json" 'data["ok"] is True'
assert_json "$WORK_DIR/web-scan.json" 'data["data"]["mode"] == "fixture"'
WEB_EXPORT_STATUS="$(/usr/bin/curl -sS -o "$WORK_DIR/web-export.json" -w '%{http_code}' -X POST -H 'content-type: application/json' -d '{"format":"csv"}' "$WEB_URL/api/export")"
if [ "$WEB_EXPORT_STATUS" != "409" ]; then
  echo "Fixture web export returned unexpected HTTP $WEB_EXPORT_STATUS" >&2
  cat "$WORK_DIR/web-export.json" >&2
  exit 1
fi
assert_json "$WORK_DIR/web-export.json" 'data["ok"] is False'
WEB_PREPARE_STATUS="$(/usr/bin/curl -sS -o "$WORK_DIR/web-prepare.json" -w '%{http_code}' -X POST -H 'content-type: application/json' -d '{"source_app":"/definitely/missing/WeChat.app"}' "$WEB_URL/api/prepare")"
if [ "$WEB_PREPARE_STATUS" != "409" ]; then
  echo "Web prepare with missing source returned unexpected HTTP $WEB_PREPARE_STATUS" >&2
  cat "$WORK_DIR/web-prepare.json" >&2
  exit 1
fi
assert_json "$WORK_DIR/web-prepare.json" 'data["ok"] is False'

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
