#!/bin/sh
set -eu

# zsh-syntax-highlighting is not bundled with oh-my-zsh, so install it into
# $ZSH_CUSTOM/plugins when missing (required by the plugins list in .zshrc).

ZSH_CUSTOM="${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}"
PLUGIN_DIR="$ZSH_CUSTOM/plugins/zsh-syntax-highlighting"

if [ ! -d "$HOME/.oh-my-zsh" ]; then
  echo "oh-my-zsh not found, skipping zsh-syntax-highlighting install"
  exit 0
fi

if [ -d "$PLUGIN_DIR/.git" ]; then
  echo "zsh-syntax-highlighting already installed"
  exit 0
fi

if ! command -v git >/dev/null 2>&1; then
  echo "Error: git is not available; cannot install zsh-syntax-highlighting" >&2
  exit 1
fi

mkdir -p "$ZSH_CUSTOM/plugins"
git clone --depth 1 https://github.com/zsh-users/zsh-syntax-highlighting.git "$PLUGIN_DIR"
echo "Installed zsh-syntax-highlighting to $PLUGIN_DIR"
