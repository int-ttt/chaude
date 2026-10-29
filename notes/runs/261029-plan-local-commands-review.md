# Review: 261029-plan-local-commands.md
date: 2026-10-29
verdict: plan direction OK; test harness and /clear lazy change are under-specified (nil/closed r.sess sites, startup spawn breaks "log empty"); several conditional items need no move at all.
basis: repl.go, tui.go, classifier.go, core.go, .claude/settings.json read at this date. Line refs as of today.

## Improvements (each with a plan edit)

1. Passthrough set is already 0 model tokens; do not port /cost /context /agents /usage /mcp /config to Go.
   - fact: repl.go:790 comment "local commands (/usage, /cost, ...) have no model usage"; passthrough skips usageLine when usage is empty. Only /init /review /doctor run a turn.
   - porting them trades claude's exact output for a partial Go copy (lastCtx only, tracker only, metrics sum) and saves 0 tokens.
   - plan edit (a)+(e): mark /cost /context /agents "keep passthrough (already 0 tokens)"; decisions 4,5,6 become "recommended: keep" unless the user's goal is "no child process" rather than "0 tokens". Add one verification step: run each once, transcript `assistant` usage empty and total_cost_usd unchanged.
   - risk if ported /cost: metricscmd.go sums cost_usd per turn; whether result.total_cost_usd is per-turn or cumulative per child is not verified -> double counting possible. Add "verify before summing" if decision 4 picks local.

2. Test plan "log empty" cannot pass: startup always spawns.
   - fact: loop() repl.go:796 and runTUI tui.go:1093 call r.start() before any command, so the fake-claude log is never empty in TestPlainLocalCmdsNoSpawn.
   - fake "exits 1": NewSession succeeds (cmd.Start), then lines closes; negative-control /usage path depends on Events behaviour on a dead child. Fragile.
   - plan edit (d): unit tests use the existing seam newSessionFn (resumeshift_test.go:68) with a counter; build Repl + tuiModel directly, call runSlash / localCmd, assert counter delta == 0 after the command (baseline taken after startup). Keep the PATH fake for one acceptance test only, asserting delta, and make the fake block on stdin (cat >/dev/null) instead of exit 1.
   - add exec seam `var execOutput = func(name string, args ...string) ([]byte, error)` for /version.
   - add `go test -race` for these tests: /status reads lastCtx/rotations from the UI goroutine while idle-turn goroutine (idleFinish) may write.

3. /clear lazy start: needStart flag alone panics or misroutes. Specify ensureSession and every r.sess site.
   - fact: r.sess dereferenced at repl.go:393 527 566 571 609 623 714 725 744 772, tui.go:874, classifier.go:180 (route closes r.sess when no turns), classifier.go:194/202 (nil-checked).
   - plan edit (c.1), exact spec:
     - clear(): r.sess.Close(); r.sess = nil; reset tracker the same way start() does (new tracker, keep sortedRestored) via a shared helper `resetTracker(parent func() string)`; no start().
     - new `func (r *Repl) ensureSession() error { if r.sess != nil { return nil }; return r.start() }`.
     - handle(): call route() first (route must be nil-safe: classifier.go:179-183, if r.sess == nil only set r.a.Model, no Close/start), then ensureSession() before line 714. On error print "cannot start claude" and return (do not record the user turn).
     - passthrough(): ensureSession() at top.
     - rotate(): if r.sess == nil -> no handoff, no Close; print "── nothing to rotate (new session starts with next message) ──"; return. Model change is picked up by the later start().
     - importSession(): check resume.go:82 path; it calls clear then applyImport, so it must not depend on a live child (verify).
     - /status tui.go:874 and usageLine repl.go:527: show "-" when r.sess == nil.
     - runTUI/loop defers already nil-check (tui.go:1098, repl.go:804).
   - effect: /clear, /clear+/model x, /clear+/clear, /clear+exit spawn 0 children.
   - side benefit to state in decision 8: .claude/settings.json has SessionStart (matcher includes clear -> session-context.py) and SessionEnd (history-export.py, 60s timeout). An eager child closed unused may run these hooks and write an empty history export (verify whether -p fires SessionStart at process start or first message).
   - cost to state in decision 8: claude startup latency moves to the first prompt after /clear (not measured).

