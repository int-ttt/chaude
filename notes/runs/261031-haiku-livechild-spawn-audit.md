# livechild / repl claude-child audit (read-only)
date: 2026-10-31
agent: haiku
status: done; no source edited

## 0. where spawn lives
- note: brief named livechild.go and repl.go; neither spawns. exec.Command for resident child is core.go:154 (NewSession).
- livechild.go: 0 exec/pty/Kill/Wait hits. only usage hit = livechild.go:22 (touchMain, ttlSeen via observedTTL).
- repl.go: 0 exec/pty/Kill/Wait/--resume/--session-id hits. spawn via newSessionFn (repl.go:236, =NewSession) called at repl.go:257.

## 1. exact claude args (core.go:143-153)
- ClaudeBin default "claude" (core.go:142-144); osenv.ResolveBin applied (core.go:154)
- core.go:145-146: args := []string{"-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose", "--include-partial-messages", "--model", o.Model}
- core.go:147-148: if !o.NoPermission -> append "--permission-prompt-tool", "stdio"
- core.go:150-151: if o.Resume != "" -> append "--resume", o.Resume
- core.go:153: append o.Extra... (repl.go:252 "--append-system-prompt" rule; repl.go:255 "--max-budget-usd" N)
- session id: no "--session-id" anywhere in internal/harness (non-test). id learned from stream events (core.go:251, 299 setID).
- env: core.go:155 cleanEnv (drops ANTHROPIC_API_KEY, core.go:23-39) + withCacheTTL (core.go:42-54, CLAUDE_CODE_PROMPT_CACHE_TTL=1h|5m)
- cmd.Dir = o.Cwd (core.go:156); Stderr nil (core.go:157)
- repl.go:239 SessionOpts{Model, Cwd, CompactWindow, Resume: r.resumeID, CacheTTL}; repl.go:240 resumeID cleared after use.

## 2. mode: resident stream-json, multi-turn
- stdin pipe kept open (core.go:158 StdinPipe); one long-lived process per Session (core.go:166 Start).
- turns: Send writes {"type":"user",...} JSON line (core.go:192-202, write core.go:187-190). Events reads until type=="result" (core.go:229-271, return at 266-268).
- stdout reader goroutine core.go:170-183 feeds s.lines (closed on EOF, core.go:171).
- control: RespondPermission core.go:205-220 (control_response), Interrupt core.go:222-225 (control_request interrupt).
- idle reader StartIdle core.go:276-307 consumes stdout between turns.
- one-shot (NOT resident) claude calls:
  - classifier.go:188-189: exec.Command("claude","-p",in,"--model",rm,"--output-format","json","--system-prompt",routerPrompt(rm),"--tools","","--no-session-persistence")
  - usagepoll.go:54: exec.CommandContext(ctx,"claude","-p","/usage","--output-format","json","--no-session-persistence")
  - tui.go:703: same /usage one-shot (exec.Command)
  - tui.go:953: exec.Command("claude","--version").Output()

