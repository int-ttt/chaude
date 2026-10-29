# merged plan: specter + local commands (reviews merged)
date: 2026-10-29
sources: 261029-sum-specter.md, 261029-sum-localcmd.md, 261029-sum-state.md (scout summaries of plan/review files)
scope this run: smallest complete subset; rest deferred (listed at end)

## decisions (default + reason)
S0: do not commit; leave dirty tree (user said no commit). new edits stay in separate files where possible.
D1: specter = Go in-memory display, no model, no agent file. reason: 0 tokens, review agrees.
D2: main row shown for every main model; subagent-row scope = all agents. reason: no cost, simpler.
D3: no haiku-specter agent file; code name progressRow/specter in tests only. reason: avoids false haiku-* route target.
D4: content = elapsed + last tool line only. reason: no token accounting.
D5: haiku agents: "did" REPLACES the outcome field of the existing return summary (no extra line), final message <=3 lines and <=300 chars. reason: R3 token duplication.
D6: order: main row (no dependency) and background fix independent; both done now. reason: disjoint files.
D7: harness shows agent Summary directly; main does not restate (policy note only, no code change this run).
D8: no OutPath tailing; async subagent row = state + elapsed only. reason: no-poll rule, parent_tool_use_id unverified.
local-1 /rotate: keep model handoff. local-2 /model <name>: keep handoff (switchModel refactor deferred).
local-3 /version: keep as is (deferred async fix). local-4 /cost, local-5 /context, local-6 /agents: keep passthrough (0 model tokens already, per R1).
local-7 plain REPL: wire /help /status /exit locally (no model send). /model, /clear, unknown "/x" unchanged (lazy /clear and custom-command impact deferred; needs user).
Acceptance strictness: "no model Send" for these commands.

## workstreams (one haiku-coder each, disjoint files)
W1 tui.go: add tuiModel fields turnStart time.Time, lastTool string; set turnStart=now, lastTool="" wherever m.busy=true is set; set lastTool in toolMsg case (trunc 40); extend mainStatusText() to append ` · <elapsed> · <lastTool>` (elapsed `<m>m<ss>s`, lastTool part omitted when empty). New test specter_test.go TestSpecterRowRenders: busy model with lastTool "Read(x)" -> mainStatusText contains elapsed and "Read(x)"; idle (not busy) path unchanged.
W2 drain.go: tool_result containing "Async agent launched" for an Agent tool_use must mark agent Background/running, not Done; <task-notification> still marks Done. Spec in notes/runs/261029-diag-background-panel.md. Test drain_test.go TestBackgroundAgentShownRunning.
W3 repl.go + new localcmd.go: func localCmd(a model,... ) kept minimal: in plain loop handle /exit (like /quit), /help (help text), /status (model, ctx, threshold via effectiveThreshold) locally before handle(). Test localcmd_test.go TestPlainLocalCmdsNoSend: Send count unchanged after those lines (use existing test seams from resumeshift_test.go/acceptance_test.go).
W4 haiku-batch-worker: .claude/agents/haiku-*.md: replace return-summary sentence with: final message <=3 lines and <=300 chars, first item = what was done (replaces outcome), details via notes/runs path.

## tests (3)
TestSpecterRowRenders (W1), TestBackgroundAgentShownRunning (W2), TestPlainLocalCmdsNoSend (W3)
verify: go build ./... ; go test ./... ; sonnet-reviewer on diff.

## deferred (need user or larger work)
lazy /clear (R3/R4 spec), switchModel + validation (R5), /version async, subagent last-tool (S3a live check), conductor vs /model (U5), plain unknown-command (U6), /clear with running agents (U7), /rotate deterministic.
