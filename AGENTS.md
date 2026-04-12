# AGENTS.md

This file provides guidance to coding agents, such as Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

Personal dotfiles managed with [chezmoi](https://www.chezmoi.io/). Contains shell configs, editor settings, AI tool configurations, and a custom hook system for AI coding agents.

## Common Commands

### chezmoi

Don't do `chezmoi diff` or `chezmoi apply` without user confirmation, because it requires Bitwarden authentication.

### Permissions Generator

```bash
just perms          # Regenerate tool permissions from .chezmoidata/permissions.yaml
```

This updates:
- `dot_claude/settings.json.tmpl` (permissions block)
- `dot_codex/rules/default.rules`
- `dot_config/opencode/opencode.json`

### agent_hooks / claude_statusline (Rust)

```bash
cd tools
cargo build --workspace --release       # Build all workspace members
cargo build -p agent_hooks --release    # Build unified CLI only
cargo build -p claude_statusline --release   # Build statusline only
cargo test --workspace                  # Run tests
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint
cargo fmt --all -- --check              # Format check
```

### permissions-gen (Go)

```bash
cd tools/permissions-gen
go run .        # Run generator
go test ./...   # Run tests
go vet ./...    # Lint
```

## Architecture

### Permissions System

Tool permissions are centralized in `.chezmoidata/permissions.yaml`. The `tools/permissions-gen` Go program generates tool-specific configs from this single source of truth.

YAML structure:
- `bash`: Shared command lists (allow/ask/deny) used by all tools
- `claude`: Claude-specific permissions; `__BASH__` sentinel controls bash entry placement
- `opencode`: OpenCode-specific bash rules with pattern matching
- `codex`: Generated from bash rules

### agent_hooks

A Rust workspace providing safety hooks for AI coding agents:

```
agent_hooks/
├── core/      # Core library - pure check functions (no I/O)
├── cli/       # Unified CLI for Claude Code, Codex, and Copilot CLI
└── opencode/  # OpenCode NAPI bindings (.node file)
```

Key functions:
- `is_rm_command()` - Block rm commands
- `check_destructive_find()` - Detect dangerous find patterns
- `is_rust_file()` - Check if a file path is a Rust file
- `check_rust_allow_attributes()` - Detect #[allow(...)] in Rust code
- `check_dangerous_path_command()` - Protect configured paths
- `check_package_manager()` - Detect package manager mismatches

The unified CLI reads provider-specific JSON from stdin and outputs hook responses for Claude Code, Codex, or Copilot CLI. OpenCode uses NAPI bindings.

### claude_statusline

A Rust binary that renders a powerline-style statusline for the Claude Status hook.

Displayed segments:
- Model name
- CWD
- Project directory (when different from CWD)
- Git branch/ref
- Session cost
- Context window usage percentage

### chezmoi Naming Conventions

- `dot_` prefix → becomes `.` (e.g., `dot_zshrc` → `.zshrc`)
- `private_` prefix → sets restricted permissions
- `.tmpl` suffix → processed as Go template
- Files in `.chezmoiscripts/` run during `chezmoi apply`

## CI Workflows

Root (`.github/workflows/`):
- **ci-permissions-gen.yml**: Verifies generated files match source; runs `just perms` and checks for drift

Tools submodule (`tools/.github/workflows/`):
- **ci-agent-hooks.yml**: Runs on agent_hooks changes; check/fmt/clippy/test/build across platforms
- **ci-claude-statusline.yml**: Runs on claude_statusline changes; check/fmt/clippy/test/build
- **release-agent-hooks.yml**: Builds and releases agent_hooks binaries
- **release-claude-statusline.yml**: Builds and releases claude_statusline binaries
