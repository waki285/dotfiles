# agent_hooks

A Rust-based hook system for AI coding agents that provides safety checks and restrictions.

## Architecture

```
agent_hooks/
├── core/           # Core library (agent_hooks)
├── claude/         # Claude Code CLI (agent_hooks_claude)
└── opencode/       # OpenCode NAPI bindings (agent_hooks_opencode)
```

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

Pre-built binaries are available from GitHub Releases. The `run_after_20_agent-hooks.sh` (Unix) or `run_after_20_agent-hooks.ps1` (Windows) scripts will automatically download the latest version.

### Manual Installation

#### Claude CLI

```bash
# Download the binary for your platform
curl -fsSL -o ~/.claude/hooks/agent_hooks_claude \
  https://github.com/waki285/dotfiles/releases/download/agent_hooks-vX.Y.Z/agent_hooks_claude-<platform>

chmod +x ~/.claude/hooks/agent_hooks_claude
```

#### OpenCode Plugin

```bash
# Download the .node file for your platform
curl -fsSL -o ~/.config/opencode/plugin/agent_hooks.node \
  https://github.com/waki285/dotfiles/releases/download/agent_hooks-vX.Y.Z/agent_hooks_opencode-<platform>.node
```

## Usage

### Command Line (Claude CLI)

Each hook type has a single command with module flags to enable specific features.

```bash
# permission-request: Handle Bash command permission checks
agent_hooks_claude permission-request --block-rm --confirm-destructive-find

# pre-tool-use: Handle Edit/Write tool checks
agent_hooks_claude pre-tool-use --deny-rust-allow [--expect] [--additional-context "..."]
```

#### Examples

```bash
# Block rm commands (PermissionRequest hook)
echo '{"tool_name":"Bash","tool_input":{"command":"rm -rf /tmp/test"}}' | \
  agent_hooks_claude permission-request --block-rm

# Enable both rm blocking and destructive find confirmation
echo '{"tool_name":"Bash","tool_input":{"command":"find . -delete"}}' | \
  agent_hooks_claude permission-request --block-rm --confirm-destructive-find

# Deny #[allow] in Rust files (PreToolUse hook)
echo '{"tool_name":"Edit","tool_input":{"file_path":"src/main.rs","new_string":"#[allow(dead_code)]"}}' | \
  agent_hooks_claude pre-tool-use --deny-rust-allow

# With --expect flag (allow #[expect], deny #[allow])
echo '...' | agent_hooks_claude pre-tool-use --deny-rust-allow --expect

# With additional context
echo '...' | agent_hooks_claude pre-tool-use --deny-rust-allow --additional-context "See project guidelines"
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
            "command": "$HOME/.claude/hooks/agent_hooks_claude pre-tool-use --deny-rust-allow --expect"
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
            "command": "$HOME/.claude/hooks/agent_hooks_claude permission-request --block-rm --confirm-destructive-find"
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

### Claude CLI

| Platform | Architecture | Binary Name |
|----------|--------------|-------------|
| macOS | x86_64 | `agent_hooks_claude-macos-x86_64` |
| macOS | arm64 | `agent_hooks_claude-macos-arm64` |
| Linux | x86_64 | `agent_hooks_claude-linux-x86_64` |
| Linux | arm64 | `agent_hooks_claude-linux-arm64` |
| Windows | x86_64 | `agent_hooks_claude-windows-x86_64.exe` |
| Windows | arm64 | `agent_hooks_claude-windows-arm64.exe` |

Linux binaries are statically linked with musl, and Windows binaries are statically linked with CRT for maximum compatibility.

### OpenCode NAPI

| Platform | Architecture | Binary Name |
|----------|--------------|-------------|
| macOS | x86_64 | `agent_hooks_opencode-macos-x86_64.node` |
| macOS | arm64 | `agent_hooks_opencode-macos-arm64.node` |
| Linux | x86_64 | `agent_hooks_opencode-linux-x86_64.node` |
| Linux | arm64 | `agent_hooks_opencode-linux-arm64.node` |
| Windows | x86_64 | `agent_hooks_opencode-windows-x86_64.node` |
| Windows | arm64 | `agent_hooks_opencode-windows-arm64.node` |

## Building from Source

```bash
cd agent_hooks

# Build all packages
cargo build --release

# Build Claude CLI only
cargo build -p agent_hooks_claude --release

# Build OpenCode NAPI only
cargo build -p agent_hooks_opencode --release

# Run tests
cargo test
```

### OpenCode .node Installation from Source

```bash
cd agent_hooks
cargo build -p agent_hooks_opencode --release

# macOS
cp target/release/libagent_hooks_opencode.dylib ~/.config/opencode/plugin/agent_hooks.node

# Linux
cp target/release/libagent_hooks_opencode.so ~/.config/opencode/plugin/agent_hooks.node
```

```powershell
# Windows
Copy-Item target\release\agent_hooks_opencode.dll "$env:USERPROFILE\.config\opencode\plugin\agent_hooks.node"
```

If you place the .node file elsewhere, set `OPENCODE_AGENT_HOOKS_NODE` to the full path. Optional: `OPENCODE_AGENT_HOOKS_EXPECT=1` to allow #[expect(...)] and `OPENCODE_AGENT_HOOKS_CONTEXT="..."` to append a custom message.

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.
