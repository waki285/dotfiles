---
name: clippy-fix
description: >
  Run cargo clippy on Rust projects and auto-fix warnings. Use when the user asks to
  "run clippy", "lint", "check for warnings", "run lints", or any request to perform
  static analysis on Rust code. Also triggers via /clippy-fix slash command.
  IMPORTANT: Also use this skill proactively whenever the agent decides to run
  `cargo clippy` on its own (e.g., after writing Rust code, before committing,
  or as part of a CI-like check workflow). Always load this skill before executing
  any `cargo clippy` command.
---

# Clippy Fix

Run `cargo clippy` with appropriate flags based on the project structure, then auto-fix any warnings found.

## Workflow

### 1. Detect project structure

Read `Cargo.toml` in the current directory (or the directory the user specifies).

Determine:
- **Workspace?** Check for `[workspace]` section. If present, use `--workspace`.
- **Features?** Check for `[features]` section. If non-trivial features exist that gate conditional compilation, use `--all-features`. If the user specifies particular features, use `--features <list>` instead.
- **Targets?** Always use `--all-targets` to cover lib, bins, tests, examples, and benches.

### 2. Build the command

Base command:

```
cargo clippy <scope> --all-targets <features> -- -D warnings
```

Where:
- `<scope>` = `--workspace` if workspace, omitted otherwise
- `<features>` = `--all-features` or `--features <list>` or omitted

Examples:
- Single crate, no features: `cargo clippy --all-targets -- -D warnings`
- Workspace, no features: `cargo clippy --workspace --all-targets -- -D warnings`
- Workspace with features: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Specific features: `cargo clippy --all-targets --features "serde,async" -- -D warnings`

### 3. Run clippy

Execute the command via Bash. If clippy exits with errors, parse the output.

### 4. Auto-fix warnings

For each warning/error reported by clippy:

1. Read the file at the reported location.
2. Apply the fix suggested by clippy (most clippy lints include a "help:" or "suggestion:" line).
3. If the suggestion is ambiguous or a non-trivial refactor, ask the user before applying.

After fixing, re-run clippy to confirm all warnings are resolved. Repeat until clean or only non-auto-fixable issues remain.

### 5. Report results

Summarize what was fixed:
- Number of warnings found and fixed
- Any remaining warnings that require manual intervention
