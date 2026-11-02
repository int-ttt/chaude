---
name: check
description: Verify that a change was applied to every target file/version, plus char limits and fact consistency. Use for 바꾼거 맞아?, 했어?, 안 바뀐 것 같은데.
argument-hint: "<what changed> [target paths]"
context: fork
agent: haiku-checker
model: haiku
background: false
---

Verify: $ARGUMENTS
If targets are not named, find every file that should contain this content with `grep -rl` over the document folders (exclude .git, logs) and check each.
Also run char-limit checks for any section with a known limit in `notes/DECISIONS.md`.
Return ≤5 lines: FAIL items first (file:line), then "나머지 N개 PASS". Full table → `notes/runs/`.
