# Permissions Management

This repository centralizes tool permissions in a single YAML file and
generates tool-specific configs from it.

## Source of Truth

- `.chezmoidata/permissions.yaml` is the single source of truth.
- `just perms` runs the generator to update the target files.

Generated targets:

- `dot_claude/settings.json.tmpl` (permissions block only)
- `dot_codex/rules/default.rules` (full file)
- `dot_config/opencode/opencode.json` (permission.bash only)

## YAML Schema

### bash

Shared command lists used by all tools.

```
bash:
  allow:
    - git diff
    - cargo build
  ask:
    - trash
  deny:
    - sudo
```

Entries are command prefixes. They are expanded per tool.

### claude

Claude-specific permissions and placement of shared bash entries.

```
claude:
  allow:
    - "Read(~/.claude/**)"
    - "Write(//tmp/**)"
    - __BASH__
  ask: []
  deny: []
  additionalDirectories:
    - "//tmp"
```

`__BASH__` is a sentinel that controls where bash entries are inserted.
If omitted, bash entries are appended to the end of the list.

### opencode

OpenCode uses `permission.bash` with pattern matching and last-match wins.
Put the catch-all `"*"` rule first and specific rules after it.

The generator maps:

- `opencode.bash.default` -> `permission.bash["*"]`
- `bash.allow/ask/deny` + `opencode.bash.allow/ask/deny` -> rule entries

```
opencode:
  bash:
    default: ask
    allow: []
    ask:
      - "find * -delete"
    deny: []
```

Commands without wildcards are expanded to both `cmd` and `cmd *`.
Wildcard patterns using `*` or `?` are passed through as-is.

### codex

Codex rules are generated from `bash.allow/ask/deny`.

- `allow` -> `decision = "allow"`
- `ask` -> `decision = "prompt"`
- `deny` -> `decision = "forbidden"`

Three-part commands are grouped into an alternatives list where possible.

## Regenerate

```
just perms
```
