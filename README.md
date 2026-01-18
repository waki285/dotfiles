# dotfiles

Personal dotfiles managed with chezmoi.

## Overview

This repository contains the following configuration files:

- **Shell**: zsh configuration files (`.zshrc`, `.zprofile`, `.zshenv`)
- **Git**: `.gitconfig` (GPG signing, editor settings, etc.)
- **Editors**: Vim, Zed configurations
- **Oh My Zsh**: Custom theme (`zenith`) and plugin settings
- **AI Tools**: Claude Code, Codex, Gemini configurations
- **claude_hooks**: Custom hook system for Claude Code (see [claude_hooks/README.md](claude_hooks/README.md))
- **Others**: Karabiner-Elements, Deno completions, etc.

## Requirements

- [chezmoi](https://www.chezmoi.io/) - dotfiles manager
- [Oh My Zsh](https://ohmyz.sh/) - Zsh framework
- [eza](https://github.com/eza-community/eza) - modern replacement for `ls`
- [Zed](https://zed.dev/) - text editor (used as Git editor)
- [GnuPG](https://gnupg.org/) - for Git commit signing
- [Bitwarden CLI](https://bitwarden.com/help/cli/) - for secrets management

## Platform Support

| Platform | Support |
|----------|---------|
| macOS (arm64) | Full |
| Linux (x86_64, arm64) | Partial (shell, git, editors) |
| Windows | Partial (git, editors, AI tools) |

## Installation

```bash
# Install chezmoi
brew install chezmoi  # macOS
# or
sh -c "$(curl -fsLS get.chezmoi.io)"  # Linux/Windows

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
├── claude_hooks/         # Claude Code hook system (Apache 2.0 licensed)
├── completions/          # Shell completion files
├── dot_claude/           # Claude Code configuration
├── dot_codex/            # Codex configuration
├── dot_config/           # XDG config directory
├── dot_gemini/           # Gemini configuration
├── dot_oh-my-zsh/        # Oh My Zsh customizations
├── Library/              # macOS application settings
├── dot_gitconfig.tmpl    # Git configuration (template)
├── dot_vimrc.tmpl        # Vim configuration (Unix)
├── _vimrc.tmpl           # Vim configuration (Windows)
├── dot_zprofile          # Zsh profile
├── dot_zshenv            # Zsh environment variables
├── dot_zshrc             # Zsh configuration
├── run_after_*.sh        # Post-apply scripts (Unix)
└── run_after_*.ps1       # Post-apply scripts (Windows)
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

### Claude Code Hooks

Custom hooks for Claude Code that provide safety checks. Each hook type has a single command with module flags:

```bash
# permission-request: Bash command checks
claude_hooks permission-request --block-rm --confirm-destructive-find

# pre-tool-use: Edit/Write tool checks
claude_hooks pre-tool-use --deny-rust-allow --expect
```

Available modules:

| Hook Type | Flag | Description |
|-----------|------|-------------|
| `permission-request` | `--block-rm` | Prevents `rm` commands, suggests `trash` instead |
| `permission-request` | `--confirm-destructive-find` | Confirms destructive `find` commands |
| `pre-tool-use` | `--deny-rust-allow` | Prevents `#[allow(...)]` attributes in Rust files |
| `pre-tool-use` | `--expect` | With `--deny-rust-allow`: allow `#[expect]`, deny `#[allow]` |

See [claude_hooks/README.md](claude_hooks/README.md) for details.

## License

- **claude_hooks/**: Apache License 2.0 (see [claude_hooks/LICENSE](claude_hooks/LICENSE))
- **Other files**: Personal configuration files (no license)
