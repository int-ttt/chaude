---
name: distill
description: Fresh start - distill only the important content (facts, decisions, deliverables, open items, lessons) from a project's old Codex transcripts into notes/, so the old sessions can be archived and work restarts from a clean context.
argument-hint: "[project folder, or its transcript folder inside .Codex/projects]"
---

# Distill old sessions into notes (main thread orchestrates; agents do all reading)

Goal: notes good enough that the old transcripts are never needed again. Cost target: Haiku for reading, one Sonnet merge, a few short main-thread turns.

1. **Chunks.** If `notes/distill/chunks/` already exists, use it. Otherwise `haiku-batch-worker`: run `bin/chaude extract-transcript $ARGUMENTS --out notes/distill` and return the INDEX line.
2. **Extract (parallel, one message).** Agents = ceil(chunks / 4), at most 20 at once; split the chunks into that many consecutive groups of near-equal size (5 chunks → 3+2, not 4+1). One `haiku-extractor` per group. Brief: the chunk file names only.
   - An extractor that stops before its last chunk → start a fresh `haiku-extractor` for the unread chunks.
3. **Merge (one `sonnet-analyst`).** Brief:
   - Read every `notes/distill/cand/*.md` and, if present, the old project AGENTS.md content below the policy section and the backup in `.Codex-backup-*/AGENTS.md`.
   - Apply CORRECTIONS and recency (newer wins). Deduplicate. Drop items that are done and no longer matter.
   - Write `notes/FACTS.md`, `notes/DECISIONS.md` (merge into existing lines; keep the template format), `notes/DELIVERABLES.md`, `notes/OPEN.md` (mark past deadlines `past`), `notes/LESSONS.md`.
   - Delete empty template lines (`- [area]` with nothing after it); never count them as items.
   - Every conflict or (?) item → `notes/distill/REVIEW.md` as numbered questions answerable in a few words.
   - Its final message is all the main thread sees: ≤3 lines, no item lists, no file contents.
4. **Verify (`haiku-checker`).** Count the items (`- ` lines with text) in each of the 5 notes files and REVIEW.md, and check every path in DELIVERABLES.md exists. These counts are the official ones.
5. **Reply** (≤5 lines): haiku-checker's counts, missing deliverables, REVIEW question count. Never relay item lists; point to the files.
   `추천:` 줄 + 목록 `- `: `notes/*.md`의 REVIEW.md 질문에 번호별로 답하면 haiku-batch-worker가 반영합니다.
After step 4, `haiku-batch-worker` deletes `notes/distill/chunks/` (the originals stay, gzipped, in the Codex-archive folder).
Explanations of the items, if asked: one `sonnet-analyst` writes `notes/runs/distill-explain.md`; reply with the path only.
6. After the user answers: `haiku-batch-worker` applies answers to notes/; if the user agrees, moves (old) deliverables to `archive/` and removes the old content below the policy section in AGENTS.md (it now lives in notes/).
