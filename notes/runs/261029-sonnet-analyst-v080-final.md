# v0.8.0 final: U8b U5c U9 + BUG-1/2
status: done; go build/vet PASS; go test ./... -count=1 PASS; race (ScrollBottom|WheelScroll|Budget) PASS
BUG-1: ChildPID recorded in all Upserts (switchchild.go), Session.PID() (core.go); test TestChildPIDRecordedOnPark
BUG-2: reconcile covers parked rows; alive ChildPID -> expired "orphan-alive", display-only notice (repl.go reconcileStart); test TestReconcileParkedAliveChild
U5c: endParked/endAllParked; /clear and /model end parked children
U8b: DesignPath recorded (opus plan file write), appended to handoff snap (withDesignPath); Upsert keeps DesignPath
/clear handoff: clearWithHandoff() in TUI only. conflict: DECISIONS.md:13 says "no handoff" -> needs DECISIONS update by main. resume.go/importSession still plain clear().
U9: budget.go new; flag --task-budget-usd default 0 (off); start() appends --max-budget-usd; gates handle+runTurn; mid-turn Interrupt; charge points runTurn, idleFinish, parked result, classifier one-shot; resumeBaseline()=0
U9 not done: /usage one-shot charge (usageFetch has no Repl, proposal-only item); drained sessions uncharged (known gap per plan)
not done: SIGHUP / harness-exit MarkEnded (adding SIGHUP changes SIGTERM-only handler behavior; judgment call)
price: FACTS.md section added; alias map opus=Opus 5.5 sonnet=Sonnet 5.5 haiku=Haiku 5.5 (assumption)
tests: budget_test.go (Delta, OffAllows, ResetPerPrompt, ResumeBaseline, GateBeforeSend, InterruptMidTurn, ExtraAppends, DisabledByDefault, ChildPID, ReconcileParked)
no commit/tag/push/version bump. scroll rule untouched. haiku worker same-model switch kept.
note: gofmt flags classifier.go const block (pre-existing, untouched)
