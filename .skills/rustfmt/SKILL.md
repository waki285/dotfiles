---
name: rustfmt
description: >
  Run cargo fmt on Rust projects to format code. Use when the user asks to
  "format", "run fmt", "format code", "rustfmt", or any request to auto-format
  Rust source code. Also triggers via /rustfmt slash command.
  IMPORTANT: Also use this skill proactively whenever the agent decides to run
  `cargo fmt` on its own (e.g., after writing Rust code, before committing,
  or as part of a CI-like check workflow). Always load this skill before executing
  any `cargo fmt` command.
---

# Rustfmt

Run `cargo fmt` with appropriate flags based on the project structure, then verify formatting is clean.

## Workflow

### 1. Detect project structure

Read `Cargo.toml` in the current directory (or the directory the user specifies).

Determine:
- **Workspace?** Check for `[workspace]` section. If present, use `--all`.

### 2. Build and run the command

Base command:

```
cargo fmt <scope> -- --check
```

Where:
- `<scope>` = `--all` if workspace, omitted otherwise

Run the `--check` variant first to see what needs formatting.

### 3. Detect nightly requirement

Check the output (both stdout and stderr) for **any** of these indicators, regardless of exit code:
- `Warning: can't set` ... `unstable features are only available in nightly channel`
- `error: Unknown option`
- `requires nightly`
- Any mention of `unstable` + `nightly` in the output

If **any** of these appear, the project's `rustfmt.toml`/`.rustfmt.toml` uses nightly-only options. All subsequent `cargo fmt` invocations in this session MUST use `+nightly`.

### 4. Apply formatting

Run without `--check` to apply formatting:

```
cargo +nightly fmt <scope>   # if nightly required (from step 3)
cargo fmt <scope>            # otherwise
```

### 5. Report results

Summarize:
- Number of files reformatted
- Whether nightly toolchain was required
- Any errors that could not be resolved
