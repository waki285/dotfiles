# claude_hooks

A Rust-based hook system for Claude Code that provides safety checks and restrictions.

## Features

### PermissionRequest Hooks (Bash)

- **block-rm**: Blocks `rm` commands and suggests using `trash` instead
- **confirm-destructive-find**: Asks for confirmation when destructive `find` commands are detected (e.g., `find -delete`, `find -exec rm`)

### PreToolUse Hooks (Edit/Write)

- **deny-rust-allow**: Denies adding `#[allow(...)]` or `#[expect(...)]` attributes to Rust files
  - Ignores comments (`//`, `/* */`) and string literals
  - Supports `--expect` flag to allow `#[expect(...)]` while denying `#[allow(...)]`
  - Supports `--additional-context` flag for custom messages

## Installation

Pre-built binaries are available from GitHub Releases. The `run_after_20_claude-hooks.sh` (Unix) or `run_after_20_claude-hooks.ps1` (Windows) scripts will automatically download the latest version.

### Manual Installation

```bash
# Download the binary for your platform
curl -fsSL -o ~/.claude/hooks/claude_hooks \
  https://github.com/waki285/dotfiles/releases/download/claude_hooks-vX.Y.Z/claude_hooks-<platform>

chmod +x ~/.claude/hooks/claude_hooks
```

## Usage

### Command Line

Each hook type has a single command with module flags to enable specific features.

```bash
# permission-request: Handle Bash command permission checks
claude_hooks permission-request --block-rm --confirm-destructive-find

# pre-tool-use: Handle Edit/Write tool checks
claude_hooks pre-tool-use --deny-rust-allow [--expect] [--additional-context "..."]
```

#### Examples

```bash
# Block rm commands (PermissionRequest hook)
echo '{"tool_name":"Bash","tool_input":{"command":"rm -rf /tmp/test"}}' | \
  claude_hooks permission-request --block-rm

# Enable both rm blocking and destructive find confirmation
echo '{"tool_name":"Bash","tool_input":{"command":"find . -delete"}}' | \
  claude_hooks permission-request --block-rm --confirm-destructive-find

# Deny #[allow] in Rust files (PreToolUse hook)
echo '{"tool_name":"Edit","tool_input":{"file_path":"src/main.rs","new_string":"#[allow(dead_code)]"}}' | \
  claude_hooks pre-tool-use --deny-rust-allow

# With --expect flag (allow #[expect], deny #[allow])
echo '...' | claude_hooks pre-tool-use --deny-rust-allow --expect

# With additional context
echo '...' | claude_hooks pre-tool-use --deny-rust-allow --additional-context "See project guidelines"
```

### Claude Code Configuration

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "$HOME/.claude/hooks/claude_hooks pre-tool-use --deny-rust-allow --expect"
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "$HOME/.claude/hooks/claude_hooks permission-request --block-rm --confirm-destructive-find"
          }
        ]
      }
    ]
  }
}
```

### Available Flags

#### `permission-request` command

| Flag | Description |
|------|-------------|
| `--block-rm` | Block `rm` commands and suggest using `trash` instead |
| `--confirm-destructive-find` | Ask for confirmation on destructive `find` commands |

#### `pre-tool-use` command

| Flag | Description |
|------|-------------|
| `--deny-rust-allow` | Deny `#[allow(...)]` attributes in Rust files |
| `--expect` | With `--deny-rust-allow`: allow `#[expect(...)]` while denying `#[allow(...)]` |
| `--additional-context <string>` | With `--deny-rust-allow`: append custom message to the denial reason |

## Supported Platforms

| Platform | Architecture | Binary Name |
|----------|--------------|-------------|
| macOS | x86_64 | `claude_hooks-macos-x86_64` |
| macOS | arm64 | `claude_hooks-macos-arm64` |
| Linux | x86_64 | `claude_hooks-linux-x86_64` |
| Linux | arm64 | `claude_hooks-linux-arm64` |
| Windows | x86_64 | `claude_hooks-windows-x86_64.exe` |
| Windows | arm64 | `claude_hooks-windows-arm64.exe` |

Linux binaries are statically linked with musl, and Windows binaries are statically linked with CRT for maximum compatibility.

## Building from Source

```bash
cd claude_hooks
cargo build --release
```

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.
