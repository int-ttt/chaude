# 261029 analysis: test organization, two Opus designers vs one Opus + executors
scope: TESTING of multi-process harness only (not session layout). isolation default: A (one git worktree per task).
sources: 261029-plan-task-isolation.md, 261029-test-remaining.md, 261029-test-parallel-terminals.md, 261029-research-budget-flag.md, internal/harness/acceptance_test.go, conductor_test.go
no source or DECISIONS.md edits.

## facts (확인됨)
- offline: go test -count=1 -> 100 PASS, 0 FAIL, 3 SKIP, 0.364s; -race ok 1.587s; vet clean.
- live tests = 3: TestLiveRotationCarriesContext (HARNESS_LIVE=1, builds chaude, runs repl with cmd.Dir=repo root), TestLiveStreamingMultipleDeltas (HARNESS_LIVE=1, NewSession default opts), TestClassifierLive (CHAUDE_LIVE=1, comment says ~$0.007 per prompt, 6 prompts per DECISIONS 0.6.8).
- parallel headless: 3 concurrent haiku calls all rc 0, wall 3.1s vs sum 9.1s; 4 calls total $0.0119; limits used: stop 2.50, budget 3.00.
- --max-budget-usd: print mode only, client-side estimate, can overshoot, subagent spend counts.
- live tests in acceptance_test.go pass no --max-budget-usd themselves (read in code); whether NewSession sets a cap: 가정 none.

## key observations
- live test spend is tiny (estimate below $0.10 per full live pass: classifier ~$0.04 + 2 short sessions; 가정 for the two session tests, model not pinned in test). The dominant quota cost is the AGENT sessions that design/run tests, not the tests. Two Opus designers cost more than all live tests combined (가정: per Opus agent session tokens >> $0.1 equivalent).
- subscription login: total_cost_usd is an estimate, not the quota meter; third-party report says the cap may misbehave on subscription (확인 안 됨). So the $3 gate is a soft proxy; one owner must keep one running total.
- worktree risk specific to now: master has many uncommitted files (children.go, dispatch.go, clients.go, tests). A new worktree checks out a commit, so tests in it would run HEAD 90be2af, not the working tree (가정 per git behavior, not verified in this repo). Tests run in A must start from a commit/branch that contains the change under test.
- TestLiveRotationCarriesContext uses filepath ../.. as root, so inside a worktree it builds the worktree's code (good). Where LEDGER/metrics get written from a worktree: unverified (plan-task-isolation first check).

## option 1: two Opus designers (offline owner + live/budget owner), shared contract file
- cost: 2x Opus session quota for design; live spend unchanged. Budget gate has one owner (live designer) -> OK on paper, but offline designer's own agent spend is outside that gate unless both log to one shared total.
- flakiness/conflicting expectations: two authors of expectations for the same behavior (e.g., rotation carries context; offline fake vs live CLI). Contract file reduces drift but needs both to re-agree on every change -> negotiation loop between two Opus agents; no tie-breaker.
- disagreement offline vs live: no single decider; each owner naturally defends its own result. Needs a third party (user or main) anyway -> option 1 collapses into option 2 at the decisive moment.
- failure recovery: per-designer worktree is clean to discard, but the contract file is shared across worktrees -> must live outside both (main repo or absolute path) or it forks per branch (A isolates notes/).
- parallelism: 2 Opus + their executors; highest quota burn; harness rule "never delegate to another Opus agent" also cuts against Opus-Opus collaboration.
- good when: offline and live test design are each large, independent, design-level work (e.g., building a fake claude CLI stub + designing a new live acceptance matrix at the same time).

## option 2: one main Opus owns plan + budget; Sonnet/Haiku execute in parallel worktrees; main decides
- cost: 1 Opus (plan + verdicts, short), executors cheap. Offline runs are 0 API quota (go test only; agent tokens only). Single owner of the $3 total by construction.
- flakiness/conflicting expectations: one author of expectations; executors only run, collect, report raw (exit code, PASS/FAIL/SKIP counts, cost, rc). No expectation drift between executors.
- disagreement offline vs live: main decides with a fixed rule (below).
- failure recovery: executor fails -> discard its worktree, recreate from the same commit, rerun once; second failure -> main diagnoses (escalation, not a retry loop).
- parallelism: bounded by main's budget table, not by negotiation.
- weak when: main becomes a design bottleneck (both test layers need new design at once), or main's context fills with raw logs (mitigate: executors write to notes/runs/, return <=5 lines).

## decision rule when offline and live disagree (owner: main Opus)
- offline FAIL, live PASS: offline is the regression gate -> code bug or stale test; do not release; fix code or update the test with a stated reason.
- offline PASS, live FAIL: rerun the live test once (same worktree, same commit). Fails again -> live is ground truth for CLI behavior; offline fake/assumption is wrong -> add an offline case reproducing it, then fix. Passes on rerun -> record as flaky in notes/runs with cost of both runs; 2 flakes on the same test -> main redesigns that test (timeouts, assertion looseness).
- both FAIL: offline first (free), live after offline is green.
- live never runs while offline is red (saves quota).

## budget gate (single owner = main)
- one running-total file at an absolute path outside all worktrees (e.g. main repo notes/runs/<date>-live-budget.md), because A isolates notes/ per worktree and per-worktree totals would each restart at 0.
- executor appends total_cost_usd per live call; main checks before each live batch: stop at 2.50, hard 3.00 (same limits as parallel test).
- per call: --max-budget-usd 0.20 where the CLI is called directly; tests that spawn sessions internally have no cap (가정) -> run those sequentially and record cost from transcript/result.
- under subscription the estimate is a proxy only; if the usage meter in the status line moves more than the estimate suggests, main halts (가정 that the meter is readable).

## how many in parallel
- offline: 1 haiku-runner per worktree is enough (suite takes <2s). Parallel only across different commits/branches under test; max 2 worktrees.
- live: max 1 live executor at a time; inside it, at most 3 concurrent headless calls (the verified number). The 2 session tests run sequentially (unbounded spend, longer runtime).
- total concurrent agents: main + 2 executors (1 offline, 1 live). More adds quota burn (each agent session) with no wall-time gain, since offline is seconds and live is minutes.

## verdict
- option 2. Reasons: budget gate and pass/fail authority need one owner; live spend is too small to justify a second Opus whose session costs more than the tests; option 1 still needs a tie-breaker on disagreement.
- flip to option 1 when: offline and live layers both need new design-level work at the same time (e.g., fake claude CLI stub for offline AND a new live matrix beyond the current 3 tests) AND that design work, not execution, is the measured bottleneck (main's plan for one layer waits on the other). Even then keep the budget gate with the live designer only, the contract file outside worktrees, and the user as tie-breaker.

## first test to run
- worktree baseline parity, 0 API quota: commit (or branch) the current working tree changes, `git worktree add /tmp/wt-test -b task/test <that commit>`, run `go test -count=1 ./internal/harness` and `go test -race -count=1 ./internal/harness` inside it.
- pass: 100 PASS, 0 FAIL, 3 SKIP (same as master working tree). Fewer tests or build errors -> the worktree is not testing the current code; fix the base before any parallel runs.
- next (first live, ~$0.005): one short headless haiku call from /tmp/wt-test with hooks on, then check whether LEDGER.md / metrics jsonl were written inside the worktree or the original repo (plan-task-isolation first check). Then `git worktree remove /tmp/wt-test` and delete the branch.

## open
- committing the uncommitted master changes is a user decision (git rule: commit only when asked).
