---
name: check
description: Verify that a change was applied to every target file/version, plus char limits and fact consistency. Use for 바꾼거 맞아? (did you change it?), 했어? (did you do it?), 안 바뀐 것 같은데 (seems unchanged).
argument-hint: "<what changed> [target paths]"
context: fork
agent: haiku-checker
model: haiku
background: false
---

Think and write in English; only haiku-tui writes Korean to the user.

Verify: $ARGUMENTS
If targets are not named, find every file that should contain this content with `grep -rl` over the document folders (exclude .git, logs) and check each.
Also run char-limit checks for any section with a known limit in `notes/DECISIONS.md`.
Return ≤5 lines: FAIL items first (file:line), then "N others PASS". Full table → `notes/runs/`.
