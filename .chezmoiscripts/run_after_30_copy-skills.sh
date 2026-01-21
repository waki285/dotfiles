#!/bin/sh
set -eu

CHEZMOI_SOURCE_DIR="${CHEZMOI_SOURCE_DIR:-$(chezmoi source-path)}"
SKILLS_SRC="$CHEZMOI_SOURCE_DIR/.chezmoitemplates/skills"

if [ ! -d "$SKILLS_SRC" ]; then
  echo "Skills source directory not found: $SKILLS_SRC" >&2
  exit 1
fi

DESTINATIONS="
$HOME/.claude/skills
$HOME/.codex/skills
$HOME/.config/opencode/skills
"

for dest in $DESTINATIONS; do
  mkdir -p "$dest"
  cp -R "$SKILLS_SRC"/* "$dest"/
done
