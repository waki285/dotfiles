#!/bin/sh
set -eu

FILE="$HOME/.claude.json"
ITEM_NAME="context7-api-key"
KEY_NAME="CONTEXT7_API_KEY"
URL="https://mcp.context7.com/mcp"

API_KEY="$(bw get password "$ITEM_NAME")"

if [ ! -f "$FILE" ]; then
  printf '%s\n' '{}' > "$FILE"
fi

if ! jq -e . "$FILE" >/dev/null 2>&1; then
  echo "Error: $FILE is not valid JSON. Fix it first." >&2
  exit 1
fi

tmp="$(mktemp)"
jq \
  --arg api_key "$API_KEY" \
  --arg url "$URL" \
  --arg key_name "$KEY_NAME" \
  '
  .hasCompletedOnboarding = true
  | .mcpServers = (.mcpServers // {})
  | .mcpServers.context7 = {
      type: "http",
      url: $url,
      headers: { ($key_name): $api_key }
    }
  ' \
  "$FILE" > "$tmp"

mv "$tmp" "$FILE"
chmod 600 "$FILE"
