#!/bin/sh
set -eu

# Download agent_hooks binaries from GitHub releases
# Version is fetched from the latest release (source of truth: Cargo.toml)

REPO="waki285/dotfiles-tools"
HOOKS_DIR="$HOME/.claude/hooks"
OPENCODE_PLUGIN_DIR="$HOME/.config/opencode/plugin"
BINARY_NAME="agent_hooks_claude"
VERSION_FILE="$HOOKS_DIR/.agent_hooks_version"

# Get latest version from GitHub API
get_latest_version() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "https://api.github.com/repos/${REPO}/releases" | \
      grep -o '"tag_name": *"agent_hooks-v[^"]*"' | \
      head -1 | \
      sed 's/.*"agent_hooks-\(v[^"]*\)".*/\1/'
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "https://api.github.com/repos/${REPO}/releases" | \
      grep -o '"tag_name": *"agent_hooks-v[^"]*"' | \
      head -1 | \
      sed 's/.*"agent_hooks-\(v[^"]*\)".*/\1/'
  else
    echo "Error: Neither curl nor wget is available" >&2
    exit 1
  fi
}

VERSION="$(get_latest_version)"

if [ -z "$VERSION" ]; then
  echo "Error: Could not determine latest version" >&2
  exit 1
fi

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      x86_64)
        CLAUDE_ASSET="agent_hooks_claude-macos-x86_64"
        OPENCODE_ASSET="agent_hooks_opencode-macos-x86_64.node"
        ;;
      arm64)
        CLAUDE_ASSET="agent_hooks_claude-macos-arm64"
        OPENCODE_ASSET="agent_hooks_opencode-macos-arm64.node"
        ;;
      *)
        echo "Unsupported macOS architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64)
        CLAUDE_ASSET="agent_hooks_claude-linux-x86_64"
        OPENCODE_ASSET="agent_hooks_opencode-linux-x86_64.node"
        ;;
      aarch64|arm64)
        CLAUDE_ASSET="agent_hooks_claude-linux-arm64"
        OPENCODE_ASSET="agent_hooks_opencode-linux-arm64.node"
        ;;
      *)
        echo "Unsupported Linux architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

CLAUDE_DOWNLOAD_URL="https://github.com/${REPO}/releases/download/agent_hooks-${VERSION}/${CLAUDE_ASSET}"
OPENCODE_DOWNLOAD_URL="https://github.com/${REPO}/releases/download/agent_hooks-${VERSION}/${OPENCODE_ASSET}"
CLAUDE_TARGET_PATH="$HOOKS_DIR/$BINARY_NAME"
OPENCODE_TARGET_PATH="$OPENCODE_PLUGIN_DIR/agent_hooks.node"

# Check if already installed with correct version
if [ -f "$CLAUDE_TARGET_PATH" ] && [ -f "$VERSION_FILE" ]; then
  INSTALLED_VERSION="$(cat "$VERSION_FILE")"
  if [ "$INSTALLED_VERSION" = "$VERSION" ]; then
    echo "agent_hooks $VERSION is already installed, skipping download"
    exit 0
  fi
fi

# Create directories if they don't exist
mkdir -p "$HOOKS_DIR"
mkdir -p "$OPENCODE_PLUGIN_DIR"

# Download the Claude CLI binary
echo "Downloading $CLAUDE_ASSET $VERSION from $CLAUDE_DOWNLOAD_URL..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "$CLAUDE_TARGET_PATH" "$CLAUDE_DOWNLOAD_URL"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$CLAUDE_TARGET_PATH" "$CLAUDE_DOWNLOAD_URL"
fi

# Make it executable
chmod +x "$CLAUDE_TARGET_PATH"

echo "Successfully installed $BINARY_NAME $VERSION to $CLAUDE_TARGET_PATH"

# Download the OpenCode NAPI binary
echo "Downloading $OPENCODE_ASSET $VERSION from $OPENCODE_DOWNLOAD_URL..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "$OPENCODE_TARGET_PATH" "$OPENCODE_DOWNLOAD_URL"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$OPENCODE_TARGET_PATH" "$OPENCODE_DOWNLOAD_URL"
fi

echo "Successfully installed agent_hooks.node $VERSION to $OPENCODE_TARGET_PATH"

# Save version file
printf '%s\n' "$VERSION" > "$VERSION_FILE"

echo "agent_hooks $VERSION installation complete"
