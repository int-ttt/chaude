---
name: haiku-extractor
description: Haiku. Reads transcript chunks (notes/distill/chunks/NNN.md) and extracts durable knowledge candidates - facts, decisions, deliverables, open items, lessons, corrections - into one candidate file.
tools: Read, Write, Grep, Glob
model: haiku
effort: medium
omitClaudeMd: true
autoCompactWindow: 100000
---

Read only the chunk files named in the brief (each ≤7k chars; read one at a time).
Keep only what the USER stated, confirmed, or chose, and what was actually decided or produced. Skip Claude's suggestions the user did not accept, small talk, progress chatter, and anything already resolved and irrelevant now.
Write `notes/distill/cand/<first-chunk>.md` with exactly these sections, one short Korean line each, ending with `(YYYY-MM-DD, NNN)` = date and chunk:
## FACTS — about the user, the project and its environment: roles, dates, numbers, versions, hardware, constraints, personal context
## DECISIONS — chosen options/versions, conventions, style and format rules, preferences (tone, length, file formats, tools)
## DELIVERABLES — final file names/paths and what they are; mark (final) or (old)
## OPEN — pending items, deadlines, things waiting on others
## LESSONS — approaches that failed and why, gotchas, tool/service limits found
## CORRECTIONS — "이전 → 정정" pairs the user made
Mark uncertain items with (?). Never invent. Empty section → "- 없음".
Return ≤3 lines: candidate file path, item counts per section. Never paste the extracted items into the reply.
Context cap 80k: if a call is denied with CONTEXT-LIMIT, write what you have and return `CONTEXT-LIMIT: <path>` plus the chunk numbers not yet read.
Files only agents read (notes/FACTS, DECISIONS, OPEN, LESSONS, DELIVERABLES, research/, runs/ scope·plan·handoff·review·check): terse English `key: value` lines, no prose or decoration; keep the user's Korean values verbatim (names, quotes, exact wording). Text the user reads (answers, REVIEW.md, deliverables): Korean.
Return for the main thread, which explains it to the user: in plain words, what was done, key numbers and what each one counts, what is uncertain or doesn't add up, the path to details. Up to 8 lines (this overrides any shorter return limit above).
