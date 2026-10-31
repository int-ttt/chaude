# review: 261030-plan-interactive-child
date: 2026-10-30
reviewer: opus-architect
plan_path: notes/runs/261030-plan-interactive-child.md
plan_status: MISSING (not found in repo; no file with "interactive" in name). Review is written against the brief goal only.
goal: can harness children (core.go:145 `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages`) run as interactive Claude Code TUI clients instead.

## dependencies on -p stream-json (would break)
- core.go:145-183: stdin JSON lines in, stdout JSON lines out (Session.lines).
- core.go:148,223 + drain.go:262 + livechild.go:37 + repl.go:305,447,590: permission via `--permission-prompt-tool stdio` / control_request can_use_tool.
- drain.go:143-168, children.go:55, repl.go:581: <task-notification> detection from user events.
- budget.go:6,115 + core.go:74-80: total_cost_usd, usage, ctxTokens, observedTTL from result/usage events.
- --include-partial-messages: TUI streaming text.
- DECISIONS.md:15 slash passthrough, :37/:45 usage one-shot `claude -p /usage`.

## feasibility
verdict: technically possible (PTY spawn), not a drop-in. Interactive TUI emits ANSI screen output, no structured events; every item above must be rebuilt from other channels.

## options
A pty_scrape: spawn `claude --model X` under a PTY (creack/pty) + VT emulator, type input, scrape screen.
  pros: true interactive mode; native slash cmds and dialogs.
  cons: no usage/cost/session_id/result; permission dialogs must be screen-matched; breaks on CLI UI changes and terminal size; rewrite of drain/livechild/repl/budget. Not acceptable.
B pty_plus_transcript_hooks: PTY for input only; structured output from session transcript JSONL (~/.claude/projects/<cwd-slug>/<session_id>.jsonl, path unverified) tailed; turn end via Stop hook, permission via PreToolUse hook decision, both injected with --settings.
  pros: interactive mode with mostly structured data; reuses existing event parsers if JSONL shape is close to stream-json messages.
  cons: no partial streaming (block-level only); no total_cost_usd/result event (budget.go must compute from usage); hooks currently main-only (DECISIONS.md:26) and --settings merge unverified; PTY multiline/paste/interrupt handling; second code path to maintain.
C keep_stream_json (status quo) + targeted gaps: keep core.go:145; cover interactive-only needs individually (slash passthrough exists, /usage one-shot exists); for human inspection close/park the child then `claude --resume <id>` in a real terminal.
  pros: zero rewrite; all structured signals kept.
  cons: if the motivation is billing (-p vs subscription) or an interactive-only feature, C does not solve it.

## recommendation
choice: C now. B only as a time-boxed spike, and only after the user states a concrete reason (billing, missing feature). A rejected.
why: every harness control signal (permission, notification, cost, rotation thresholds) comes from stream-json; B recovers most but not partial streaming or cost, A recovers none.

## missing in plan (add if the plan is written)
- step 0: state the motivation (billing / feature / UX). Without it the choice cannot be made.
- step 0b: verify billing first: does `-p` on subscription login use subscription included usage (notes/research/billing.md:10,18 unverified). If yes, the main motive for interactive disappears.
- spike before rewrite: one Go test under internal/harness/testdata or a scratch cmd: PTY spawn `claude --model haiku` in tmpdir, send "say hi" + Enter, detect Stop hook, read transcript JSONL, compare fields with stream-json result. Gate: all of session_id, assistant text, usage present.
- feature flag: SessionOpts.Mode ("stream"|"pty"), default "stream"; never replace core.go:145 path.
- must NOT change: classifier.go one-shot, usagepoll one-shot, rotation thresholds (DECISIONS.md:46), hooks main-only rule unless DECISIONS updated.

## unverified
1. -p on subscription billing vs interactive (billing.md:18).
2. transcript JSONL path, flush timing, whether it carries usage/cache fields per message.
3. Stop / PreToolUse hooks fire for PTY interactive sessions launched with --settings; merge with user settings.
4. PreToolUse hook can fully replace can_use_tool (allow/deny/ask with UI) without the TUI dialog appearing.
5. <task-notification> delivery while interactive session is idle (no stdin turn).
6. PTY input: multiline via bracketed paste, slash cmds, ESC interrupt, trust/onboarding dialogs on first run in a new cwd.
7. --output-format / --include-partial-messages ignored without -p (assumed).
8. 5h rate / /usage reporting identical between modes.
9. concurrency: interactive `--resume <id>` while a -p child of the same session is alive (assume unsafe).

## user question
- 인터랙티브 전환을 원하시는 이유가 과금(구독 사용량) 때문인지, 특정 기능 때문인지 알려주십시오.
