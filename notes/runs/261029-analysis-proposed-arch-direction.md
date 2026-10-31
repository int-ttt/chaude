# 261029 analysis: proposed arch as a multi-client direction (not main-session structure)
scope: 1 Opus designer; each Claude Code client keeps 1 always-active Sonnet (exec); Opus hands finished design to that client's Sonnet; Sonnet runs Sonnet/Haiku agents + long jobs. Main stays Haiku (DECISIONS 42); 43 (sonnet: no direct file reads) unchanged.
sources: 261029-analysis-proposed-arch.md, 261029-analysis-task-org-two-opus-vs-one.md, research/client-assumptions.md, research/prompt-cache-ttl.md, research/billing.md, internal/harness/clients.go + children.go + dispatch.go (signatures/comments only).
no source or DECISIONS.md edits.

## facts used (확인됨 unless marked)
- cache: 5m default (write 1.25x), 1h option (write 2x), read 0.1x, hit resets timer. Claude Code on subscription within plan: main conversation 1h, subagents 5m; overage drops main to 5m; verify via usage.cache_creation (ephemeral_1h / ephemeral_5m).
- exec Sonnet as a `claude -p` child = its own main conversation -> 1h: 가정 (doc does not say `-p`). Haiku agents it launches = subagents -> 5m.
- harness already: clients.json rows keyed (cwd, model) with Role design|exec, OwnerPID, ChildPID, LastAPI, LastCtx; idleDeadline = 3599s if ttl 1h else 290s; pickChild takes live warm child, else resumes a warm parked row not owned by another live harness, else starts new; reconcile parks/expires rows of dead owners; harness exit closes all children (DECISIONS 22).
- subscription: 5-hour + weekly limits exist; shared across concurrent sessions = 확인 안 됨 (treat as shared, conservative). Limit sizes not in notes.
- cross-client cache hit for identical prefixes: 확인 안 됨 (treat as per-client).

## (a) per-client always-active Sonnet, main question removed
- yes, cleanest owner shape. Without the 42 conflict it is simply a Role=exec child beside the Haiku main; registry already models it. One exec per client = one owner of execution per client, clients never share an exec (pickChild refuses rows of other live owners).
- alternatives worse: one shared exec Sonnet across clients = cross-client ownership + one context growing with every client's tasks; no exec (option 2) = Opus/main waits on jobs.
- keying caveat: childKey is (cwd, model). Under isolation A each worktree is its own cwd -> one Sonnet per worktree, not per client. Rule: exec Sonnet cwd = main repo, worktree paths passed absolute in the brief.
- "always active" must mean "warm while useful", not "process kept forever": state in a conversation dies at 270k rotation, client exit, or crash (earlier file).

## (b) idle cost per client: live/resumed vs on-demand from state file
- idle itself = 0 tokens (no requests). Cost appears only at the next wake. C = exec ctx, P = prefix (20-40k, 가정), L = ledger file (2-5k, 가정).
| wake after gap g | live/resume conversation | fresh start from state file |
|---|---|---|
| g < TTL | 0.1C | 2(P+L) at 1h / 1.25(P+L) at 5m |
| g > TTL, 1h | 2C | 2(P+L) |
| g > TTL, 5m (overage) | 1.25C | 1.25(P+L) |
- example C=100k, P+L=35k (가정): warm 10k; cold resume 200k (1h) / 125k (5m); file start 70k / 44k.
- rule: warm (g < idleDeadline) -> reuse the live child (always cheaper while 0.1C < 2(P+L), i.e. C < 700k: always). Past TTL -> never resume the conversation; start new from the file (cheaper whenever C > P+L, i.e. after the first task).
- keep-alive pings: at 1h each ping = 0.1C to avoid a 2C rewrite that a file start already caps at 2(P+L) -> not worth it. Reject.
- 1h vs 5m for exec: 1h wins when wakes come at 5-60 min gaps (typical: waiting for a job); 5m only when overage forces it. Harness ttl "auto" already adapts idleDeadline.

## (c) how many clients before the subscription limit
- open clients cost 0 while idle; the limit binds on concurrently ACTIVE work, not client count.
- per active client consumption = exec Sonnet turns + its Haiku/Sonnet agents + its share of Opus calls. Opus is the heaviest per token (ratio not checked, 가정 ~5x Sonnet).
- verified concurrency: 3 parallel headless calls, all rc 0 (trivial prompts). Real-task load at 3: 가정.
- recommendation: many clients open, max 3 active exec Sonnets at once (registry count of live Role=exec rows), Opus design calls serialized globally (1 Role=design row) -> also enforces "one Opus designer" across clients.
- limit numbers per plan unknown -> measure: /usage before/after a pilot hour with 1, 2, 3 active clients; raise cap only if 5-hour usage per active-hour x 5 < 80% of the window.

## (d) long-running agents: whose, and what happens on client close
- today: an in-process bg agent lives inside the exec Sonnet's claude process -> belongs to that client. Client exit closes all children -> agent dies (가정, consistent with DECISIONS 22 reason). Rotation keeps it via draining, exit does not.
- detached job (haiku-runner, nohup + log + exit-code file) survives the client (가정, likely). But then its result has no owner: registry rows of the dead owner become parked/expired; nobody reads the exit file.
- direction: long jobs (> 5 min) belong to the TASK, not the client. Always detached, recorded in the task ledger (pid, log path, exit file). In-process bg agents only for short work.
- on client close with open jobs: harness writes owner_pid=0 in the ledger (orphan) + notice in that client; any client's exec Sonnet (or the next start of the same client) adopts orphaned ledgers whose owner_pid is dead, reads exit files, continues. Adoption = 1 file start, no rerun.

## (e) the one change
- task ledger with ownership: notes/runs/<date>-exec-<task>.md at absolute main-repo path; fields: plan path, plan rev, owner_pid (client harness), steps [id, status, agent, output path, retries], jobs [pid, log path, exit file], escalations. Exec Sonnet writes it after every step. Harness: exec child is disposable (reuse while warm, else start from ledger); on owner death, reconcile marks the ledger orphaned and the next exec Sonnet adopts it.
- effect: exec state survives 270k rotation, idle expiry, crash, and client exit; idle cost capped at 2(P+L); jobs never lost with their client.
- secondary (not the one change): global cap 3 active exec rows + 1 design row in clients.json.

## flip condition
- drop the per-client exec Sonnet (back to option 2, Opus/Haiku direct) if a 3-task pilot shows median exec-Sonnet decisions per task (reorder, retry, diff reject, escalation) = 0 AND no job > 5 min.
- (multi-client side) if over the pilot no client ever exits with open jobs, adoption is unused: keep the ledger, skip the orphan/adopt code.

## first check
- pilot: 2 clients, 3 tasks (1 with a > 5 min detached job). Record in notes/runs/<date>-pilot-exec-multiclient.md: wake gaps per exec, cache_creation 1h vs 5m tokens per wake, file-start count, exec decisions per task, /usage delta per active hour.
- close client 1 mid-job: pass = ledger orphaned, client 2's exec adopts, job not rerun, result reported.

## open (user only)
- cap of 3 active exec clients acceptable until /usage measured?
- confirm 43 also binds the exec Sonnet (it verifies via diffstat + test output from Haiku, never reads files).