## 3. PTY / interactive TUI child
- none. grep pty / creack/pty / kr/pty in internal/harness/*.go and whole repo *.go: no hits.
- go.mod (module chaude, go 1.22) requires: charmbracelet/bubbles v0.20.0, bubbletea v1.3.4, lipgloss v1.1.0, x/ansi v0.8.0, go-runewidth, termenv v0.16.0. no creack/pty, no kr/pty.
- indirect only: muesli/cancelreader v0.2.2 (stdin cancel reader, not a pty), x/term v0.2.1, mattn/go-isatty v0.0.20, x/sys v0.30.0. none imports a pty package directly.
- repo grep for creack|kr/pty hits only notes/runs/*.md (text), no .go file.

## 4. crash/exit, restart/resume
- core.go:135 dead atomic.Bool; set core.go:172 on stdout EOF (reader goroutine).
- core.go:246-248 Events returns false when lines closed (process gone). no error surfaced.
- core.go:352-353 Dead() — only callers: idlesweep.go:50 (childDead, parked child) and switchchild.go:35-38 (childDead) used at switchchild.go:160.
- core.go:355-369 Close(): StopIdle, close stdin, cmd.Wait with 15s timeout (core.go:362-367), then Process.Kill (core.go:366). Kill appears only here.
- repl.go:505 `_ = r.sess.Send(text)` — send error ignored. no restart of a dead ACTIVE child found: no Dead() check in runTurn (repl.go:495-626) or loop (repl.go:989).
- idlesweep.go:34-65 sweepIdle: dead parked child -> Close + MarkEnded "child-exit" (idlesweep.go:50-54); cold child -> Close + "expired" (idlesweep.go:55-60).
- idlesweep.go:69-77 idleRotateNow -> rotate(...) (cache cold rotation).
- repl.go:755-816 rotate(reason): closes old via r.sess.Close (repl.go:761 snapshot_kept path; 794 normal), handoff snapshot first via makeSnapshotN (repl.go:770), then r.start() (repl.go:805). start error -> stderr "chaude: cannot start claude:" (repl.go:766, 806).
- repl.go:238-260 start(): builds opts, newSessionFn (repl.go:257), sets r.sess (repl.go:261).
- repl.go:727-750 makeSnapshotN: handoff turn = r.runTurn(handoffPrompt,false) at repl.go:733; handoffPrompt at repl.go:30.
- repl.go:781-795 drains: old child kept as draining (repl.go:781) up to MaxDrain, others Close (repl.go:783).
- switchchild.go:144-237 switchChild(target,resumeID): pickChild, Take from childSet, Close stale (switchchild.go:165), resume via reg.Get (switchchild.go:172-173) or parked[target] (switchchild.go:179-181); sets r.resumeID (switchchild.go:212).
- switchchild.go:77 parkOld, :97 retire, :115 endParked, :131 endAllParked.
- livechild.go:116-143 unbindActive (park active into liveChild), livechild.go:147-183 bindChild (restore; adds "## While this session was parked" pendingSnapshot).
- children.go:81 Park, :92 Take, :119 CloseAll (CloseAll calls Close).
- repl.go:1198-1203 reconcileStart: orphan notice for claude pid still running after harness died (no kill).
- resume.go:58-60 resumeMsg validation: only "claude" source.
- classifier.go:295 r.resumeID = resume (classifier path).
- no "Restart" function exists; "restart" appears only in comments (repl.go:760, repl.go:1-like wording, orchws.go:67).

## 5. usage / cache token parse
- core.go:73-76 ctxTokens = input + cache_read + cache_creation.
- core.go:78-88 observedTTL reads usage.cache_creation.ephemeral_1h_input_tokens / ephemeral_5m_input_tokens.
- livechild.go:9-28 touchMain: stream_event message_start -> lastAPI (:16-19); assistant usage -> ttlSeen (:22-25).
- livechild.go:84-85 parked result total_cost_usd -> budget.charge.
- repl.go:320-321 idleEvent: main assistant usage -> r.idle.usage.
- repl.go:414-420 idleFinish: result usage, ctxTokens, cost.
- repl.go:545-546 runTurn: lastUsage from main assistant; :547-550 mid-turn budget interrupt.
- repl.go:617-620 result total_cost_usd -> budget.charge.
- repl.go:655-679 usageLine prints cache_read_input_tokens (:677) and cache_creation_input_tokens (:678).
- metrics.go:54-58 usageFields extracts cache_creation_input_tokens etc.
- budget.go:122 cost formula uses cache_read and cache_creation.
- turnspend.go:14 spendKeys include cache_* keys.

## doubts / not found
- no --session-id: not found anywhere in non-test harness code.
- no PTY, no creack/pty or kr/pty in go.mod/go.sum-level usage (go.sum not checked line by line).
- no automatic restart of a crashed ACTIVE child: only parked dead children are reaped (idlesweep.go:50). inferred from absence of Dead() check on active path; a crash mid-session would leave r.sess dead until the next rotate/switch.
- livechild.go holds no spawn code; spawn is core.go.
