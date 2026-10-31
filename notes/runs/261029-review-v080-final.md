# review: v0.8.0 final (uncommitted internal/harness vs 5e415b5)
verdict: PASS
blocking: 0
checks:
  go build/vet: pass
  go test -count=1 ./...: pass; -race ./internal/harness: pass
  BUG-1 ChildPID: ok; Session.PID (core.go), childPID, set in all 4 Upserts (switchchild.go:91,207,223,233) + recordDesignPath; TestChildPIDRecordedOnPark
  BUG-2: ok; reconcile (clients.go) covers live+parked, ChildPID alive -> expired "orphan-alive", never parked; test in budget_test.go:165
  U9 default off: ok; flag --task-budget-usd default 0, budget nil when 0, all methods nil-safe, --max-budget-usd only when >0
  resume cost 0: ok; resumeBaseline()==0 (budget.go), tested budget_test.go:57
  scroll rule: ok; tui.go diff touches only /clear text, /clear dispatch, /model endParked; no refresh()/viewport change
  secrets/email: none (grep 2500slee, gmail, sk-ant, api key in diff + internal/: 0 hits)
issues:
- low | core.go:385 | doc comment "ID is the session id..." now sits above PID(); move PID() above that comment
- low | repl.go:446-449 | budget-blocked runTurn returns "" and handle() still appends an empty turn to r.tr; makeSnapshotN retries twice -> budgetStop printed twice. Fix: return early in handle() before tr.add, or dedupe notice
- low | budget.go:12 | last map[*Session]float64 never pruned (leak per session, tiny); delete entry on Session close
- low | clients.go reconcile | pid reuse can mark a row orphan-alive falsely (display only, no kill); acceptable
- low | budget.go:84 | priceTable (Opus/Sonnet/Haiku 5.5) not verifiable offline; trusted from docs.claude.com fetch 2026-10-29
- info | classifier.go | gofmt -l flags it, pre-existing at HEAD, not from this diff
- info | git stash list holds an old stash@{0} (542b1e2, pre-existing); I ran `git stash` once by mistake, it was denied by the permission system; tree unchanged (48 status entries)
note: idlesweep.go and multiproc_s_test.go are already committed at HEAD (not in uncommitted diff).
