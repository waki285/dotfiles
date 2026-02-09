---
name: codex-consult
description: >-
  Consult OpenAI Codex CLI (codex exec) for second opinions, alternative approaches,
  and expert analysis. Use when: (1) user explicitly asks to consult Codex ("Codexに聞いて",
  "Codexに相談して", "ask Codex", "consult Codex"), (2) debugging obscure errors after
  initial attempts fail, (3) complex algorithmic or architectural decisions with multiple
  valid approaches, (4) unfamiliar libraries/APIs where Context7 lacks coverage, (5) wanting
  to validate a non-trivial solution before presenting it.
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

Run the wrapper script via Bash:

```bash
bash <skill-dir>/scripts/codex-consult.sh "<prompt>" "<topic-slug>"
```

Where `<skill-dir>` is this skill's directory. The script:
1. Tries `codex exec -c model_provider=voids --full-auto --skip-git-repo-check` first
2. Falls back to default provider on error
3. Saves the response to `~/.claude/second-opinions/YYYY-MM-DD_HH-MM-SS_<topic>.md`
4. Prints the output file path to stdout

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
