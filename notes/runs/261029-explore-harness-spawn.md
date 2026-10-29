# explore: claude child spawn map (internal/harness)
task: read-only map of claude child process start, model, tracking, rotation, version, tests
1 spawn:
  - core.go:108 NewSession(o SessionOpts) (*Session, error); ClaudeBin default "claude" (core.go:113)
  - core.go:115-116 args: -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model o.Model
  - core.go:117-122 optional --permission-prompt-tool stdio, --resume o.Resume, o.Extra...
  - core.go:124 exec.Command(osenv.ResolveBin(o.ClaudeBin), args...); cmd.Env=cleanEnv(o.CompactWindow); cmd.Dir=o.Cwd
  - struct SessionOpts core.go:81 {Model, Cwd, Resume, Extra, ClaudeBin, NoPermission, CompactWindow}
  - struct Session core.go:92 ("one claude -p child"); Send core.go:161; Close core.go:321
  - repl.go:176 Repl.start() builds SessionOpts; newSessionFn var repl.go:174 (= NewSession, test seam)
  - repl.go:177 opts.Model = r.a.Model; Extra = --append-system-prompt rule (repl.go:185)
  - other claude spawns: classifier.go:122 classify() exec.Command("claude","-p",...,"--model",conductorModel); tui.go:695 "/usage" probe; tui.go:941 "claude --version"
2 model:
  - repl.go:868 flag -model default "haiku" (Main, repl.go:865)
  - Args.Model repl.go:41 -> SessionOpts.Model repl.go:177 -> core.go:116 --model
  - core.go:104-106 empty Model defaults to "haiku"
  - classifier.go:14 conductorModel = "sonnet" (classifier only)
  - repl.go:461 r.a.Model=="haiku" checked in runTurn (direct Haiku path)
  - tui.go:1150 model switch: r.rotate("model "+f[1])
3 tracking (Repl fields, repl.go:129-163):
  - sess *Session (current main child); tr *Transcript; rotations int; lastCtx int; rate map
  - tracker *agentTracker (rebuilt in start, repl.go:180-181, tk.parent=s.ID); agents *agentStore
  - draining []*drain (old sessions); pool *workerPool (Haiku sessions); parked map[model]sessionID
  - resumeID string (consumed once by start); curTask; cat string (current [cat: X]); pendingSnapshot string
4 rotation/handoff:
  - repl.go:571 rotate(reason) -> runTurn(handoffPrompt) up to 2 attempts, validateHandoff (handoff.go:47), start() new child, old drained (repl.go:669 pollDrains)
  - repl.go:571-580 pendingSnapshot branch: restart without handoff, rotations++
  - triggers (all call rotate):
    a. repl.go:764 ctx: autoOK && cooled && r.lastCtx >= effectiveThreshold() (repl.go:695: opus/sonnet = opusSonnetThreshold 270000 repl.go:27; else a.Threshold default 80000 repl.go:870); inside handle() repl.go:702-775
    b. repl.go:757 category switch (catRot && cooledCat), handle()
    c. repl.go:830 "manual" (loop, repl.go:801)
    d. tui.go:1135 "manual"; tui.go:1150 "model X"
    e. classifier.go:196 "route <model>" (route(), classifier.go:145; switchSession classifier.go:204)
  - flags: -threshold (repl.go:870), -max-auto-rotations, -no-category-rotate, -no-handoff-validate
5 version/changelog:
  - version var: cmd/chaude/main.go:16 `var version = "0.7.2"` (ldflags -X main.version); printed main.go:42
  - CHANGELOG: /Users/leekwangseok/PycharmProjects/claude-harness/CHANGELOG.md; newest heading line 3 "## 0.6.6"; zero "## 0.7" entries (stale vs 0.7.2)
6 tests:
  - no repl_test.go exists; 24 *_test.go files in internal/harness, feature-named (conductor_test.go, resumeshift_test.go, localcmd_test.go, unit_test.go, idle_test.go, etc.)
  - pattern: plain func TestXxx (conductor_test.go:9 TestParseRoute, :54 TestClassifyPromptSizeRule)
  - child stubbing: package var swap newSessionFn (localcmd_test.go:13, resumeshift_test.go:68), restored with defer
  - Repl literal construction: &Repl{...} used in agentview_test, idle_test, localcmd_test, mode_test, resume_test, resumeshift_test, unit_test, usagebusy_test, batchinput_test, agentstore_test
not found: no repl_test.go; no other exec of claude outside core.go/classifier.go/tui.go in internal/harness (non-test).
