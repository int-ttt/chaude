# 261029 explore: harness readout (read-only)
status: done
scope: internal/harness (repl.go, dispatch.go, children.go, classifier.go switchSession, clients.go), cmd/chaude/main.go, budget grep repo-wide

## repl.go
- Repl struct: repl.go:132-165. active child = field sess *Session (line 139). Parked map: parked map[string]string model->sessionID (line 160). resumeID string (line 161).
- start(): repl.go:179. Builds SessionOpts with Resume: r.resumeID (180), clears r.resumeID (181), calls newSessionFn -> r.sess = s (196). Uses StartIdle -> idleEvent (220).
- switchSession is NOT in repl.go. Defined at classifier.go:204.
- switchSession(model, resume) classifier.go:204-221: if r.sess!=nil && r.a.Model!=model, parks old id into r.parked[r.a.Model] (209-210); closes r.sess; sets r.a.Model=model; if resume!="" clears pendingSnapshot; sets r.resumeID=resume; calls r.start().
- Park/resume logic call sites: classifier.go:178 (route to haiku, uses rr.Worker as resume id), classifier.go:183-186 (back to non-haiku model with a parked id: delete r.parked[model], switchSession(model,pid)), else classifier.go:189-199 (r.a.Model=model; r.start() or r.rotate("route "+model)).
- route(): classifier.go:145. loop() repl.go:804; /resume dispatch repl.go:847-848 -> resumeLine (resume.go:102).

## dispatch.go (41 lines)
- type childAction int (9); consts actStartNew (12), actUseLive (13), actResume (14).
- pickChild(set *childSet, reg *clientRegistry, key, cwd, model string, limit int, now time.Time, deadline time.Duration) childAction (21).
  - live child in set and isWarm -> actUseLive (22-28)
  - reg.Get row parked + SessionID set (30-33); other live owner pid -> actStartNew (34-36); !isWarm -> actStartNew (37-39); else actResume (40).
- pickChild has no non-test caller yet.

## children.go (113 lines)
- childKey(cwd, model string) string (14): abs + EvalSymlinks + "\x00" + model.
- type closer interface{ Close() } (30).
- type liveChild struct (33-42): Sess closer; Tracker *agentTracker; Model, Role string; LastAPI time.Time; LastCtx int; Files []string; TurnFrom int.
- type childSet struct (45-48): mu sync.Mutex; m map[string]*liveChild.
- newChildSet() (50).
- (s) Park(key string, c *liveChild) (55-63): replaces; closes old Sess if different.
- (s) Take(key string) (*liveChild, bool) (66-74): removes and returns.
- (s) Expired(now time.Time, deadline time.Duration) []string (78-89): does NOT remove; sorted keys with now-LastAPI >= deadline.
- (s) CloseAll() (93-113): parallel Close, empties set.
- No Resume method and no Expire method in children.go. Resume decision is pickChild's actResume (dispatch.go:40); expiry is registry-side: clients.go reconcile(now, deadline) (162), isWarm (135), idleDeadline (145). Registry states: clientLive/clientParked/clientExpired (clients.go:15-26).
- Not wired: childSet/liveChild are not referenced by Repl struct or non-test code (newChildSet only in tests; Park/Take/Expired/CloseAll no non-test callers).

## cmd/chaude/main.go
- line 16: var version = "0.7.2" (YES). Also printed at line 42 for --version. Comment line 15 says set via -ldflags "-X main.version=X.Y.Z".

## budget
- grep -i budget over repo (excluding .git): zero hits in *.go, *.py, *.sh, *.ps1, *.json code.
- claude CLI args in core.go:143-144 (NewSession): -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model <o.Model>; plus --permission-prompt-tool stdio unless NoPermission; --resume if set. No budget flag.
- classifier.go:122 exec claude -p ... --model conductorModel --output-format json: no budget flag seen in the grep.
- Only mentions: notes/runs/261029-plan-multiproc-impl.md:108 (planned, unverified: "--max-budget-usd 3" conditional on claude --help support), notes/DECISIONS.md:22 (plan budget proposals, unverified), notes/runs/261029-plan-overview-cap-review.md:42 (snapshot char budget, unrelated), samples/ jsonl (not read).

## not found
- switchSession in repl.go: not found there (it is in classifier.go).
- Expire/Resume methods on childSet: not found.
- budget flag in code: not found.
