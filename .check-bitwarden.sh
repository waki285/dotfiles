#!/bin/sh
set -eu

# Check if bw command exists
if ! command -v bw >/dev/null 2>&1; then
  echo "Error: Bitwarden CLI (bw) is not installed." >&2
  echo "Install it with: brew install bitwarden-cli" >&2
  exit 1
fi

# Check if logged in and unlocked
status="$(bw status 2>/dev/null | jq -r '.status' 2>/dev/null || echo "unknown")"

case "$status" in
  unlocked)
    exit 0
    ;;
  locked)
    echo "Error: Bitwarden is locked. Run: bw unlock" >&2
    exit 1
    ;;
  unauthenticated)
    echo "Error: Bitwarden is not logged in. Run: bw login" >&2
    exit 1
    ;;
  *)
    echo "Error: Could not determine Bitwarden status." >&2
    exit 1
    ;;
esac
