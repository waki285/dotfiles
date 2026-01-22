# dotfiles

Personal dotfiles managed with chezmoi.

## Overview

This repository contains the following configuration files:

- **Shell**: zsh configuration files (`.zshrc`, `.zprofile`)
- **Git**: `.gitconfig` (GPG signing, editor settings, etc.)
- **Editors**: Vim, Zed configurations
- **Oh My Zsh**: Custom theme (`zenith`, managed via `.chezmoiexternal.toml`) and plugin settings
- **AI Tools**: Claude Code, Codex, Gemini, OpenCode configurations
- **tools**: Development tools as a Git submodule (see [dotfiles-tools](https://github.com/waki285/dotfiles-tools))
  - **agent_hooks**: Custom hook system for Claude Code and OpenCode
  - **permissions-gen**: Tool permission generator
- **Others**: Karabiner-Elements, Deno completions, VS Code prompt instructions, etc.

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

## Permissions Management

Tool permissions are centralized in `.chezmoidata/permissions.yaml` and
generated into tool-specific configs.

- Generator: `tools/permissions-gen`
- Run: `just perms`
- Details: `docs/permissions.md`

## Directory Structure

```
.
├── .chezmoiscripts/      # Post-apply scripts (run_after_*.sh, run_after_*.ps1)
├── AppData/              # Windows application settings (Zed)
├── completions/          # Shell completion files
├── docs/                 # Documentation (permissions.md, etc.)
├── dot_claude/           # Claude Code configuration
├── dot_codex/            # Codex configuration
├── dot_config/           # XDG config directory (opencode, karabiner, etc.)
├── dot_gemini/           # Gemini configuration
├── Library/              # macOS application settings (VS Code prompts)
├── tools/                # Development tools (Git submodule -> dotfiles-tools)
├── dot_gitconfig.tmpl    # Git configuration (template)
├── dot_vimrc.tmpl        # Vim configuration (Unix)
├── _vimrc.tmpl           # Vim configuration (Windows)
├── dot_zprofile          # Zsh profile
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

### Claude Code Hooks

Custom hooks for Claude Code that provide safety checks. Each hook type has a single command with module flags:

```bash
# permission-request: Bash command checks
agent_hooks permission-request --block-rm --confirm-destructive-find --dangerous-paths "~/"

# pre-tool-use: Edit/Write tool checks
agent_hooks pre-tool-use --deny-rust-allow --expect --check-package-manager
```

Available modules:

| Hook Type | Flag | Description |
|-----------|------|-------------|
| `permission-request` | `--block-rm` | Prevents `rm` commands, suggests `trash` instead |
| `permission-request` | `--confirm-destructive-find` | Confirms destructive `find` commands |
| `permission-request` | `--dangerous-paths <paths>` | Protects specified paths from rm/trash/mv |
| `pre-tool-use` | `--deny-rust-allow` | Prevents `#[allow(...)]` attributes in Rust files |
| `pre-tool-use` | `--expect` | With `--deny-rust-allow`: allow `#[expect]`, deny `#[allow]` |
| `pre-tool-use` | `--additional-context <msg>` | Appends custom message to denial reason |
| `pre-tool-use` | `--check-package-manager` | Denies mismatched package manager commands |

See [tools/agent_hooks/README.md](tools/agent_hooks/README.md) for details.

## License

- **Repository (excluding third-party files listed in THIRD_PARTY.md)**: Apache License 2.0 (see [LICENSE](LICENSE))
- **Third-party components**: see [THIRD_PARTY.md](THIRD_PARTY.md)
- **agent_hooks/**: Apache License 2.0 (see [tools/agent_hooks/LICENSE](tools/agent_hooks/LICENSE))
- **tools/**: Apache License 2.0 (see [tools/LICENSE](tools/LICENSE))
