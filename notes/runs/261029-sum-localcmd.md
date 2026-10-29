# Summary: local slash-command plan + reviews
date: 2026-10-29
sources: P = 261029-plan-local-commands.md ; R = 261029-plan-local-commands-review.md ; RR = 261029-plan-routing-review.md (all in notes/runs/)
status: plan only, no code changed (P:3)

## 1. Plan steps (P section c)
local set chosen: /help /status /model(no arg) /mouse /exit /quit /clear(lazy start)
conditional (after decisions): /agents /version /cost /context /rotate /model <name>
model-only, keep: /init /review /compact /doctor /mcp /config /usage (P b)
files: internal/harness/tui.go, internal/harness/repl.go, new internal/harness/localcmd.go
1. /clear: repl.go:919-930 r.clear(): drop r.start(); set needStart=true. runTurn (repl.go:388) and passthrough (repl.go:770) start child lazily. tui.go:927-931 sends "/clear" via submit (tui.go:1116 -> r.clear). Output "── new session ──" unchanged. (Superseded by R3 spec.)
2. /help (tui.go:859-863): no change unless /cost /context /agents move; help text tui.go:166-167.
3. /status (tui.go:873-877), /model no arg (tui.go:879-884), /mouse (tui.go:865-871), /exit /quit (tui.go:857-858): no runSlash change; verify submit path tui.go:1113-1135.
4. plain REPL: add local branch in loop repl.go:812-846 for /help /status /model /clear /exit /quit so they do not reach handle() (repl.go:696). Shared func localCmd(line string) (out string, ok bool). (Scope narrowed by R8.)
5. remove those routes from submit (tui.go:1112-1135) only if they never reach it; /clear stays until lazy change.
runSlash entry: tui.go:853.
implementation order (R10): 
  1 test seams (newSessionFn counter, execOutput seam, baseline-delta helper, negative control)
  2 localCmd extraction, TUI output byte-identical (golden test)
  3 plain loop wiring (biggest gain: plain mode currently sends /help /status /clear /model /exit to model)
  4 switchModel (R5 a-c), both paths
  5 /version async + seam (R7)
  6 /clear lazy + ensureSession + agent marking (R3, R4), only after D8; riskiest
  7 conditional items only after decisions
  agents: steps 1-5 haiku-coder with exact spec; step 6 sonnet-analyst implements, sonnet-reviewer checks diff.

## 2. Open decisions (P e), with options and recommended default
D1 /rotate: (a) keep model handoff (costs tokens, better summary); (b) deterministic snapshot via buildSnapshot (repl.go:536), 0 tokens, lower quality, "(empty)" when no turns; (c) hybrid: deterministic when lastCtx small or turns <= TailTurns (R9; fallback exists repl.go:603-606).
   default: P gives none. R9/RR: (a) keep handoff recommended.
D2 /model <name>: same choice as D1 for switching. R5b: no turns and no pendingSnapshot -> switch with 0 tokens (classifier.go:179 already does Close+start); R5d: handoff runs on the OLD model.
   default: P gives none. R5 proposes switchModel(): 0 tokens when no turns, else rotate("model "+name).
D3 /version: (a) keep `claude --version` (a claude process, 0 tokens; breaks strict "no claude process" test), async and cached (R7); (b) read version without spawning: P found no source; R U4 suggests init-event field claude_code_version (unverified; init may arrive only after first message). 사용자 확인 필요.
   default: P gives none. R U4: keep (a).
D4 /cost: (a) local cumulative from metrics jsonl (metricscmd.go sums cost_usd per session); (b) keep claude /cost. Equivalence to claude output not verified; double-count risk if total_cost_usd is cumulative (R1). 사용자 확인 필요.
   default: R1/U2: keep passthrough (recommended).
D5 /context: (a) total ctx only from r.lastCtx (repl.go:518), partial; (b) keep claude category breakdown.
   default: R1/U2: keep (recommended).
D6 /agents: (a) running agents from tracker (agentTracker running() drain.go:131, counts() drain.go:176, sortedAll() agentview.go:82), this run only; (b) keep claude agent-definitions list.
   default: R1/U2: keep (recommended).
D7 plain REPL (HARNESS_PLAIN=1 / non-TTY): (a) wire local set there too; (b) TUI only.
   default: P recommends (a).
D8 /clear lazy start (U1, R3): accept? Gain: 0 empty children, no unused SessionStart/SessionEnd hook runs. Cost: claude startup latency moves to first prompt after /clear (unmeasured). 사용자 확인 필요.
   default: none stated.
D9 acceptance strictness: "no claude process at all" (covers /version) vs "no claude -p / stream-json child" (allows /version). R7: test passes under both if /version makes exactly one --version call.
   default: none stated.
Extra user decisions from R:
U5 /model <name> while conductor on (repl.go:875 default true; classifier.go:165-167 switches back to haiku on next prompt): (a) also turn conductor off; (b) pin model until /model again; (c) keep (next prompt may switch back). 사용자 확인 필요.
U6 plain mode unknown "/x": reject locally, or keep sending to model (custom commands/skills)? 사용자 확인 필요.
U7 /clear with running agents: stop and mark interrupted, or refuse until they finish? 사용자 확인 필요.
haiku-specter request: no such agent in .claude/agents. Nearest: haiku-scout (haiku, effort high). Name/scope 사용자 확인 필요. Agents spend tokens; chosen local commands spend 0 by design.

