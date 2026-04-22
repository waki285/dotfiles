---
name: split-rust-file
description: >
  Split large Rust source files into module-based layouts. Use when the user
  asks to split a specific Rust file, or when you need to reduce oversized Rust
  files and no file was specified, especially for files over 1000 lines. Prefer
  module extraction, never use `include!`, never leave `A.rs` alongside `A/`,
  never create `A/A.rs`, and keep `mod.rs` limited to module declarations and
  re-exports.
---

# Split Rust File

Refactor large Rust files into normal Rust modules while keeping the public API
and module paths coherent.

## When To Use

- The user explicitly asks to split a Rust file.
- No file was specified, and you need to split a Rust file that exceeds 1000 lines.

If no file was specified:

1. Enumerate Rust files under the current project.
2. Count lines.
3. Pick files over 1000 lines as candidates.
4. If there is exactly one clear candidate, use it.
5. If there are multiple plausible candidates, ask the user which file to split.

Do not guess when the target file is ambiguous.

Count lines with platform-appropriate commands:

### macOS/Linux

Use `find` with `wc -l`, then filter candidates over 1000 lines:

```bash
find . -type f -name '*.rs' -exec wc -l {} + | awk '$1 > 1000'
```

If you want to inspect all Rust files before filtering:

```bash
find . -type f -name '*.rs' -exec wc -l {} + | sort -n
```

### Windows

Use `Get-ChildItem` and `[System.IO.File]::ReadAllLines(...).Length` to count
physical lines per file, including blank lines:

```powershell
Get-ChildItem -Recurse -Filter *.rs |
  ForEach-Object {
    [pscustomobject]@{
      Lines = [System.IO.File]::ReadAllLines($_.FullName).Length
      Path = $_.FullName
    }
  } |
  Where-Object { $_.Lines -gt 1000 } |
  Sort-Object Lines
```

If you want to inspect all Rust files before filtering, remove the `Where-Object`
stage.

## Required Rules

- Split by Rust modules. Do not use `include!`.
- Do not create layouts where `./A/` and `./A.rs` coexist.
- Do not create `./A/A.rs`.
- Do not place constants, statics, type definitions, trait definitions, impl
  blocks, functions, or executable logic in `mod.rs`.
- Keep `mod.rs` as a thin routing layer with module declarations and re-exports.

## Preferred Layouts

If the original file is `foo.rs`, convert it into a directory module:

```text
before:
src/foo.rs

after:
src/foo/mod.rs
src/foo/types.rs
src/foo/parse.rs
src/foo/render.rs
```

If the original file is already `foo/mod.rs`, keep `foo/mod.rs` thin and move
definitions into sibling files:

```text
src/foo/mod.rs
src/foo/types.rs
src/foo/parse.rs
src/foo/render.rs
```

Forbidden layouts:

```text
src/foo.rs
src/foo/types.rs
```

```text
src/foo/foo.rs
```

## Workflow

### 1. Understand the file before moving code

- Read the full file.
- Identify natural responsibility boundaries such as types, parsing, rendering,
  runtime orchestration, helpers, and tests.
- Check parent and sibling modules so that visibility, re-exports, and module
  paths remain valid after the split.

### 2. Design the module boundaries

- Split by responsibility, not by arbitrary line ranges.
- Prefer a small number of meaningful modules over many tiny files.
- Keep closely coupled private helpers with the module that owns them.
- Preserve stable external paths when practical by re-exporting from `mod.rs`.

### 3. Build the module tree

- For `foo.rs`, move it to `foo/mod.rs` and add child modules under `foo/`.
- For existing directory modules, keep `foo/mod.rs` and add or expand sibling files.
- Declare child modules from `mod.rs`.
- Re-export public items from `mod.rs` when that avoids unnecessary API churn.

`mod.rs` should typically look like this:

```rust
mod parse;
mod render;
mod types;

pub use parse::Parser;
pub use render::Renderer;
pub use types::{AstNode, ParseError};
```

### 4. Move code carefully

- Move definitions into the most cohesive child module.
- Update `use`, `pub`, `pub(crate)`, and `super::` paths after each move.
- Avoid circular dependencies. If two modules keep needing each other, the split
  is probably wrong.
- Keep tests near the module they validate when possible.

### 5. Validate

- Run `cargo fmt`.
- Run `cargo check` at minimum.
- Run targeted tests when the split affects behavior or public APIs.

Do not consider the refactor complete until the crate builds cleanly.

## Heuristics

- Good split candidates: files that mix data types, parsing, rendering, CLI
  entry logic, protocol conversions, or unrelated helper groups.
- Bad split candidates: files that are long only because of one tightly coupled
  algorithm or one large match table.
- If the file is large but the code is still one cohesive unit, explain that
  instead of forcing a poor module structure.

## Reporting

Summarize:

- which file was split
- which modules were created
- whether any public paths changed
- which validation commands were run
