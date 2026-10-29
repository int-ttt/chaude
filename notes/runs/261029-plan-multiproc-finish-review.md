# review: 261029-plan-multiproc-finish.md
date: 261029
verdict: plan mostly sound; fix 6 name/signature mismatches and 5 ordering/test gaps before execution. no source edited.
method: grep of internal/harness/{children,clients,dispatch,repl,classifier,tui,localcmd}.go, cmd/chaude/main.go; notes/runs/scroll-bottom.md

## A. wrong names / signatures (evidence: code)
A1 plan §3 U3b: "SessionOpts.CacheTTL = parseCacheTTL(a.CacheTTL)" -> already wired. repl.go:181 passes `CacheTTL: r.a.CacheTTL`; repl.go:912 validates via parseCacheTTL (returns (string, error), repl.go:869). Args fields CacheTTL/IdleRotate/ClientsFile exist (repl.go:62-64). Real remaining gap: Repl has NO fields reg / children / lastAPI / ttlSeen / idleRotate (grep empty). Rewrite U3b as "add Repl fields + construct registry/childSet in Main/start", drop the CacheTTL passthrough. TestCacheTTLFlagWiring probably already covered by cachettl_test.go; check before adding.
A2 §3 U3b: `r.children = &childSet{}` -> nil map; Park writes s.m[key] (children.go:55) -> panic. Use `newChildSet()` (children.go:50).
A3 §3 U5a: "liveChild gets mu, LastAPI, parked" and "start() allocates *liveChild" -> liveChild is in children.go:33 (not repl.go) with exported fields Sess closer, Tracker, Model, Role, LastAPI, LastCtx, Files, TurnFrom; no mu, no parked, no cat/ttlSeen/pendingSnapshot. LastAPI exists already. Plan's field list in U5b ("sess, tracker, Model, lastCtx, cat, ...") must map to these names; Sess is `closer` (Close only), so rebinding to r.sess (*Session) needs a type decision (closer vs *Session; `s.lines` closed check in U8 needs the concrete type). Decide in U5a; this is the biggest design hole.
A4 §3 U6b: childSet.Park signature is Park(key, c) (children.go:55); plan writes `childSet.Park(old)`. Key = childKey(cwd, model) (children.go:14). Park also closes any prior child under the same key: plan rule "SessionID in childSet never resumed" must also state Park-replace closes the old one (data loss if old had running agents -> drain first).
A5 §3 U6b step 1/3: plan calls `switchSession(target, worker)`; actual signature switchSession(model, resume string) (classifier.go:269) where resume is a SESSION ID. Verify call sites pass rr.Worker as a session id (classifier.go ~181-198) and say so; naming it "worker" is misleading.
A6 §3 U6b "new seam `var classifyFn = classify`": classify is at classifier.go:178, called at classifier.go:218. Fine, but tests listed (TestRoute*) need it as a prerequisite sub-step ordered BEFORE the tests; and classify shells out to real `claude` (exec.Command) so without the seam tests cost money. Put seam in U6b step 0.
A7 §3 U5b "tui.go:312/329/378/983 read tracker per tick" and "/status ~tui.go:890": real hits are tui.go:374-378 (TickMsg reads m.r.tracker.running()) and tui.go:891 (`m.r.sess.SessionID` unguarded). Line numbers drift; cite function names instead. localcmd.go:36 already guards `r.sess != nil` (pattern OK).
A8 §3 U5d: "Fixes empty-session rotate panic" unproven. repl.go:606 already guards len(r.tr.turns)==0 on the failed-handoff path. Either show the panic path or drop the claim; makeSnapshot extraction must keep the `held` early-return (repl.go:600-611, r.lastRotFail) which has side effects (logRotate, r.p) - a "no close, no start" helper returning (snap, ok) cannot express "held" vs "invalid but carried"; define a 3-state return.