## 3. Tests
harness (P d, revised by R2): fake claude on PATH (t.Setenv) + ClaudeBin. R2: startup always spawns (loop repl.go:796, runTUI tui.go:1093 call r.start()), so "log empty" cannot pass; fake exit-1 is fragile. Revised: count via newSessionFn seam (resumeshift_test.go:68), take baseline after startup, assert delta 0. Keep PATH fake for ONE acceptance test with a child that blocks on stdin (cat >/dev/null). Add execOutput seam for /version. Run go test -race (/status reads lastCtx/rotations while idleFinish may write).
tests (P d):
- TestLocalHelpNoSpawn: output has "/help"; delta 0
- TestLocalStatusNoSpawn: output has model, "ctx", threshold; delta 0
- TestLocalModelNoArgNoSpawn: output "model <a.Model>"; delta 0
- TestLocalMouseNoSpawn: mouse flag toggled; delta 0
- TestLocalExitNoSpawn: quit cmd returned; delta 0
- TestLocalClearNoSpawn: session reset, sess nil, needStart/ensureSession; delta 0; running tracker row ends interrupted (R4)
- TestLocalAgentsNoSpawn (if D6 local): lists running tracker rows
- TestLocalVersionNoSpawn (if D3): exactly one exec call args ["--version"], never "-p", delta 0 (R7)
- TestLocalCostNoSpawn, TestLocalContextNoSpawn (if D4/D5 local): from metrics / lastCtx
- TestLocalRotateNoSpawn (if D1/D2 deterministic): deterministic path, delta 0
- TestPlainLocalCmdsNoSpawn: chosen commands via loop (HARNESS_PLAIN=1 / piped stdin), delta 0
extend existing: acceptance_test.go (builds binary), usagebusy_test.go.
negative control: /usage and /compact must register a spawn/Send in harness (R2: /usage via passthrough increments Send count, not spawn count, since child exists).
golden test: TUI output byte-identical after localCmd extraction (R10 step 2).

## 4. Review findings
P-review (R), all 10 items:
R1 passthrough /cost /context /agents /usage /mcp /config already 0 model tokens (repl.go:790 comment; usageLine skipped when usage empty). Do not port to Go. Plan edit: mark keep. Porting /cost risks double counting (metricscmd.go per-turn sum vs cumulative total_cost_usd, unverified) -> verify first. Verification step: run each once, check transcript assistant usage empty and total_cost_usd unchanged.
R2 test "log empty" cannot pass (startup spawns: repl.go:796, tui.go:1093); fake exit-1 fragile (NewSession succeeds then lines close; /usage negative control depends on Events on dead child). Use newSessionFn counter + delta; execOutput seam; go test -race.
R3 /clear lazy: needStart flag alone panics or misroutes. r.sess deref sites: repl.go:393 527 566 571 609 623 714 725 744 772; tui.go:874; classifier.go:180 (route closes r.sess when no turns); classifier.go:194/202 nil-checked. Spec:
   - clear(): r.sess.Close(); r.sess=nil; reset tracker like start() via shared helper resetTracker(parent func() string) (new tracker, keep sortedRestored); no start().
   - func (r *Repl) ensureSession() error { if r.sess != nil {return nil}; return r.start() }
   - handle(): call route() first (route nil-safe: classifier.go:179-183: if r.sess nil only set r.a.Model, no Close/start), then ensureSession() before repl.go:714; on error print "cannot start claude", return, do not record user turn.
   - passthrough(): ensureSession() at top.
   - rotate(): if r.sess nil -> no handoff, no Close; print "── nothing to rotate (new session starts with next message) ──"; return. Model change picked up by later start().
   - importSession(): check resume.go:82 (calls clear then applyImport); must not need live child (verify).
   - /status tui.go:874 and usageLine repl.go:527: show "-" when r.sess nil.
   - runTUI/loop defers already nil-check (tui.go:1098, repl.go:804).
   - Hook side note: .claude/settings.json SessionStart (matcher includes clear -> session-context.py) and SessionEnd (history-export.py, 60s timeout). Eager child closed unused may run hooks and write empty export (verify).
R4 /clear kills running agents unmarked: clear() Close with no drain, no markInterrupted; agentStore rows stay "running". Fix: before Close, mark like exit does, or warn "N agents running; /clear stops them" (U7). Add test: tracker row ends interrupted.
R5 /model <name> semantic bugs (fix regardless of D2):
   a. r.a.Model set before rotate (tui.go:1129); if rotate holds (repl.go:596-600) label shows new model while child runs old.
   b. no turns -> handoff is pure cost; classifier.go:179 already does Close+start without handoff. Extract switchModel(name) for tui submit and plain loop: validate; if no turns and pendingSnapshot=="" -> restart (or lazy nil) 0 tokens; else rotate("model "+name); on hold or start error revert r.a.Model and say so.
   c. no validation: "/model foo" -> start fails, closed session stays, next Send writes closed stdin, Events waits up to 300s. Accept haiku|sonnet|opus or "claude-" id; else "unknown model".
   d. handoff runs on old model (e.g. opus handoff before switching to haiku); state this cost in D2.
