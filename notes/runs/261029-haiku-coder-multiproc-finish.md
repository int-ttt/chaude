# haiku-coder multiproc finish run
date: 261029
plan: notes/runs/261029-plan-multiproc-finish.md
order: U3b, U7a, U5a, U5b, U5d, U6b, U7b, U8

## done
- U7a [H]: internal/harness/clients.go (expiredZero added after idleDeadline); internal/harness/clients_test.go (TestExpiredZeroDeadline appended)
  - check: go build ./... PASS; go vet ./internal/harness/ PASS; gofmt -l clean
  - test: go test ./internal/harness/ -run 'ExpiredZero' -count=1 -v => PASS TestExpiredZeroDeadline
  - full: go test ./internal/harness/... -count=1 => ok chaude/internal/harness

## blocked
- U3b [H part]: gate G1 (plan §1) not met. G1 = user approval for U0a commit; repl.go units WAIT until approved. Not edited. Agent instruction is not user approval.
- U5a, U5b, U5d, U6b, U7b, U8: plan labels S (sonnet-analyst). Depend on U3b (repl.go gated). Not started. Route to sonnet-analyst after G1.
- U-pre, U0a, U-rel-1, U-rel-2: out of this run per instruction. Not done. No commit, tag, or version bump.

## doubts
- expiredZero: spec §4 U7a defines deadline<=0 -> false (returns deadline>0). U7b pseudocode says "if expiredZero(deadline) -> false", which inverts that. Resolve in U7b spec before use (likely !expiredZero).
- isWarm not changed (not in U7a scope).
- untracked/dirty tree (G1 pending) unchanged except the two clients files.

## run 2 (261029, same plan)
- U-pre [H]: baseline on tree before commit: go vet ./... PASS; go test ./... -count=1 => ok chaude/internal/harness (other pkgs no test files).
- U0a [H]: commits (no push):
  - e3c499d "classifier: size tag defaults to large; status line: 5h rate usage" (internal/ all: classifier, conductor_test, core, repl, tui, unit_test, + new children/clients/dispatch files and tests)
  - 1773929 "metrics: 2026-10-29" (notes/LEDGER.md, notes/DECISIONS.md, notes/metrics/2026-10-29.jsonl)
  - untracked notes/runs/*, notes/research/billing.md, notes/archive/ left untracked (plan §4 U0a item 3).
  - note: internal/harness/classifier.go is NOT gofmt-clean at HEAD (pre-existing, not touched).
- U3b [H]: done (uncommitted, in 0.8.0 tree). repl.go: 7 Repl fields (reg, children, idleRotate, ttl, lastAPI, ttlSeen, idleFiredFor); wireClients() called in Main after newAgentStore; sets reg/children only when ClientsFile != "".
  - test: cachettl_test.go TestRegistryNilWhenNoFile added (imports path/filepath, testing, time). PASS.
  - check: go build ./... PASS; go vet ./internal/harness/ PASS; go test ./internal/harness/... -count=1 => ok.
- U7a [H]: done in run 1 (TestExpiredZeroDeadline). Still uncommitted.
- U5a, U5b, U5d, U6b, U7b, U8 [S]: NOT started in this run. Plan §4 routes S units to sonnet-analyst; they change the event loop (repl.go idleEvent/handle), classifier route, and tui TickMsg under -race. Not a minimal fix. Handoff: sonnet-analyst, order U5a -> U5b -> U5d -> U6b -> U7b -> U8.
- U-rel-1 [H]: blocked. Plan §4 U-rel-1 depends on U8 PASS. Also D3: 0.8.0 scope includes U5/U6b/U7/U8; releasing now would ship without them.
- U-rel-2: blocked by U-rel-1.
- tag v0.8.0: not created. no push.
- doubts: U7b pseudocode inverts expiredZero (run 1 doubt still open; resolve in U7b spec: use !expiredZero).