## B. ordering risks
B1 §4 "U3b -> U5a": U3b creates r.children, but U5a changes liveChild layout; build registry/childSet wiring after the liveChild shape is fixed (A3), or Haiku wiring in U3b will be rewritten.
B2 §4 U7 depends on U5a (lastAPI). But U7 text uses r.lastAPI and r.idleFiredFor while U5a stores LastAPI on liveChild; define single source (active child's LastAPI) in U5a, else two clocks.
B3 §3 U8 -> U6b ordering OK, but sweepIdle's "active child idleDue -> rotate-like close" re-enters rotate() (repl.go:575) which uses pendingSnapshot early-exit (repl.go:578-584 "snapshot_kept"). Test that an idle rotate with an existing pendingSnapshot does not drop it.
B4 §1 G1/§4: U-pre listed before U0a approval but "parallel allowed U-pre with U0a staging": U-pre runs go test on a dirty tree (classifier.go etc. modified); its baseline is only valid for that dirty tree. Record `git diff --stat` hash in the baseline file; otherwise PASS names are not reproducible after U0a.
B5 §3 U-rel-1 vs §4: [haiku] steps §5 says U-rel-1 "only after U-rel-2 PASS", but §3 U-rel-1 depends "all units PASS" and §4 puts U-rel-1 BEFORE U-rel-2. Reviewer must see the version bump; fix §5 text (or keep §4 order and delete the contradiction).

## C. missing tests
C1 U5b: no test for Close-ordering of c.parked.Store(true) FIRST vs idle callback racing (race test only listed in U5a). Add TestUnbindParkedRace with -race.
C2 U6b: add test for Park-replace (same key twice) closing the older child exactly once.
C3 U8: TestSweepIdle* needs fake clock; sweepIdle(now) takes now (good) but tick path tui.go TickMsg must pass time.Now() only; add a test that sweepIdle is NOT invoked while busy / permission prompt (A6 in plan) - listed in text but no test name.
C4 U-rel-1: add check `go run ./cmd/chaude --version` string is "chaude 0.8.0" (main.go:42 prints "chaude " + version) - plan acceptance says "prints 0.8.0".
C5 CloseAll on quit: tui.go:1118 defer closes only r.sess inside `if r.sess != nil`; test also the case r.sess == nil after unbindActive failure (start failure path U6b step 5).

## D. release risks
D1 main.go:16 `var version = "0.7.2"` is overridable by -ldflags (main.go:15 comment); verify no Makefile/goreleaser pins another version before bump.
D2 U-rel-2 commit trailer says "Claude Haiku 5.5"; current session attribution reminder says "Claude Sonnet 5.5 <noreply@anthropic.com>". Plan §3 note claims it matches "the given instruction" - not verifiable; confirm with user, default to the active attribution.
D3 CHANGELOG top is 0.6.6 while code is 0.7.2: 0.8.0 section lists only new items; fine, but changelog says "유휴 3599초 핸드오프 후 종료" - ships only if U8 lands. If U8 slips, drop the line (flag defaults at repl.go:903 already advertise 3599s idle-rotate; with no sweeper the flag is a no-op: release risk of a dead flag).
D4 Non-TTY loop (repl.go loop defer ~810) has no timer: idle-rotate silently ignored. State in --help text, not just docs.

## E. scroll-bottom rule (notes/runs/scroll-bottom.md) risks
Rule: refresh() (tui.go:331-337) follows tail unless m.held && !atBottom; GotoBottom is allowed on typing (tui.go:682-683), sendPrompt (714-715), popup/agent view close.
E1 §3 U8 sweepIdle runs inside TickMsg (tui.go:374). Any notice it emits (rotate banner, "[parked ...]" queued notice, expired msg) must go through r.p -> refresh(), never call m.vp.GotoBottom() directly; otherwise a user who scrolled up (m.held) is yanked to the bottom every idle event. Add TestIdleNoticeKeepsHeldOffset in agentview_test.go next to TestWheelScrollHoldsPosition.
E2 §3 U5a parked-event plumbing: parked child "renders nothing"; ensure the queued notice is appended at the NEXT user prompt (not rendered immediately), else it appends output while held and sets unseen indicator spuriously. Add test: parked background reply does not set m.unseen.
E3 §3 U5b bindChild/unbindActive change r.sess/r.tracker under the UI; tui.go:891 /status crash (A7) also sidesteps tail-follow; guard and re-test Model switch while m.held (switch banner must keep offset, as "held && not at bottom -> offset kept").
E4 §3 U6b start-failure rollback prints error: print through r.p (refresh path), not stderr (classifier.go switchSession uses fmt.Fprintln(os.Stderr) in alt-screen: corrupts display and bypasses scroll logic). Pre-existing, but U6b step 5 should not copy it.
E5 acceptance for TUI-touching units (U5b, U8): add `go test -count=1 -race ./internal/harness -run 'ScrollBottom|WheelScroll'` per scroll-bottom.md verify line.

## F. open questions for user
F1 scope of 0.8.0: U8b/U9/U5c deferred (plan §6) - confirm.
F2 attribution trailer (D2).
F3 liveChild.Sess type: closer vs *Session (A3).
