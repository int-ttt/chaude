# scope: B~F + M0 test on T1~T5 (user: "앞서 논의한 모든 안(a~e)을 전부 테스트해서 결과 비교"; a~e read as B~F, A excluded)
status: NOT run. escalated before any spend.
evidence (from notes/runs/261030-plan-orchestration-options.md, read by scout):
- L2: plan only, nothing measured; all usd deltas = estimates from 10-29 metrics.
- L6: decision NO for now; L42: "test plan (do not run now)".
- L43: 2 runs/option (cold+warm), fresh session per run, same start commit; reuse headless harness 261030-sonnet-analyst-ab-headless.md + ab-v2-*.jsonl; cost from notes/metrics/<date>.jsonl.
- L53/L56: acceptance B <= 0.85 x M0 and PASS >= M0; order M0 -> B -> C(only if B accepted) -> D -> F -> E(only if D or F accepted).
- M0 numeric usd: not in file (only definition: current routing measured from metrics 10-29..10-30).
blockers:
1. E needs a new hook gate (main-thread-guard.py or agent-model-pin.py: RETURN cap <= 0.25 x raw bytes) that does not exist; F needs workstream-id routing to livechild.go. B/C/D are prompt/policy only. So E and F cannot be tested without code changes (hooks + internal/harness), which the plan says not to implement.
2. T3 needs a seeded one-line bug in internal/harness; working tree already has many uncommitted changes, so "same start commit" is not satisfiable without a clean worktree.
3. Plan conflict: plan says do not run now + conditional order (C only if B accepted, E only if D/F accepted); request says run all. 
4. Cost: 5 options x 5 tasks x 2 runs = 50 runs (+ M0) of real model spend; M0 may be read-only from metrics.
options:
 A) run B,C,D only (policy-only) + M0 from metrics; mark E,F as untestable-without-implementation. cost: 30 runs.
 B) implement E gate + F routing first (>=5 interlocked files, separate task), then run all.
 C) follow plan order strictly (M0 -> B first, stop at gate), cheapest.
open decision for user: which option, and run count (plan says 2).
expected values are estimates only: ranking D > F > C > B > E (risk-adjusted); no measured results exist.
