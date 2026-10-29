# review: 261029-plan-parallel-load.md
date: 2026-10-29
verdict: run only after the changes below; as written, the verdict and the cap are not defensible.
scope: review only; plan not edited; claude not run.

1. (1) parallel check + load step: not enough as written
   - problem a: the prompt "Reply with the single word OK." is too short. Each call is mostly node startup + one short request, so wrapper intervals [start,end] overlap even if the server queues the requests. Overlap of process lifetimes proves 3 processes existed together. It does not prove the requests ran at the same time.
   - problem b: 2 rounds x 3 measures only concurrency 3. With no concurrency-1 reference there is nothing to compare, so it is a repeatability check, not a load measurement.
   - decision (minimum change):
     - add step 1a baseline: 2 solo calls (b1, b2), run one after the other, same command. b1 is also the auth check (see 4).
     - change the fixed prompt so the request takes real time: "Write the numbers 1 to 150 separated by single spaces. No other text." (about 450 output tokens, a few seconds on haiku; cost cents).
     - keep step 1 (3 parallel) and step 2 (2 rounds x 3). The load metric is then: median duration_api_ms at c=3 / median at c=1 (b1,b2). Report the ratio. Call it "load at c=3 vs c=1" only; do not claim a throughput curve.
     - total calls: 2 + 3 + 6 = 11.
   - parallel verdict rule (replaces line 31): all calls exit 0 AND is_error=false AND every pair of API intervals in the batch overlaps by >= 50% of the shorter one AND batch makespan (max t_result - min t_init) <= 0.6 x sum(duration_api_ms of the batch). Serialized: makespan >= 0.9 x sum. In between: report "부분 병렬 / 판정 보류" with the numbers.
   - optional for the load step: c=5 is not needed for the verdict and does not fit the fallback cap in (2). Leave it out.

2. (2) $2.50 ledger stop + $0.20 per call: does not hold $3 if the flag is ignored
   - problem: the ledger is checked only between batches. If --max-budget-usd is ignored under subscription login, nothing limits a single call. The plan's fallback "9 x 0.20 = $1.80" assumes the $0.20 cap holds, which is exactly the case being covered, so the argument is circular. The "cost field 0/absent" branch has the same flaw.
   - decision:
     - limit each call by its structure, not by the flag: add `--max-turns 1` and run without tools (`--tools ""` if `claude --help` or the docs list it; otherwise keep the no-tool prompt and record num_turns). One call = one model request.
     - per-call kill at 120 s wall clock (macOS has no `timeout`: `perl -e 'alarm shift; exec @ARGV' 120 env ... claude ...`).
     - exact fallback number: assumed worst case per in-flight call B = $0.60. Derivation (가정, Haiku 4.5 list prices, not in notes): 200k context x $2.00/M (1h cache write) = $0.40, plus 32k output x $5/M = $0.16, total $0.56, rounded up.
     - pre-launch rule (replaces the post-hoc 2.50 check): launch a batch of n only if ledger_sum + n x 0.60 <= 3.00. For n=3, ledger_sum must be <= $1.20. For n=1 (baselines), <= $2.40.
     - extra halt: any single call total_cost_usd > $0.20, or b1 cost > $0.10 (setup much heavier than assumed, e.g. MCP tool definitions loaded) -> stop and report.
     - total_cost_usd absent or 0 in b1 -> the ledger cannot work. Stop after step 1 (b1, b2 + 3 parallel = 5 calls; 5 x 0.60 = $3.00 worst case) and skip step 2.
     - keep `--max-budget-usd 0.20` as a secondary guard. If any call returns subtype error_max_budget_usd, record it as evidence that the flag is enforced.
     - expected spend: 11 calls x about $0.03-0.07 = about $0.3-0.8 (가정). The $1.20 gate should never be reached.

