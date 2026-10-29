# Plan: run slash commands locally in Go (0 model tokens)
date: 2026-10-29
status: plan only, no code changed
basis: notes/runs/261029-scout-command-exec.md (22 slashCmds: local 10, model-sent 11, mixed 1)
line refs: internal/harness/tui.go, repl.go as of this scout; re-grep before editing

## (a) Candidates: can run from local state in Go, no `claude -p`

| cmd | verdict | state source (Go) | what it reads / output today | current handler |
|---|---|---|---|---|
| /help | LOCAL, ready | slashCmds (tui.go:150) | static list + keys | tui.go:859-863 |
| /status | LOCAL, ready | Repl: a.Model, lastCtx, effectiveThreshold(), rotations, sess.SessionID | already local | tui.go:873-877 |
| /model (no arg) | LOCAL, ready | r.a.Model | already local | tui.go:879-884 |
| /mouse | LOCAL, ready | tuiModel.mouse + tea mouse cmd | already local | tui.go:865-871 |
| /exit, /quit | LOCAL, ready | quit() | already local | tui.go:857-858 |
| /clear | LOCAL, needs change | new session, no prompt | r.clear() calls r.start() = spawns child (no prompt, no tokens); spec requires no spawn | repl.go:919-930, tui.go:927-931 |
| /agents | CONDITIONAL (decision 6) | agentTracker: running() drain.go:131, counts() drain.go:176, sortedAll() agentview.go:82 | local list of running agents of this run; claude /agents shows agent definitions (not local to Go state) | tui.go:927 -> passthrough repl.go:770 |
| /version | CONDITIONAL (decision 3) | none (needs claude binary version) | runs `claude --version` (a process, not -p, 0 tokens) | tui.go:924-926 |
| /cost | CONDITIONAL (decision 4) | metrics jsonl (metricscmd.go sums cost_usd per session) and per-turn total_cost_usd (repl.go:513 usageLine) | no cumulative cost kept in Repl; claude /cost content not verified equal | tui.go:927 -> repl.go:776 |
| /context | CONDITIONAL (decision 5) | r.lastCtx only (repl.go:518) | total ctx only; claude /context category breakdown needs child state | tui.go:927 -> repl.go:776 |
| /rotate | CONDITIONAL (decision 1) | buildSnapshot (repl.go:536) is deterministic | handoff needs a model turn (handoffPrompt, repl.go:585); without it snapshot note = "(empty)" + turns | tui.go:1114 -> repl.go:565 |

Notes:
- /model <name> (with arg) is model-sent today: submit -> rotate("model x") (tui.go:1128). Same decision as /rotate (decision 2).
- Plain REPL (repl.go:912-915, loop repl.go:795-853): /help /clear /model /status /version /mouse /exit go to handle() and are SENT TO THE MODEL. Local set must also be wired into the plain loop, or the plain loop stays model-sent (decision 7).

Chosen local list (no decision needed): /help /status /model(no arg) /mouse /exit /quit /clear(lazy start).
Conditional (see section e): /agents /version /cost /context /rotate /model <name>.

## (b) Keep model-only, and why

- /init: runs a model turn by design (analyze repo, write CLAUDE.md proposal). Needs the model.
- /review: runs a model turn by design (reads changes, reviews). Needs the model.
- /compact: rewrites the child's context via a model summary. Child state is inside claude; Go cannot compact it.
- /doctor: checks claude installation/config. Checks and wording belong to claude; a Go copy would drift. Help text already says "runs a turn" (tui.go:166-167).
- /mcp: MCP server status is held by the claude child. Go has no view of it.
- /config: claude settings are read/written by claude; a Go port would have to track claude's config schema. Keep until a decision says otherwise.
- /usage: plan limits come from claude (5h/7d windows). Only r.rate partial data is local (repl.go:513-520). Busy path already uses a one-shot `claude -p /usage` (tui.go:692); keep.

## (c) Implementation spec (chosen list; conditional items only after decisions)

