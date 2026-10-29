---
name: migrate-session
description: Migrate an already very large Codex session (hundreds of turns, 500K+ context) into clean new session(s) via compact handoff files, preserving the old transcript. Use before a task switch or when quality/continuity degrades.
---

# Migrate an oversized session

The session is ALREADY large. Every extra turn here re-processes the whole context, so finish the migration in as few turns as possible and start no new work.

Division of labor: only Opus has the conversation history, so Opus composes the content of FACTS / DECISIONS / HANDOFF (terse lines, no prose) and hands it to `haiku-batch-worker` to write to disk. Everything that reads or writes the disk goes to Haiku.

## 1. Freeze & inspect (Haiku)
- Stop implementation/writing work.
- In parallel: `haiku-scout` → inventory of working folders (`ls -t`, newest first; latest/final version of each document; stale copies; `git status --short` / `git diff --stat` if git exists). `/status` → running jobs (PID, log, progress).
- Never revert, reset, stash, or overwrite user changes.

## 2. Split by workstream
If the session mixed several workstreams (e.g. separate documents, features, analyses), write one handoff section per workstream so each can get its own new session. Mark workstreams that are finished as `CLOSED` (one line, no detail).

## 3. Write the handoff files
`notes/FACTS.md` — confirmed facts the user stated or corrected (activities, awards, roles, dates, hardware, numbers). One line each. This is the most valuable output: it prevents repeated corrections.

Scan the session for corrections (아니/근데/아 …): every corrected fact → FACTS.md; every corrected interpretation or preference → DECISIONS.md as a rule ("세션 = 면접 1회 단위, 질문은 섞어서 배치" 같은 형태).

`notes/DECISIONS.md` — chosen versions (e.g. "A안 채택"), tone/style rules, banned phrases, format rules (font size, char limits, file formats). One line each.

`SESSION_HANDOFF.md` (template in `templates/`) — per open workstream:
- Goal + acceptance criteria (limits, deadlines)
- Source-of-truth file paths (final version only; mark stale copies)
- Done / in progress / not started
- Failed approaches with reason
- Exactly one next action
- Pending external info (waiting on the user or other people)

Exclude: transcript summaries, full document text, resolved back-and-forth, generic background, secrets. Link to files instead of copying content.

## 4. Validate (Haiku)
- `haiku-checker`: every path in the handoff exists; FACTS.md matches the final documents (spot-check); report mismatches. Opus fixes only the reported lines.
- Note uncommitted work and running jobs that may not survive.
- Each next action must be executable without the old chat.
- Report: files written + sizes, workstreams list. Then stop.

## 5. Reset
Tell the user:
- (optional) `/rename <old-name>` so the old session is easy to find later.
- Exit and run `Codex` again to start a fresh context (the user does not use `/clear`). The old transcript stays on disk and can be reopened with `/resume`; do NOT resume it for new work.
- Prefer a new session over `/compact` for migration: compact keeps the same conversation and its summary keeps growing.

## 6. First prompt in the new session
"Read AGENTS.md, notes/FACTS.md, notes/DECISIONS.md, and only the `<workstream>` section of SESSION_HANDOFF.md. Delegate all file access per AGENTS.md. Do not summarize the old conversation or redo completed work. Execute only the next action."