R6 /model overridden by conductor on next prompt: repl.go:875 default true; classifier.go:165-167 switches to haiku when classifier says haiku and r.a.Model != haiku; 171-186 switches to other routed model. -> U5.
R7 /version defects: tui.go:925 exec.Command("claude") literal (not osenv.ResolveBin / SessionOpts.ClaudeBin); runs synchronously in Update (UI blocks); error ignored (prints "claude " empty); no harness version. Fix: tea.Cmd async, ResolveBin, cache after first call, show harness version too, "claude version unavailable" on error. Test: exactly one call ["--version"], never -p, delta 0; passes under both D9 options. Alternative in D3: init-event claude_code_version (unverified).
R8 localCmd scope: /mouse and /exit need tuiModel side effects (tea.EnableMouseCellMotion, quit()); cannot live in localCmd(line)(out,ok). localCmd covers read-only text only: /help /status /model(no arg) /mode(no arg) /version(cached). Side-effect commands stay per path. Plain loop additions after echo (repl.go:831-833): /exit (same as /quit; currently sent to model); /clear -> r.clear(); /model <name> -> switchModel; unsupportedCmds -> note (currently sent to model); /mouse -> "TUI only" note; unknown "/x" not in passthroughCmds -> "unknown command" instead of handle() (U6; changes custom commands/skills in plain mode).
R9 /rotate hybrid: deterministic fallback exists (repl.go:603-606, buildSnapshot). Options: keep handoff (recommended: purpose is quality summary); always deterministic; deterministic when lastCtx small or turns <= TailTurns. With lazy /clear, rotate on nil sess is a no-op (R3).
R10 implementation order: see section 1.
Safe-to-run classification (R):
   read-only: /help /status /model(no arg) /mode(no arg) /resume (list/picker) /version (spawns claude --version only)
   local state change (intended): /mouse /mode <name> /mode conductor|conductor-off /exit /quit
   session state change (drops child): /clear /import /model <name> /rotate. Close runs SessionEnd hook; running agents die. List separately.
User decisions (R): U1 = D8; U2 = D4/D5/D6; U3 = D1/D2 (incl. old-model handoff, hybrid); U4 = D3; U5, U6, U7 as above.

Routing review (RR, Korean source, findings in English):
RR-Q1 structure: main session fixed Haiku (classifier.go:38-41 pickModel returns haiku when cur ""/haiku; pickTarget :49-51 haiku regardless of kind). Classification is NOT Haiku: separate one-shot `claude -p --model sonnet` per prompt (classifier.go:14 conductorModel="sonnet", :109-115 classify; no tools, no session, cwd tmpdir :115-117). Output 5 tag lines topic/size/kind/worker/task (classifier.go:14-15). size=large adds planHint (classifier.go:92-95, branch :94): Haiku plans -> opus-architect/sonnet-analyst write improvements only -> Haiku executes (classifier.go:31-34). Escalation is by Sonnet tag, not Haiku judgment; no Haiku self-escalation code found (scope: classifier.go, CLAUDE.md, DECISIONS.md grep).
RR-doc mismatch: CLAUDE.md:2 header and :9 "Opus main" paragraph are legacy (applies only to forced Opus main; CLAUDE.md:31 says so). DECISIONS.md:30-33 records old design (Sonnet picks target haiku|sonnet|opus); :38 overrides. Model output of classifier is effectively unused (always haiku); worker field is the only live part, so prompt is longer than needed (opinion).
RR-cost: each prompt pays a Sonnet classify call (metrics stage=classify, DECISIONS.md:32).
RR-Q2 main vs subagent token use (opinion; no measurement): subagents win for one-shot/independent/search-heavy/parallel work and when cache TTL breaks often; main Opus/Sonnet wins for 5-10+ turn continuous design/debug on the same context, if brief would be as long as history. Opus main delegating to Opus wastes (CLAUDE.md:9). Context-growth controls already exist: rotation at 270k (DECISIONS.md:16), idle rotation (:19-23), auto compact 150k (CLAUDE.md:26). Session-worker experiment (DECISIONS.md:25, :27) has no results yet (unverified).
RR-recommendation: keep current (Haiku main + Sonnet classify + Opus/Sonnet subagents); measure session workers per DECISIONS.md:27. Risks: per-prompt Sonnet overhead on small prompts; Sonnet mis-tags size -> Haiku does hard work and reworks; estimates unmeasured.
RR-open: average classify cost vs Haiku handling cost; needs aggregation of notes/metrics/*.jsonl stage=classify (not done).

## Not found / gaps
- P gives no recommended default for D1, D2, D3, D7(no), D8, D9 beyond what R states (see section 2).
- No test code exists yet; all tests above are proposed names.
- Plan-level line refs are as of 2026-10-29; re-grep before editing (P:5).