Dispatch: runSlash (tui.go:853) already handles local cases. Changes:
1. /clear: tui.go:927-931 sends "/clear" to submit (tui.go:1116 -> r.clear). Change r.clear (repl.go:919-930): drop r.start() call; set r.needStart = true. runTurn (repl.go:388) and passthrough (repl.go:770) start the child lazily when needStart is set. Local /clear then needs no spawn. Output "── new session ──" unchanged.
2. /help: already local. Replace "(from claude)" wording only if /cost /context /agents move (decision 6, 4, 5).
3. /status, /model (no arg), /mouse, /exit, /quit: no code change in runSlash. Verify no submit path (tui.go:1113-1135).
4. Plain REPL: add a local branch in loop (repl.go:812-846) for /help /status /model(no arg) /clear /exit /quit so they do not reach handle() (repl.go:696). Shared function recommended: localCmd(line string) (out string, ok bool) used by both paths, to keep text in one place.
5. Remove the route for these from submit (tui.go:1112-1135) only if they never reach it. /clear stays in submit (lazy version) until changed.

Files touched (expected): internal/harness/tui.go (runSlash, submit), internal/harness/repl.go (clear, runTurn/passthrough lazy start, loop), new internal/harness/localcmd.go (shared text + state readers). No change to model-sent handlers.

## (d) Acceptance: per command, Go test asserts no claude child started

Harness for all tests: put a fake `claude` executable first on PATH (t.Setenv("PATH", dir+":"+old)) and set ClaudeBin to it. Fake appends argv to a log file and exits 1. Test asserts log file absent or empty after the command.

| cmd | test name (proposed) | assert |
|---|---|---|
| /help | TestLocalHelpNoSpawn | output contains "/help"; log empty |
| /status | TestLocalStatusNoSpawn | output contains model, "ctx", threshold; log empty |
| /model | TestLocalModelNoArgNoSpawn | output "model <a.Model>"; log empty |
| /mouse | TestLocalMouseNoSpawn | mouse flag toggled; log empty |
| /exit, /quit | TestLocalExitNoSpawn | quit cmd returned; log empty |
| /clear | TestLocalClearNoSpawn | session reset, needStart=true, log empty until next prompt |
| /agents (if decided) | TestLocalAgentsNoSpawn | lists running tracker rows; log empty |
| /version (if decided) | TestLocalVersionNoSpawn | no exec of claude (version read from source) |
| /cost, /context (if decided) | TestLocalCostNoSpawn, TestLocalContextNoSpawn | from metrics/lastCtx; log empty |
| /rotate, /model x (if decided) | TestLocalRotateNoSpawn | deterministic snapshot path; log empty |
| plain REPL | TestPlainLocalCmdsNoSpawn | all chosen commands via loop with HARNESS_PLAIN=1 / piped stdin; log empty |

Existing related tests to extend: acceptance_test.go (builds binary, runs repl), usagebusy_test.go (busy /usage one-shot). Negative control: the same fake PATH must record a spawn for /usage and /compact (proves the harness detects spawns).

## (e) User decisions needed

1. /rotate: keep model handoff (current, costs tokens), or deterministic snapshot with no handoff turn (0 tokens, lower summary quality)?
2. /model <name>: same choice as 1 for switching models.
3. /version: keep `claude --version` (a claude process, 0 tokens, breaks a strict "no claude process" test), or read the version without spawning (no source found for this; 사용자 확인 필요)?
4. /cost: show local cumulative cost from metrics jsonl (per-session sum), or keep claude's /cost? Equivalence to claude output not verified. 사용자 확인 필요.
5. /context: show only total ctx from r.lastCtx (partial), or keep claude's category breakdown?
6. /agents: show running agents from the tracker (local, this run only), or keep claude's agent definitions list?
7. Plain REPL (non-TTY, HARNESS_PLAIN=1): wire the local set there too (recommended), or TUI only?
8. /clear: accept lazy child start (r.clear change, repl.go:919-930)?
9. Acceptance strictness: "no claude process at all" (covers /version) vs "no claude -p / stream-json child" (allows /version)?

## Haiku-specter request (evaluation)

No agent named `haiku-specter` exists: .claude/agents has haiku-batch-worker, haiku-checker, haiku-coder, haiku-drafter, haiku-extractor, haiku-researcher, haiku-runner, haiku-scout, opus-architect, sonnet-analyst, sonnet-reviewer. Nearest match: haiku-scout (model: haiku, effort high per DECISIONS). Trade-off: an agent for "silent runs" from opus/sonnet would spend haiku tokens on each run and add latency, while the chosen local commands spend 0 tokens by design. An agent only helps where a judgment step is still needed (e.g., /rotate handoff note, if decision 1 keeps the model). Any agent used this way must follow the return-summary rule (DECISIONS 36). Name and scope to be confirmed by user 사용자 확인 필요.