4. /clear kills running agents without marking them.
   - fact: clear() calls r.sess.Close() with no drain and no markInterrupted; agentStore rows stay "running" until exit.
   - plan edit (c.1): before Close, if agentsBusy() call the same marking used at exit (markInterrupted for this session's agents) or warn "N agents running; /clear stops them". Add to TestLocalClearNoSpawn: running tracker row ends interrupted.

5. /model <name>: fix three semantic bugs regardless of decision 2.
   - a. r.a.Model is set before rotate (tui.go:1129). If rotate holds (repl.go:596-600) the label shows the new model while the child still runs the old one.
   - b. No turns yet: a handoff turn is pure cost. classifier.go:179 already does "no turns and no pendingSnapshot -> Close+start, no handoff". plan edit: extract `switchModel(name string)` used by tui submit and plain loop: validate name, if len(r.tr.turns)==0 && pendingSnapshot=="" -> restart (or set r.sess=nil if lazy) with 0 tokens; else rotate("model "+name); on rotate hold or start error, revert r.a.Model and say so.
   - c. No validation: "/model foo" -> start fails, r.sess stays a closed session, next Send writes to closed stdin, Events waits up to 300s. Accept haiku|sonnet|opus or a "claude-" id; else note "unknown model".
   - d. Handoff runs on the old model (e.g. an opus handoff before switching to haiku). Put this cost in decision 2's wording.

6. /model under conductor is overridden on the next prompt.
   - fact: Conductor default true (repl.go:875); route() classifier.go:165-167 switches to haiku whenever the classifier says haiku and r.a.Model != haiku; classifier.go:171-186 switches to any other routed model.
   - so "/model sonnet" lasts until the next classified prompt. plan edit (e): add user decision (see U5).

7. /version: current code has its own defects; test must cover it either way.
   - fact tui.go:925: exec.Command("claude") literal (not osenv.ResolveBin / SessionOpts.ClaudeBin), runs synchronously inside Update (UI blocks for node startup), error ignored (prints "claude " empty), no harness version shown.
   - plan edit (c): run it as a tea.Cmd (async), via ResolveBin, cache the result after first call, show harness version too, show "claude version unavailable" on error. Option worth listing in decision 3: read `claude_code_version` from the child's system/init event if the installed CLI emits it (verify; init may only arrive after the first message in stream-json input mode).
   - plan edit (d): yes, cover /version. Assert via exec seam: exactly one call with args ["--version"], never "-p", and session counter delta 0. This passes under both strictness options, so decision 9 only decides whether that one --version call is allowed.

8. localCmd shared function: narrow its scope.
   - /mouse and /exit need tuiModel side effects (tea.EnableMouseCellMotion, quit()); they cannot live in `localCmd(line) (out string, ok bool)`.
   - plan edit (c.4): localCmd covers read-only text commands only: /help, /status, /model (no arg), /mode (no arg), /version (cached text). Side-effect commands stay per path.
   - plain loop additions (repl.go:812-846), placed after the echo at repl.go:831-833: /exit (same as /quit, currently sent to the model), /clear -> r.clear(), /model <name> -> switchModel, unsupportedCmds -> note (currently sent to the model), /mouse -> "TUI only" note, unknown "/x" not in passthroughCmds -> "unknown command" instead of handle(). The last one changes behaviour for skills/custom slash commands typed in plain mode; list it as U6.

9. /rotate (decision 1): give a hybrid option.
   - deterministic fallback already exists (repl.go:603-606, buildSnapshot with recent turns).
   - plan edit (e.1): options = keep handoff (recommended: rotate's purpose is a quality summary) / always deterministic / deterministic when lastCtx is small or turns <= TailTurns. With lazy /clear, rotate on r.sess==nil is a no-op (item 3).

10. Implementation order (replace plan (c) order).
   - 1. Test seams: newSessionFn counter, execOutput seam, baseline-delta helper, negative control (/usage via passthrough increments Send count, not spawn count, since the child exists). No behaviour change.
   - 2. localCmd extraction for already-local read-only commands; TUI output byte-identical (golden test).
   - 3. Plain loop wiring (item 8): biggest real gain today, since plain mode sends /help /status /clear /model /exit to the model.
   - 4. switchModel (item 5 a-c), both paths.
   - 5. /version async + seam (item 7).
   - 6. /clear lazy + ensureSession + agent marking (items 3, 4), only after U1; riskiest, touches route/rotate/handle.
   - 7. Conditional items only after decisions.
   - agent: steps 1-5 are bounded -> haiku-coder with exact spec; step 6 touches routing/rotation invariants -> sonnet-analyst implements, sonnet-reviewer checks diff.

## Safe-to-run-locally classification (for plan section a)
- read-only, safe: /help, /status, /model (no arg), /mode (no arg), /resume (list/picker), /version (spawns `claude --version` only).
- local state change, intended: /mouse, /mode <name>, /mode conductor|conductor-off, /exit, /quit.
- session state change (drops child/context): /clear, /import, /model <name>, /rotate. Not "surprise-free": they close a child (SessionEnd hook runs, running agents die). Plan should list them separately from the read-only set.

## User decisions (separate)
- U1 (plan 8): /clear lazy start? gain: 0 empty children, no unused-session hooks; cost: startup latency on first prompt after /clear. 사용자 확인 필요.
- U2 (plan 4,5,6): keep /cost /context /agents as passthrough (already 0 tokens, recommended) or port to Go for "no child process" reasons? 사용자 확인 필요.
- U3 (plan 1,2): /rotate and /model <name> handoff policy; include "handoff runs on the old model" and the hybrid option. 사용자 확인 필요.
- U4 (plan 3,9): /version source: keep `claude --version` (async, cached) or init-event field (unverified). Test covers it either way. 사용자 확인 필요.
- U5 (new): /model <name> while conductor is on: (a) also turn conductor off, (b) pin the model until /model again (route skips model switch), or (c) keep current (next prompt may switch back). 사용자 확인 필요.
- U6 (new): plain mode unknown "/x": reject locally, or keep sending to the model (custom commands/skills)? 사용자 확인 필요.
- U7 (new): /clear with running agents: stop and mark interrupted, or refuse until they finish? 사용자 확인 필요.
