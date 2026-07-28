#!/usr/bin/env bash
# Backup MacFriends local data for restore/rollback.
# Does not stop a running managed WeChat; prefer `macfriends detach` first for a cold snapshot.
set -euo pipefail
umask 077

APP_SUPPORT="${MACFRIENDS_ROOT:-$HOME/Library/Application Support/MacFriends}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DEST_ROOT="${1:-$APP_SUPPORT/backups}"
DEST="$DEST_ROOT/macfriends-backup-$STAMP"

if [ ! -d "$APP_SUPPORT" ]; then
  echo "MacFriends data directory not found: $APP_SUPPORT" >&2
  exit 1
fi

mkdir -p "$DEST"

copy_if_exists() {
  local src="$1"
  local name="$2"
  if [ -e "$src" ]; then
    /usr/bin/ditto "$src" "$DEST/$name"
    echo "backed up: $name"
  else
    echo "skip missing: $name"
  fi
}

copy_if_exists "$APP_SUPPORT/runtime" "runtime"
copy_if_exists "$APP_SUPPORT/results" "results"
copy_if_exists "$APP_SUPPORT/bundle" "bundle"
copy_if_exists "$APP_SUPPORT/logs" "logs"

# Socket is ephemeral; keep agent log if it lives under /tmp.
IPC_DIR="/tmp/macfriends-${USER:-user}"
if [ -f "$IPC_DIR/agent.log" ]; then
  mkdir -p "$DEST/ipc"
  /usr/bin/ditto "$IPC_DIR/agent.log" "$DEST/ipc/agent.log"
  echo "backed up: ipc/agent.log"
fi

cat > "$DEST/MANIFEST.txt" <<EOF
created_at_utc=$STAMP
source=$APP_SUPPORT
hostname=$(hostname 2>/dev/null || true)
user=${USER:-unknown}
note=Restore by copying runtime/results/bundle/logs back under Application Support/MacFriends, then run: macfriends status --json && macfriends doctor --json
EOF

echo "Backup complete: $DEST"
echo "Restore guidance is in $DEST/MANIFEST.txt"
