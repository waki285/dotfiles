# agent_hooks

A Rust-based hook system for AI coding agents that provides safety checks and restrictions.

## Architecture

```
agent_hooks/
├── core/           # Core library - pure check functions
├── claude/         # Claude Code CLI (agent_hooks_claude)
└── opencode/       # OpenCode NAPI bindings (agent_hooks_opencode)
```

## Features

### Bash Command Checks

- **block-rm**: Blocks `rm` commands and suggests using `trash` instead
- **confirm-destructive-find**: Detects destructive `find` commands (e.g., `find -delete`, `find -exec rm`)

### Rust Code Checks (Edit/Write)

- **deny-rust-allow**: Denies adding `#[allow(...)]` or `#[expect(...)]` attributes to Rust files
  - Ignores comments (`//`, `/* */`) and string literals
  - Configurable to allow `#[expect(...)]` while denying `#[allow(...)]`
  - Supports custom messages

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

### Claude Code

#### Configuration

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

#### CLI Flags

##### `permission-request` command

| Flag | Description |
|------|-------------|
| `--block-rm` | Block `rm` commands and suggest using `trash` instead |
| `--confirm-destructive-find` | Ask for confirmation on destructive `find` commands |

##### `pre-tool-use` command

| Flag | Description |
|------|-------------|
| `--deny-rust-allow` | Deny `#[allow(...)]` attributes in Rust files |
| `--expect` | With `--deny-rust-allow`: allow `#[expect(...)]` while denying `#[allow(...)]` |
| `--additional-context <string>` | With `--deny-rust-allow`: append custom message to the denial reason |

#### CLI Examples

```bash
# Block rm commands
echo '{"tool_name":"Bash","tool_input":{"command":"rm -rf /tmp/test"}}' | \
  agent_hooks_claude permission-request --block-rm

# Deny #[allow] in Rust files, allow #[expect]
echo '{"tool_name":"Edit","tool_input":{"file_path":"src/main.rs","new_string":"#[allow(dead_code)]"}}' | \
  agent_hooks_claude pre-tool-use --deny-rust-allow --expect
```

### OpenCode

#### Configuration

Add to your `opencode.json`:

```json
{
  "agentHooks": {
    "allowExpect": true,
    "additionalContext": "See project guidelines"
  }
}
```

#### Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allowExpect` | boolean | `false` | Allow `#[expect(...)]` while denying `#[allow(...)]` |
| `additionalContext` | string | - | Custom message to append to denial errors |

#### Plugin Setup

1. Place `agent_hooks.node` in `~/.config/opencode/plugin/`
2. Place `agent_hooks.ts` in `~/.config/opencode/plugin/`
3. Configure options in `opencode.json` (see above)

The plugin automatically:
- Blocks `rm` commands
- Warns on destructive `find` commands
- Denies `#[allow(...)]` / `#[expect(...)]` in Rust files based on configuration

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

## Core API

The core library exports simple check functions that can be used by any client:

```rust
// Check if a command contains rm
pub fn is_rm_command(cmd: &str) -> bool

// Check for destructive find commands, returns description if found
pub fn check_destructive_find(cmd: &str) -> Option<&'static str>

// Check if a file path is a Rust file
pub fn is_rust_file(file_path: &str) -> bool

// Check for #[allow(...)] / #[expect(...)] attributes
pub fn check_rust_allow_attributes(content: &str) -> RustAllowCheckResult
```

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

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.
