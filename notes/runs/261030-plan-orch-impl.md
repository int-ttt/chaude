# plan: orchestration modes B/C/D/E/F (impl)
user: "전부 구현한 뒤 테스트해라"
source: notes/runs/261030-plan-orchestration-options.md (:11-30), scope notes/runs/261030-scope-orchestration-test.md
goal: all options selectable side by side for the A/B test; default = M0 (no behavior change).
choice: one switch `--orch m0|b|c|d|e|f` + env CHAUDE_ORCH (lowercase) exported to child + hooks.
  rejected: editing CLAUDE.md per option (uncommitted edits, cannot A/B); separate binaries (dup code).
steps (order):
1 [haiku-coder] internal/harness/orch.go (new): parseOrch, orchRule texts (B/C/D/E/F), orchDefaultModel (b,c,d,f=opus; e=haiku; m0=""), applyOrchEnv -> os.Setenv("CHAUDE_ORCH", mode). orch_test.go. repl.go: flag `orch`, invalid -> error exit 2; model default only if --model not explicitly set (fs.Visit); start(): rule += orchRule.
2 [haiku-coder] hooks: agent-model-pin.py E gate (main thread only, CHAUDE_ORCH=e): absolute paths in prompt -> sum bytes of existing regular files = raw; cap from `RETURN ... <=N chars`; raw>0 and (no cap or cap > 0.25*raw) -> deny "E gate"; sonnet-*/opus-* deny unless prompt contains "ESCALATE". main-thread-guard.py: CHAUDE_ORCH=e -> exit 0 (main works inline). tests/hooks/test_orch_gate.py (unittest, subprocess).
3 [haiku-coder] F: internal/harness/orchws.go (new): parseWS(`^\[ws:([A-Za-z0-9_-]{1,32})\]\s*`), wsState{SessionID, Turns, LastUsed, Ctx, LastText}, wsNeedsRotate (turns>=20 || idle>270s || ctx>=40000), wsHandoff (<=1500 runes). classifier.go route(): orch b|c|d|e -> no classifier switch (main fixed); orch f -> ws-tagged -> haiku child (reuse SessionID unless rotate; on rotate fresh child + handoff prefix), untagged -> stay main. post-turn: record session id/ctx/last text for current ws. orchws_test.go.
4 [haiku-checker] go build ./... ; go test ./internal/harness/ ; gofmt -l internal/ ; python3 -I -m unittest discover -s tests/hooks -p 'test_*.py'
edge: m0 must equal pre-change behavior (no rule text, no env effect in hooks); unknown CHAUDE_ORCH in hooks -> treat as m0; nonexistent paths ignored; dirs ignored.
must NOT change: existing uncommitted edits (no git checkout/stash/reset/restore), CLAUDE.md, settings.json, CHANGELOG.md.
not done here: 50-run real-spend comparison (T1~T5 x 2 runs) - needs clean worktree + user OK on spend.
