#!/bin/sh
set -eu

FILE="$HOME/.claude.json"

# context7 settings
CONTEXT7_ITEM_NAME="context7-api-key"
CONTEXT7_KEY_NAME="CONTEXT7_API_KEY"
CONTEXT7_URL="https://mcp.context7.com/mcp"

# searxng settings
SEARXNG_URL="http://127.0.0.1:8080"

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is not installed. Please install jq first." >&2
  exit 1
fi

CONTEXT7_API_KEY="$(bw get password "$CONTEXT7_ITEM_NAME")"

if [ ! -f "$FILE" ]; then
  printf '%s\n' '{}' > "$FILE"
fi

if ! jq -e . "$FILE" >/dev/null 2>&1; then
  echo "Error: $FILE is not valid JSON. Fix it first." >&2
  exit 1
fi

tmp="$(mktemp)"
jq \
  --arg context7_api_key "$CONTEXT7_API_KEY" \
  --arg context7_url "$CONTEXT7_URL" \
  --arg context7_key_name "$CONTEXT7_KEY_NAME" \
  --arg searxng_url "$SEARXNG_URL" \
  '
  .hasCompletedOnboarding = true
  | .mcpServers = (.mcpServers // {})
  | .mcpServers.context7 = {
      type: "http",
      url: $context7_url,
      headers: { ($context7_key_name): $context7_api_key }
    }
  | .mcpServers.searxng = {
      type: "stdio",
      command: "npx",
      args: ["-y", "mcp-searxng"],
      env: { "SEARXNG_URL": $searxng_url }
    }
  ' \
  "$FILE" > "$tmp"

mv "$tmp" "$FILE"
chmod 600 "$FILE"
