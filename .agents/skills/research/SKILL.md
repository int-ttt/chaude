---
name: research
description: Web research saved as a sourced table - official numbers, requirements, deadlines, prices, docs, release notes, service/tool limits.
argument-hint: "<question>"
context: fork
agent: haiku-researcher
model: haiku
background: false
---

Research: $ARGUMENTS
Prefer official sources. Never invent numbers; list unverified items separately.
Save `notes/research/<short-topic>.md` (table + source URLs).
Return ≤5 lines: key numbers and the saved path.
