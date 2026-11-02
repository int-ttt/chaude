---
name: fact
description: Apply a fact correction everywhere — update notes/FACTS.md and fix every file containing the wrong fact. Use for "A가 아니라 B야" (it's B, not A) style corrections.
argument-hint: "<old> → <new> [context]"
context: fork
agent: haiku-batch-worker
model: haiku
background: false
---

Think and write in English; only haiku-tui writes Korean to the user.

Fact correction: $ARGUMENTS
1. Update or add the line in `notes/FACTS.md` (format: `- [area] fact (source: user)`).
2. `grep -rn` the old value across documents (exclude .git, logs).
3. Replace only where the context matches this fact; list ambiguous occurrences unchanged.
Return ≤5 lines: files changed with counts, ambiguous occurrences (file:line).
