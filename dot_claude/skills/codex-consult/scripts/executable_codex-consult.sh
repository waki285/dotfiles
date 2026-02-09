#!/usr/bin/env bash
# Wrapper for codex exec with provider fallback and output saving.
#
# Usage: codex-consult.sh <prompt> [topic-slug]
#
# 1. Try with -c model_provider=voids
# 2. On failure, retry without it
# 3. Save last message to ~/.claude/second-opinions/<timestamp>_<topic>.md

set -euo pipefail

if [ $# -eq 0 ]; then
    echo "Usage: codex-consult.sh <prompt> [topic-slug]" >&2
    exit 1
fi

PROMPT="$1"
TOPIC="${2:-general}"
# Sanitize topic: lowercase, replace non-alphanumeric with hyphens, trim to 30 chars
TOPIC=$(echo "$TOPIC" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g' | cut -c1-30 | sed 's/-*$//')

OUTPUT_DIR="$HOME/.claude/second-opinions"
mkdir -p "$OUTPUT_DIR"

TIMESTAMP=$(date +%Y-%m-%d_%H-%M-%S)
OUTPUT_FILE="$OUTPUT_DIR/${TIMESTAMP}_${TOPIC}.md"

# Try with voids provider first
if codex exec \
    -c model_provider=voids \
    --full-auto \
    --skip-git-repo-check \
    -o "$OUTPUT_FILE" \
    "$PROMPT" 2>/dev/null; then
    echo "$OUTPUT_FILE"
    exit 0
fi

# Fallback: retry without voids provider
echo "Retrying without voids provider..." >&2
if codex exec \
    --full-auto \
    --skip-git-repo-check \
    -o "$OUTPUT_FILE" \
    "$PROMPT"; then
    echo "$OUTPUT_FILE"
    exit 0
fi

echo "Codex consultation failed" >&2
exit 1
