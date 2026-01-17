#!/bin/sh
set -eu

# Download claude_hooks binary from GitHub releases

REPO="waki285/dotfiles"
VERSION="v0.1.1"
HOOKS_DIR="$HOME/.claude/hooks"
BINARY_NAME="claude_hooks"
VERSION_FILE="$HOOKS_DIR/.claude_hooks_version"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64)
        ASSET_NAME="claude_hooks-macos-arm64"
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
        ASSET_NAME="claude_hooks-linux-x86_64"
        ;;
      aarch64|arm64)
        ASSET_NAME="claude_hooks-linux-arm64"
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

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"
TARGET_PATH="$HOOKS_DIR/$BINARY_NAME"

# Check if already installed with correct version
if [ -f "$TARGET_PATH" ] && [ -f "$VERSION_FILE" ]; then
  INSTALLED_VERSION="$(cat "$VERSION_FILE")"
  if [ "$INSTALLED_VERSION" = "$VERSION" ]; then
    echo "claude_hooks $VERSION is already installed, skipping download"
    exit 0
  fi
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Download the binary
echo "Downloading $ASSET_NAME from $DOWNLOAD_URL..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "$TARGET_PATH" "$DOWNLOAD_URL"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$TARGET_PATH" "$DOWNLOAD_URL"
else
  echo "Error: Neither curl nor wget is available" >&2
  exit 1
fi

# Make it executable
chmod +x "$TARGET_PATH"

# Save version file
printf '%s\n' "$VERSION" > "$VERSION_FILE"

echo "Successfully installed $BINARY_NAME $VERSION to $TARGET_PATH"
