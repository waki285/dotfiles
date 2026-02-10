---
name: codex-consult
description: >-
  Consult OpenAI Codex CLI (codex exec) for second opinions, alternative approaches,
  and expert analysis. Use when: (1) user explicitly asks to consult Codex ("Codexに聞いて",
  "Codexに相談して", "ask Codex", "consult Codex"), (2) stuck on an obscure error after
  2+ failed attempts, (3) facing an architectural decision with significant trade-offs,
  (4) working with an unfamiliar library/API not covered by Context7, (5) the user's
  problem has multiple valid solutions and a second perspective would help, (6) after
  completing a plan in plan mode to validate the approach and catch blind spots.
---

# Codex Consult

## Auto-trigger Guidelines

Proactively consult Codex when:
- Stuck on an obscure error after 2+ failed attempts
- Facing an architectural decision with significant trade-offs
- Working with an unfamiliar library/API not covered by Context7
- The user's problem has multiple valid solutions and a second perspective would help
- **After completing a plan in plan mode** — consider consulting Codex to validate the approach, catch blind spots, or get alternative perspectives before presenting the plan to the user

Do NOT auto-trigger when:
- The answer is straightforward or well-known
- Context7 or web search already provides sufficient information
- Already consulted Codex in the current conversation for the same topic

## Execution

Run `codex exec` directly via the Bash tool. Do NOT use wrapper scripts.

**Timeout**: Codex can take up to 10 minutes or more for complex queries. Always set the
Bash tool's `timeout` parameter to `600000` (10 minutes) when invoking `codex exec`.

The output directory `~/.claude/second-opinions/` should already exist. If it does not,
the `-o` flag will create it implicitly via `codex exec`.

### Step 1: Run with voids provider (primary)

```bash
codex exec -c model_provider=voids --full-auto --skip-git-repo-check -o ~/.claude/second-opinions/<TIMESTAMP>_<TOPIC>.md "<PROMPT>"
```

- `<TIMESTAMP>`: Use `$(date +%Y-%m-%d_%H-%M-%S)` format
- `<TOPIC>`: Lowercase, alphanumeric + hyphens only, max 30 chars (e.g., `rust-lifetime`, `api-design`)
- `<PROMPT>`: The consultation prompt (see Prompt Formulation below)

### Step 2: Fallback (if Step 1 fails)

If the voids provider errors, retry without `-c model_provider=voids`:

```bash
codex exec --full-auto --skip-git-repo-check -o ~/.claude/second-opinions/<TIMESTAMP>_<TOPIC>.md "<PROMPT>"
```

## Prompt Formulation

Craft prompts with this structure:

```
Context: [language, framework, versions]
Problem: [clear description]
Code: [relevant snippet, under 100 lines]
Question: [specific, open-ended question]
```

Keep prompts focused. Summarize large code blocks — do not dump entire files.

## Response Handling

1. Read the saved output file with the Read tool
2. Synthesize Codex's response with your own analysis
3. Present to the user:
   - Mention that Codex was consulted
   - Show the saved file path
   - Highlight where perspectives align or differ
   - Give a combined recommendation

## Security

Never include in prompts:
- API keys, tokens, passwords, or secrets
- Personal identifiable information
- Proprietary business logic (unless the user explicitly permits it)
