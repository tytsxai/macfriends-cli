#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ADAPTER_PATH="${MACFRIENDS_ADAPTER_PATH:-$ROOT_DIR/fixtures/adapter.wechat-macos-arm64.json}"
BETA_OPT_IN="${MACFRIENDS_ALLOW_BETA_RELEASE:-0}"

if [ ! -f "$ADAPTER_PATH" ]; then
  echo "Release guard failed: missing adapter manifest: $ADAPTER_PATH" >&2
  exit 1
fi

json_value() {
  local key="$1"
  /usr/bin/plutil -extract "$key" raw "$ADAPTER_PATH" 2>/dev/null || true
}

release_channel="$(json_value "release_channel")"
profile_resolution="$(json_value "primitive_resolution.profile")"
contacts_resolution="$(json_value "primitive_resolution.contacts")"
scan_resolution="$(json_value "primitive_resolution.scan")"

release_channel="${release_channel:-unknown}"
profile_resolution="${profile_resolution:-unknown}"
contacts_resolution="${contacts_resolution:-unknown}"
scan_resolution="${scan_resolution:-unknown}"

unresolved=()
for item in \
  "profile=$profile_resolution" \
  "contacts=$contacts_resolution" \
  "scan=$scan_resolution"; do
  state="${item#*=}"
  if [ "$state" != "resolved" ]; then
    unresolved+=("$item")
  fi
done

if [ "${#unresolved[@]}" -eq 0 ] && [ "$release_channel" = "production" ]; then
  echo "Release guard passed: adapter primitives are resolved for production."
  exit 0
fi

if [ "$BETA_OPT_IN" = "1" ]; then
  echo "Release guard beta opt-in accepted."
  echo "Adapter release_channel=$release_channel; primitive_resolution=${unresolved[*]:-resolved}."
  echo "Artifact is beta/testable only and must not be described as production-ready."
  exit 0
fi

echo "Release guard blocked packaging." >&2
echo "Adapter release_channel=$release_channel; primitive_resolution profile=$profile_resolution contacts=$contacts_resolution scan=$scan_resolution." >&2
echo "Real primitives are not resolved, so this cannot be presented as a production-ready release." >&2
echo "To intentionally build a beta artifact, rerun with MACFRIENDS_ALLOW_BETA_RELEASE=1." >&2
exit 2
