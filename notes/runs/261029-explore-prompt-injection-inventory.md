# inventory: text injected per model call (excl. router prompt)
date: 2026-10-29
mode: read-only; no repo files edited. Sizes are approx chars (wc -m for files, hand count for hook/Go strings, +-10%).

## A. always-on, every main-thread call
- CLAUDE.md: /Users/leekwangseok/PycharmProjects/claude-harness/CLAUDE.md = 8005 chars (8236 bytes). applies: every main call. subagents: not loaded (all 11 agents have omitClaudeMd: true).
- ~/.claude/CLAUDE.md: absent.
- system prompt append (--append-system-prompt, repl.go:184-191; default on: NoDelegationRule/NoCategoryRotate default false, repl.go:896-897):
  - delegationRule: repl.go:34-36, 292 chars
  - categoryRule: category.go:10-13, 593 chars
  - total: ~885 chars, every call

## B. UserPromptSubmit hooks (settings.json hooks.UserPromptSubmit, run every prompt, inject only on match)
- style-adapter.py (.claude/hooks/style-adapter.py), no match = no injection:
  - [ack] ~150 chars: approval words (ㅇㅇ, 응, 진행, 해봐 ...)
  - [retry] ~95 chars: starts with 다시/재시도/재시작
  - [frag] ~75 chars: prompt <30 chars, no question, no pending inbox
  - [paste] ~120 chars + path: prompt >1500 chars or >20 lines or contains <pasted_content
  - [inbox] variable: all queued lines of notes/INBOX.md (max 1000 read); file deleted after delivery
  - [lean] ~60 chars: once, on 절약모드 restore
  - [update] ~250 chars: once per version change
  - [normal] ~110 chars: every prompt while lean is off (일반모드)
  - block paths (status fast path, inbox store, mode switch): suppress prompt, no model call, no injection
- correction-guard.py: [corr X] ~70-110 chars per match; [corr xN] ~70 chars at streak >=2; not always
- context-alert.py: [ctx Nk] ~150 chars; currently OFF (settings.json env LEAN_RESET_AT=0)

## C. SessionStart (session-context.py; matcher startup|resume|compact|clear; once per start/compact, not per prompt)
- snapshot: notes/OPEN.md <=20 lines, LEDGER tail 5, previous-session 6 msgs x <=200 chars, INBOX tail 50 lines; hard cap 3000 chars (emit slice)
- [normal mode] line (~150 chars) when lean off; [update] line when version changed

## D. other hooks: no prompt text injected
- agent-model-pin.py (PreToolUse Agent|Task): edits model field or deny reason only
- main-thread-guard.py, haiku-context-guard.py (PreToolUse): deny reason only when a call is blocked
- ledger.py (SubagentStop), history-export.py (SessionEnd, PreCompact): none

## E. harness Go wrappers
- classifier.go:28 delegateHint ~290 chars; prepended at classifier.go:100 only for delegate route (conditional)
- classifier.go:32 planHint ~570 chars; prepended at classifier.go:102 only for haiku + large task (conditional; regex count gave 364, manual count ~570)
- repl.go:32 preamble ~150 chars + repl.go:551 handoff note: only on session rotation (conditional)
- repl.go:541 [ctx ...]: status display only, not injected
- [ack] [retry] [frag] [paste] [inbox] [lean] [normal]: not in Go; defined only in style-adapter.py (no grep hits in internal/)
- --append-system-prompt: only repl.go:191 (no other system/append usage found in internal/ cmd/ outside tests)

## F. subagent definitions: .claude/agents/*.md (body loads only when that agent is spawned; all 11 omitClaudeMd: true)
- sonnet-analyst.md 2955; opus-architect.md 2770; haiku-extractor.md 2148; haiku-drafter.md 2053; haiku-scout.md 1694; haiku-runner.md 1538; haiku-coder.md 1524; haiku-batch-worker.md 1407; haiku-checker.md 1305; haiku-researcher.md 1184; sonnet-reviewer.md 1133
- total: 19711 chars
- skills (.claude/skills/*/SKILL.md, 7 files, 11230 bytes): not in scope; bodies load on invoke only

## G. largest three
1. CLAUDE.md 8005 chars (always, main thread)
2. SessionStart snapshot <=3000 chars (once per start/compact)
3. sonnet-analyst.md 2955 chars (subagent only)
(largest always-on after CLAUDE.md: system rules ~885 chars)

## H. not found
- ~/.claude/CLAUDE.md
- .claude/settings*.local.json
- [ctx] as injected text (display only)
- [ack]/[retry]/[frag]/[paste]/[inbox]/[lean]/[normal] in Go sources

## I. doubts
- hook strings sized by hand; +-10%
- planHint regex count (364) looked truncated; manual ~570 used
