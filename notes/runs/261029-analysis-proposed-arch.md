# 261029 analysis: proposed arch (1 Opus designer -> per-session always-active Sonnet -> Sonnet/Haiku/long-running agents)
scope: judge user proposal vs option 2 of 261029-analysis-task-org-two-opus-vs-one.md (one Opus decides plans + merges, Sonnet/Haiku executors per worktree, isolation A).
sources: that file, 261029-plan-task-isolation.md, research/billing.md, DECISIONS.md [rotate] 22/27/45, [routing] 39/42/43, internal/harness/children.go, clients.go, dispatch.go (signatures only), 261029-plan-multiproc-rebind.md §1.
no source or DECISIONS.md edits.

## facts used (확인됨)
- prompt cache: default TTL 5 min, 1h exists; write 1.25x (5m) / 2x (1h), read 0.1x. Idle process sends no requests -> idle itself costs 0; cost appears at the next wake (context re-write if TTL passed).
- harness already keeps live children per (cwd, model): childSet Park/Take, max 1 live child per model = 3 per harness, closed by 3599s expiry sweep (clients.go idleDeadline, plan-multiproc-rebind §1.4).
- Sonnet main rotates at 270k with overview <= 6,000 chars (DECISIONS 45). Rotation with running bg agents keeps the old child alive as draining (DECISIONS 22): agents live inside the parent claude process.
- DECISIONS 42: session main is Haiku-fixed, Opus only for design. DECISIONS 43: sonnet-analyst reads no files itself.
- subscription: no per-token billing within plan limits; total_cost_usd is an estimate (billing.md). Cost below = quota consumption, not money.
- note: plan-task-isolation verdict was "기본 D, 필요 시 A"; task-org analysis assumed A. This file follows the brief (A per task).

## what the proposal is
- = option 2 + one explicit execution owner per session. Opus stays the single design author (same as option 2). New: a Sonnet that owns execution (sequencing, agent choice, waiting, retry, verification, reporting) instead of main/Opus doing it.
- so the real question is only: is the Sonnet layer worth its cost, and is "always active" the right form.

## (a) cost
- idle Sonnet costs 0 while idle. Each wake after > 5 min = re-write of its whole context at 1.25x. Sonnet context grows with every brief + agent return in the session -> late wakes are the expensive ones (e.g. 150k ctx: re-write ~ 187k input-equivalent vs 15k if warm). sizes 가정.
- keep-alive ping every < 5 min costs 0.1x ctx each; vs one extra re-write (1.25 - 0.1 = 1.15x): break-even ~11 pings ~ 55 min of waiting. Keep-alive only pays if a wake is certain within ~55 min; otherwise let it expire. 1h TTL (2x write) beats 5m only with >= 2 wakes after > 5 min gaps per hour (same as earlier file); exposure of 1h choice to us 가정 (harness has ttl "auto").
- N sessions = N Sonnet prefixes (20-40k each, 가정) + N growing contexts. Option 2 has 0 standing executor contexts (subagents end at return).
- gain: Opus never waits. In option 2 the Opus/main side waits for executors and pays its own re-write on wake; in the proposal the waiting context is Sonnet-priced (가정 Sonnet ~ 1/5 Opus per token, price ratio not checked here).
- net: cheaper than option 2 only when tasks are multi-step or have long jobs (Opus waits avoided > Sonnet standing cost). For one-step tasks it is pure extra.

## (b) design decisions: where and who revises
- live in Opus's plan file notes/runs/<date>-plan-<topic>.md at absolute path in the main repo (outside worktrees, per earlier fact). Not in Sonnet's conversation.
- execution finds a problem -> Sonnet must NOT redesign. Sonnet writes escalation (plan step, failing check, evidence path, 1-line hypothesis) -> one stateless opus-architect call -> plan rev2 in the same file -> Sonnet resumes from the changed step.
- Sonnet may decide only what the plan leaves open: step order within dependencies, which agent, retries, accept/reject a Haiku diff against acceptance checks. "What must NOT change" list in the plan is hard.
- risk: a capable middle manager improvises a design fix to avoid the escalation hop -> silent design drift. Mitigation: rule "any change to a name/signature/data format in the plan = escalate".

## (c) Sonnet middle layer: hop or plan holder
- holds the plan's EXECUTION state (which steps done, running agent IDs, output paths, retry counts), not the design. That is real decision power (order, retry, reject, escalate) -> not an empty hop for multi-step / long tasks.
- empty hop when the task is 1 step (<= 1 file, 1 agent, no long job): Opus can hand to haiku-coder directly (current opus-architect rule). Rule: Sonnet layer only for >= 2 steps or any long-running job.
- conflict: verifying Haiku diffs needs reading files; DECISIONS 43 forbids sonnet-analyst direct reads. Either this Sonnet is a different role (session executor, reads diffstat + test output only) or it asks Haiku for summaries. Needs a user call if 43 is meant to cover it.
- conflict: DECISIONS 42 (session main = Haiku). If "session's Sonnet" = the session main, this changes 42. If it is a separate child next to the Haiku main, 42 stands.

