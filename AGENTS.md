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

### agent_hooks (Rust)

```bash
cd agent_hooks
cargo build --release           # Build all packages
cargo build -p agent_hooks_claude --release  # Build Claude CLI only
cargo test                      # Run tests
cargo clippy --all-targets --all-features -- -D warnings     # Lint
cargo fmt --all -- --check      # Format check
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
├── claude/    # Claude Code CLI binary (agent_hooks_claude)
└── opencode/  # OpenCode NAPI bindings (.node file)
```

Core functions:
- `is_rm_command()` - Block rm commands
- `check_destructive_find()` - Detect dangerous find patterns
- `check_rust_allow_attributes()` - Detect #[allow(...)] in Rust code
- `check_dangerous_path_command()` - Protect configured paths
- `check_package_manager()` - Detect package manager mismatches

The Claude CLI reads JSON from stdin and outputs hook responses. OpenCode uses NAPI bindings.

### chezmoi Naming Conventions

- `dot_` prefix → becomes `.` (e.g., `dot_zshrc` → `.zshrc`)
- `private_` prefix → sets restricted permissions
- `.tmpl` suffix → processed as Go template
- Files in `.chezmoiscripts/` run during `chezmoi apply`

## CI Workflows

- **ci-agent-hooks.yml**: Runs on agent_hooks changes; check/fmt/clippy/test/build across platforms
- **ci-permissions-gen.yml**: Verifies generated files match source; runs `just perms` and checks for drift
- **release-agent-hooks.yml**: Builds and releases agent_hooks binaries
