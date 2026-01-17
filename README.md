# dotfiles

Personal dotfiles managed with chezmoi.

## Overview

This repository contains the following configuration files:

- **Shell**: zsh configuration files (`.zshrc`, `.zprofile`, `.zshenv`)
- **Git**: `.gitconfig` (GPG signing, editor settings, etc.)
- **Editors**: Vim, Zed configurations
- **Oh My Zsh**: Custom theme (`zenith`) and plugin settings
- **AI Tools**: Claude Code, Codex, Gemini configurations
- **Others**: Karabiner-Elements, Deno completions, etc.

## Requirements

- [chezmoi](https://www.chezmoi.io/) - dotfiles manager
- [Oh My Zsh](https://ohmyz.sh/) - Zsh framework
- [eza](https://github.com/eza-community/eza) - modern replacement for `ls`
- [Zed](https://zed.dev/) - text editor (used as Git editor)
- [GnuPG](https://gnupg.org/) - for Git commit signing

## Installation

```bash
# Install chezmoi
brew install chezmoi

# Apply dotfiles
chezmoi init --apply <repository-url>
```

## Usage

```bash
# Check for changes
chezmoi diff

# Apply changes
chezmoi apply

# Open source directory
chezmoi cd

# Add a file
chezmoi add ~/.config/xxx
```

## Directory Structure

```
.
├── completions/          # Shell completion files
├── dot_claude/           # Claude Code configuration
├── dot_codex/            # Codex configuration
├── dot_config/           # XDG config directory
├── dot_gemini/           # Gemini configuration
├── dot_oh-my-zsh/        # Oh My Zsh customizations
├── Library/              # macOS application settings
├── dot_gitconfig.tmpl    # Git configuration (template)
├── dot_vimrc             # Vim configuration
├── dot_zprofile          # Zsh profile
├── dot_zshenv            # Zsh environment variables
└── dot_zshrc             # Zsh configuration
```

## Key Configurations

### Shell

- Oh My Zsh with custom theme (zenith)
- Plugins: git, zsh-autosuggestions
- eza as ls alias

### Development Environment

- nvm (Node.js version manager)
- pyenv (Python version manager)
- Bun (JavaScript runtime)
- Deno (JavaScript/TypeScript runtime)
- Java 21 (for Android development)

### Git

- GPG signing for commits and tags
- Zed as default editor
- `push.autoSetupRemote = true` for automatic remote setup

## License

Personal configuration files.
