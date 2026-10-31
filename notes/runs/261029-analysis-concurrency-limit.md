# concurrency limit (261029)
conclusion: chaude has NO enforced cap on concurrent claude children. DefaultMaxWorkers=3 is parsed but never used.
evidence:
- dispatch.go:11-12 DefaultMaxWorkers=3; dispatch.go:16-30 resolveMaxWorkers (flag > CHAUDE_MAX_WORKERS > default; <1 error)
- repl.go:66,1018,1035-1040 a.MaxWorkers set; grep of all non-test *.go: no other reader of MaxWorkers -> no enforcement (dead config)
- children.go:71-90 childSet = map keyed by childKey(cwd,model) (children.go:16); Park replaces+closes same-key child. Limit is implicitly 1 parked child per (cwd,model) + 1 active; no count check.
- dispatch.go:61-83 pickChild: resume only if registry row not owned by another live chaude (OwnerPID alive) -> multiple chaude processes can run side by side, each with own children.
- idlesweep.go:33-60 sweepIdle only reaps dead/cold parked children (time based), not a count cap.
effective limit: models(~3: haiku/sonnet/opus) x cwd per chaude process, time-bounded by idle sweep; across chaude processes unlimited. Not blocked from running several clients.
processes (ps/lsof):
- chaude 61618 (parent zsh 9704 <- GoLand): owns 61653 + 65110, both `claude -p stream-json --model haiku`, cwd claude-harness. Consistent with chaude (headless -p, same flags).
- 90171 `claude -c` cwd univ (parent zsh 56587), 8104 `claude` cwd claude-opus-lean (parent zsh 31793), 74326 `claude` cwd TextGame (parent zsh 66129), 51677 `claude` cwd claude-harness (parent GoLand 9181): interactive, started from shells/IDE, not chaude children (no -p stream-json, parent not chaude). Inference from ppid/args.
oddity (uncertain): two haiku children with identical args/cwd under one chaude (21m and 5m old). Per-key design should hold one per (cwd,model) in childSet; the second is likely the active child plus a replaced/rotated one not yet closed, or a short-lived second spawn. Not verified; worth checking switchchild.go:86-90/221 and rotate path if it persists.
