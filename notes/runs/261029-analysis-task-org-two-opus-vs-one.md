# 261029 analysis: task organization, two Opus designers vs one Opus + executors
scope: how TASKS (work items) are organized. isolation default: A (one git worktree per task, branch per task).
sources: 261029-plan-task-isolation.md, 261029-test-parallel-terminals.md, research/client-assumptions.md, 261029-analysis-test-org-two-opus-vs-one.md (testing-only sibling), DECISIONS.md [routing] [main-model] [rotate].
no source or DECISIONS.md edits.

## facts (확인됨)
- parallel headless: 3 concurrent haiku calls, all rc 0, wall 3.1s vs sum 9.1s; 4 calls total $0.0119. Trivial prompts, max-turns 1, outside the repo.
- prompt cache: default TTL 5 min; 1h TTL exists; write 1.25x (5m) / 2x (1h); read 0.1x.
- several Claude Code sessions can run at once, each with its own model.
- cache shared across concurrent sessions: 확인 안 됨 (docs: identical prefixes in the same workspace can hit). Treat as per-session.
- A isolates notes/ per worktree -> any file both sides must see (design file, plan index, budget total) must live at an absolute path outside the worktrees.
- DECISIONS: main session is Haiku by default, Opus only for design turns; Opus/Sonnet mains rotate at 270k; agent rule "never delegate to another Opus agent".
- worktree from current master checks out a commit, not the uncommitted changes: 가정 (git behavior, not checked in this repo).

## option 1: two Opus designers, one worktree + task each, shared design file
- cost: 2 Opus sessions, each pays its own prefix write (system + tools + CLAUDE.md, size 가정 20-40k tokens) and its own growing context. Plus negotiation turns: every interface change = write file, other side re-reads, replies. Both designers idle while their executors run (minutes); idle > 5 min -> next turn rewrites the whole context at 1.25x. Two long-lived sessions = two contexts that expire and rewrite. 1h TTL (2x write) beats 5m only if a session wakes after a > 5 min gap at least 2 times per hour (2 x 1.25 = 2.5 > 2); whether Claude Code exposes the 1h choice: 가정 not exposed.
- conflicting design decisions: two authors of the same interface. Shared file has no lock and no tie-breaker; each defends its own task. Simultaneous edits to the file = last writer wins (no git merge, it is outside the worktrees).
- merge order: nobody owns it. Each designer prefers its own branch first; the second must rebase onto the first and may need to re-negotiate. Needs a rule or a third party (user) -> collapses toward option 2.
- context per session: lowest pressure (each holds ~1 task + shared file). Only real advantage.
- recovery if one stops: its task is orphaned mid-negotiation; the other may block on an interface answer. A new Opus must rebuild from the design file + the branch; agreements made in conversation but not written are lost.
- tasks at once: 2 (one per designer). More tasks = more Opus sessions, cost scales linearly in Opus.
- rule fit: conflicts with "never delegate to another Opus agent" and "Opus only for design turns"; would need the user to run two Opus terminals by hand.

## option 2: one Opus decides plans + merges; Sonnet/Haiku execute per worktree
- cost: 1 Opus. Can be stateless: opus-architect is invoked per decision point (plan task N, decide merge), reads plan files and diff summaries, writes a file, ends. Short sessions -> no long idle context to expire; each call pays prefix + the files it reads. Executors are cheap; their per-process overhead is small (trivial calls ~$0.0025 each; real tasks cost more, 가정).
- conflicting design decisions: one author for every interface. Executors do not design; if a spec is wrong they stop and report (escalate), not improvise.
- merge order: main decides by a fixed rule (below). One owner by construction.
- context per session: the risk. One designer seeing all N tasks grows context with N. Mitigation: plans in notes/runs/<date>-plan-<task>.md (absolute path, main repo), executors return <= 5 lines and write raw logs to files, main reads diffstat + test result, not full diffs. Stateless invocations keep each session far below the 270k rotation (가정 until measured).
- recovery: main stops -> executors finish their own worktrees; nothing merges until a new Opus call reads plan files + branch state (git is the state). Executor stops -> discard worktree, recreate from the same base commit, rerun once; second failure -> main diagnoses.
- tasks at once: bounded by subscription quota (shared by every window) and by main's review throughput, not by Opus count. Verified concurrency = 3 headless processes; start with max 3 executors.
- weak when: main is the design bottleneck (executors wait for plans), or tasks are design-heavy and independent so a second designer would work in parallel without negotiating.

## merge rule (owner: main Opus, option 2)
1. dependency order: the task that defines or changes a shared interface merges first.
2. then smallest diff first (fewer rebase conflicts for the rest).
3. before each merge: rebase the branch onto the current target, `go test -count=1 ./internal/harness` (and `-race`) must PASS in the worktree.
4. notes conflicts: LEDGER.md / DECISIONS.md resolved by main (append both, keep date order); metrics/*.jsonl via union merge (가정: order-independent, plan-task-isolation A).
5. rebase conflict in source -> back to that task's executor with the conflict list; main does not hand-edit.

## comparison
| item | 1: two Opus | 2: one Opus + executors |
|---|---|---|
| Opus sessions | 2 long-lived | 1, stateless per decision |
| cache expiry exposure | 2 contexts, idle waits | short calls, small |
| interface conflicts | possible, no tie-breaker | none (one author) |
| merge order owner | none (needs user) | main, fixed rule |
| context per session | low | grows with N, mitigated by files |
| recovery | orphaned task, lost agreements | state in files + branches |
| tasks at once | 2 per 2 Opus | ~3 (quota + review bound) |
| fits current rules | no | yes |

## verdict
- option 2. Reasons: merge order and interface decisions need one owner; option 1 still needs a tie-breaker at the decisive moment; two long-lived Opus contexts pay cache rewrites during every executor wait; option 2 matches DECISIONS (Haiku main, Opus for design turns only) and scales tasks without adding Opus.

## flip condition (to option 1)
- all three, measured on a real batch:
  (a) executors sit idle waiting for main's plans longer than they run (main is the bottleneck),
  (b) tasks touch disjoint files and the interface is fixed before start (<= 1 interface change per task after start),
  (c) one Opus invocation cannot hold the needed design context for one task cycle (hits the 270k rotation or needs a handoff mid-decision).
- even then: user as tie-breaker and merge-order owner, design file at an absolute path outside worktrees, single budget total.

## first check
- 2-task pilot under option 2, after the hook-path check in 261029-plan-task-isolation.md (where LEDGER/metrics land from a worktree).
- precondition: the current uncommitted master changes must be committed or branched first (user decision), else worktrees start from 90be2af.
- run: 2 small real tasks, 2 worktrees, 1 haiku-coder each, opus-architect called only for plan + merge.
- record per task in notes/runs/<date>-pilot-task-org.md: Opus calls and their total_cost_usd / token usage, executor wall time, executor wait-for-main time, rebase conflicts count, test result at merge.
- read: wait-for-main > executor time on both tasks -> condition (a) holds, check (b)/(c); otherwise option 2 stands.

## open (user only)
- committing current master changes so worktrees have a correct base (commit only when asked).