3. (3) fields to record (the verdict must not rest on start times alone)
   - decision: use `--output-format stream-json --verbose` and put one clock on each stdout line: `perl -MTime::HiRes=time -ne 'printf "%.3f %s", time, $_'`. Per call, record:
     - wrapper: t_launch (before exec), t_exit (after exit), pid (`$!`), exit code, stderr text.
     - stream timestamps: t_init (system/init line), t_first (first assistant line), t_result (result line). API interval = [t_init, t_result]. It excludes node startup and is used for overlap and makespan.
     - result fields: subtype, is_error, duration_ms, duration_api_ms, num_turns, session_id, total_cost_usd, usage.input_tokens, usage.output_tokens, usage.cache_creation_input_tokens, usage.cache_read_input_tokens, modelUsage keys (model ids).
     - init field: apiKeySource.
     - derived per batch: pairwise overlap (s) and overlap %, makespan, sum(duration_api_ms), peak concurrency from a sweep over the API intervals (not the wrapper intervals), ratio to median baseline.
   - field names (duration_api_ms, apiKeySource, init line shape) are 가정. Verify them on b1 output before step 1. If a name differs, map it and note the mapping in the result file. If there are no per-line timestamps from stream-json, fall back to the wrapper intervals and mark the verdict "추정".
   - distinct session_id and pid only prove separate sessions. They do not prove concurrency; do not use them as parallel evidence.
   - cache_read_input_tokens > 0 on later calls is evidence of a shared prompt cache. Record it; do not interpret cost differences.

4. (4) /tmp only + child env without ANTHROPIC_API_KEY: yes, with fixes
   - decision /tmp: yes. Use `D=$(mktemp -d /tmp/plc.XXXXXX)` instead of a fixed /tmp/plc (no stale files), one subdir per call, and the ledger at $D/ledger.txt. Never use the repo as cwd. Notes:
     - macOS /tmp is /private/tmp. Session files still go to ~/.claude/projects/-private-tmp-plc-*. Add `--no-session-persistence` if --help lists it; otherwise record this side effect.
     - /tmp does not stop user-level loading: ~/.claude/settings.json hooks, ~/.claude/CLAUDE.md and claude.ai MCP connectors still load and add time and tokens. Add `--strict-mcp-config` (no --mcp-config) to load no MCP. In step 0, record the hook count in ~/.claude/settings.json (count only). If there are hooks, add `--setting-sources project` if --help lists it (auth is not stored in settings). Do not use --bare (DECISIONS: fails on subscription login).
   - decision env: yes, drop it, and make it consistent and checked:
     - fix the inconsistency: step 0.4 drops API_KEY + AUTH_TOKEN, but the step 1 command drops API_KEY only. Use one launcher script $D/run.sh for every call: `env -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN ...`.
     - step 0 check ($0): `env -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN /usr/bin/env | grep -c -E '^ANTHROPIC_(API_KEY|AUTH_TOKEN)='` must print 0.
     - stop (do not unset silently) if ANTHROPIC_BASE_URL, CLAUDE_CODE_USE_BEDROCK or CLAUDE_CODE_USE_VERTEX is set: they change the provider or billing.
     - env -u does not cover apiKeyHelper. Step 0: `grep -c apiKeyHelper ~/.claude/settings.json` (count only). Nonzero -> stop.
     - auth check: b1 init apiKeySource must be "none" (가정 field). Any other value -> halt after b1, so at most 1 call is billed to the API. If `claude auth status` exists ($0), run it under the same env and record only the auth method.

5. other fixes
   - the step 1 command line still uses `date +%s.%N` (prints a literal N on macOS). Put the perl clock in the command itself so the executor does not copy the broken line.
   - stop conditions: add "exit 0 but is_error=true or subtype != success", and grep the rate-limit words in the result text too, not only in stderr.
   - order: step 0 -> b1 (auth + field check) -> b2 -> step 1 (3 parallel) -> gate -> step 2 round 1 -> gate -> round 2. Each gate = the pre-launch rule from (2) plus the stop conditions.
   - executor: haiku-runner is fine. Give it run.sh and the parser (one awk/python -I script in $D, not in the repo) so it does not improvise the overlap math.
   - result file: add columns t_init, t_result, duration_api_ms, cache_read_input_tokens, apiKeySource. Report the c=3/c=1 ratio and which pre-launch gate values were checked.