## (d) long-running jobs
- start: haiku-runner, detached (nohup / background process writing log + exit-code file at a known path). Not as an in-process bg subagent only: agents die with their parent claude process (DECISIONS 22 reason) 가정 for detached shells surviving, likely but unchecked.
- who waits: Sonnet (cheap, and it holds the job list). Wait must be event-driven (harness completion notice / result-file poll, already designed in DECISIONS 22), not a model polling loop.
- who restarts: Sonnet decides one retry via haiku-runner; second failure -> escalate to Opus (diagnosis = design work).
- who reports: Sonnet -> main session <= 5 lines (outcome · paths · PASS/FAIL), details in notes/runs/. Opus never hears about jobs unless escalated.

## (e) context limits, 270k
- Opus: stateless per design call (plan / revise / merge decision) -> far below 270k (가정 until measured, same as earlier).
- Sonnet: the only growing context. Hits 270k on long sessions with many tasks. At 270k: rotation with handoff (overview <= 6,000 chars); running agents -> old child drains (DECISIONS 22).
- problem as stated: execution state lives in the conversation -> 6,000-char overview may drop step status / agent IDs -> new Sonnet re-runs or loses jobs. Fix = state in a file (see one change); then rotation loses nothing and costs only the new prefix.

## (f) failure: Sonnet stops
- as stated: nobody notices. Opus has ended (stateless), Haiku agents return to a dead parent, user sees silence. In-process bg agents die with it (가정 as above).
- who should notice: the harness, not a model. It already tracks children (childSet, clientRegistry reconcile, process exit). Needed: on Sonnet child exit / no event for X min while jobs open -> notice to main "[exec down] task · ledger path"; restart = new Sonnet reads the ledger file + branch state (git = source state), detached jobs keep running.

## (g) "always active" readings
- live process: a kept-open claude child per session (harness already supports, max 1 per model, 3599s idle close). Idle = 0 tokens; wake after TTL = re-write. Most likely meant. Fit: good.
- standing agent: a subagent that persists. Claude Code subagents end at return (resume across turns 가정, unverified per DECISIONS 21). Not available as "always active".
- schedule: cron/loop waking Sonnet to poll. Every tick = a turn reading full context (0.1x warm, 1.25x cold). Pure waste vs event-driven notices. Reject.
- meant: live process. Correct form: live process while the session has open work, closed when idle past deadline, re-created from file on demand. "Always available", not "always running".

## comparison
| item | option 2 (earlier) | proposal as stated | proposal + one change |
|---|---|---|---|
| design author | 1 Opus | 1 Opus | 1 Opus |
| execution owner | main/Opus | Sonnet (conversation) | Sonnet (ledger file) |
| Opus waits for jobs | yes | no | no |
| standing contexts | 0 executors | N Sonnet | N Sonnet, only while work open |
| 270k rotation loss | n/a (Opus stateless) | execution state at risk | none |
| Sonnet dies | n/a | nobody notices | harness notices, resume from file |
| 1-step task | direct | extra hop | direct (layer skipped) |

## verdict
- not the cleanest as stated: "always active" puts execution state and liveness inside a process, so 270k rotation and Sonnet death lose state and nobody notices.
- with the one change below it is the cleanest of the three: single design author (as option 2), Opus never waits, execution has one owner per session, failure is detected by code.

## one change
- Sonnet = file-backed and harness-supervised, not always running: execution state in notes/runs/<date>-exec-<task>.md (absolute path, main repo; fields: plan path, plan rev, steps [id, status, agent, output path, retries], open jobs [pid/agent id, log path, exit file], escalations). Sonnet writes it after every step; harness starts/resumes Sonnet on open work or job completion and posts "[exec down]" when the child exits with open steps. Layer skipped for 1-step tasks.

## flip condition (back to option 2, no Sonnet layer)
- measured on a pilot batch: median Sonnet-owned decisions per task (reorder, retry, diff reject, escalation) = 0 AND no task has a long-running job (> 5 min). Then Sonnet only relays and its standing context is pure cost -> option 2.

## first check (pilot, after 261029-plan-task-isolation hook-path check)
- 3 tasks: 1 one-step, 1 multi-step, 1 with a > 5 min job. Record per task in notes/runs/<date>-pilot-exec-layer.md: Opus calls, Sonnet decisions count, Sonnet ctx at end, Sonnet wake count after > 5 min gap, wait time, total_cost_usd per process (estimate).
- kill the Sonnet child once mid-job: pass = harness notice appears, new Sonnet resumes from ledger, job not rerun.

## open (user only)
- is the session's Sonnet the session main (changes DECISIONS 42 Haiku main) or a separate child beside the Haiku main?
- does DECISIONS 43 (no direct file reads) apply to this executor Sonnet?
