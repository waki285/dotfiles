#!/bin/sh
set -eu

# Download agent_hooks and claude_statusline binaries from GitHub releases

REPO="waki285/dotfiles-tools"
HOOKS_DIR="$HOME/.claude/hooks"
OPENCODE_PLUGIN_DIR="$HOME/.config/opencode/plugin"
AGENT_HOOKS_BINARY_NAME="agent_hooks"
STATUSLINE_BINARY_NAME="claude_statusline"
AGENT_HOOKS_VERSION_FILE="$HOOKS_DIR/.agent_hooks_version"
STATUSLINE_VERSION_FILE="$HOOKS_DIR/.claude_statusline_version"

download_file() {
  url="$1"
  target="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$target" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$target" "$url"
  else
    echo "Error: Neither curl nor wget is available" >&2
    exit 1
  fi
}

get_latest_version() {
  prefix="$1"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "https://api.github.com/repos/${REPO}/releases" | \
      grep -o "\"tag_name\": *\"${prefix}-v[^\"]*\"" | \
      head -1 | \
      sed "s/.*\"${prefix}-\\(v[^\"]*\\)\".*/\\1/"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "https://api.github.com/repos/${REPO}/releases" | \
      grep -o "\"tag_name\": *\"${prefix}-v[^\"]*\"" | \
      head -1 | \
      sed "s/.*\"${prefix}-\\(v[^\"]*\\)\".*/\\1/"
  else
    echo "Error: Neither curl nor wget is available" >&2
    exit 1
  fi
}

AGENT_HOOKS_VERSION="$(get_latest_version "agent_hooks")"
STATUSLINE_VERSION="$(get_latest_version "claude_statusline")"

if [ -z "$AGENT_HOOKS_VERSION" ]; then
  echo "Error: Could not determine latest agent_hooks version" >&2
  exit 1
fi

if [ -z "$STATUSLINE_VERSION" ]; then
  echo "Error: Could not determine latest claude_statusline version" >&2
  exit 1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      x86_64)
        AGENT_HOOKS_ASSET="agent_hooks-macos-x86_64"
        OPENCODE_ASSET="agent_hooks_opencode-macos-x86_64.node"
        STATUSLINE_ASSET="claude_statusline-macos-x86_64"
        ;;
      arm64)
        AGENT_HOOKS_ASSET="agent_hooks-macos-arm64"
        OPENCODE_ASSET="agent_hooks_opencode-macos-arm64.node"
        STATUSLINE_ASSET="claude_statusline-macos-arm64"
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
        AGENT_HOOKS_ASSET="agent_hooks-linux-x86_64"
        OPENCODE_ASSET="agent_hooks_opencode-linux-x86_64.node"
        STATUSLINE_ASSET="claude_statusline-linux-x86_64"
        ;;
      aarch64|arm64)
        AGENT_HOOKS_ASSET="agent_hooks-linux-arm64"
        OPENCODE_ASSET="agent_hooks_opencode-linux-arm64.node"
        STATUSLINE_ASSET="claude_statusline-linux-arm64"
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

AGENT_HOOKS_DOWNLOAD_URL="https://github.com/${REPO}/releases/download/agent_hooks-${AGENT_HOOKS_VERSION}/${AGENT_HOOKS_ASSET}"
OPENCODE_DOWNLOAD_URL="https://github.com/${REPO}/releases/download/agent_hooks-${AGENT_HOOKS_VERSION}/${OPENCODE_ASSET}"
STATUSLINE_DOWNLOAD_URL="https://github.com/${REPO}/releases/download/claude_statusline-${STATUSLINE_VERSION}/${STATUSLINE_ASSET}"

AGENT_HOOKS_TARGET_PATH="$HOOKS_DIR/$AGENT_HOOKS_BINARY_NAME"
STATUSLINE_TARGET_PATH="$HOOKS_DIR/$STATUSLINE_BINARY_NAME"
OPENCODE_TARGET_PATH="$OPENCODE_PLUGIN_DIR/agent_hooks.node"

mkdir -p "$HOOKS_DIR"
mkdir -p "$OPENCODE_PLUGIN_DIR"

NEED_AGENT_HOOKS_DOWNLOAD=1
if [ -f "$AGENT_HOOKS_TARGET_PATH" ] && [ -f "$OPENCODE_TARGET_PATH" ] && [ -f "$AGENT_HOOKS_VERSION_FILE" ]; then
  INSTALLED_AGENT_HOOKS_VERSION="$(cat "$AGENT_HOOKS_VERSION_FILE")"
  if [ "$INSTALLED_AGENT_HOOKS_VERSION" = "$AGENT_HOOKS_VERSION" ]; then
    NEED_AGENT_HOOKS_DOWNLOAD=0
  fi
fi

NEED_STATUSLINE_DOWNLOAD=1
if [ -f "$STATUSLINE_TARGET_PATH" ] && [ -f "$STATUSLINE_VERSION_FILE" ]; then
  INSTALLED_STATUSLINE_VERSION="$(cat "$STATUSLINE_VERSION_FILE")"
  if [ "$INSTALLED_STATUSLINE_VERSION" = "$STATUSLINE_VERSION" ]; then
    NEED_STATUSLINE_DOWNLOAD=0
  fi
fi

if [ "$NEED_AGENT_HOOKS_DOWNLOAD" -eq 0 ] && [ "$NEED_STATUSLINE_DOWNLOAD" -eq 0 ]; then
  echo "agent_hooks ${AGENT_HOOKS_VERSION} and claude_statusline ${STATUSLINE_VERSION} are already installed, skipping download"
  exit 0
fi

if [ "$NEED_AGENT_HOOKS_DOWNLOAD" -eq 1 ]; then
  echo "Downloading $AGENT_HOOKS_ASSET $AGENT_HOOKS_VERSION from $AGENT_HOOKS_DOWNLOAD_URL..."
  download_file "$AGENT_HOOKS_DOWNLOAD_URL" "$AGENT_HOOKS_TARGET_PATH"
  chmod +x "$AGENT_HOOKS_TARGET_PATH"
  echo "Successfully installed $AGENT_HOOKS_BINARY_NAME $AGENT_HOOKS_VERSION to $AGENT_HOOKS_TARGET_PATH"

  echo "Downloading $OPENCODE_ASSET $AGENT_HOOKS_VERSION from $OPENCODE_DOWNLOAD_URL..."
  download_file "$OPENCODE_DOWNLOAD_URL" "$OPENCODE_TARGET_PATH"
  echo "Successfully installed agent_hooks.node $AGENT_HOOKS_VERSION to $OPENCODE_TARGET_PATH"

  printf '%s\n' "$AGENT_HOOKS_VERSION" > "$AGENT_HOOKS_VERSION_FILE"
fi

if [ "$NEED_STATUSLINE_DOWNLOAD" -eq 1 ]; then
  echo "Downloading $STATUSLINE_ASSET $STATUSLINE_VERSION from $STATUSLINE_DOWNLOAD_URL..."
  download_file "$STATUSLINE_DOWNLOAD_URL" "$STATUSLINE_TARGET_PATH"
  chmod +x "$STATUSLINE_TARGET_PATH"
  echo "Successfully installed $STATUSLINE_BINARY_NAME $STATUSLINE_VERSION to $STATUSLINE_TARGET_PATH"

  printf '%s\n' "$STATUSLINE_VERSION" > "$STATUSLINE_VERSION_FILE"
fi

echo "Installation complete: agent_hooks=${AGENT_HOOKS_VERSION}, claude_statusline=${STATUSLINE_VERSION}"
