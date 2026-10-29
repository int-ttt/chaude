# plan: parallel claude sessions check + load test
date: 2026-10-29
status: plan only (no claude command run, no source edited)
cap: total $3 hard; stop launching at ledger sum >= $2.50; per call --max-budget-usd 0.20
calls: 3 (parallel check) + 6 (load) = 9 max

## facts used (from notes)
- several Claude Code sessions can run at once: 확인됨 (code.claude.com model-config)
- `-p --output-format json` payload has `total_cost_usd`: 확인됨 (headless docs)
- `--max-budget-usd` is print-mode only; docs say "spend can pass the cap, leave headroom": 확인됨
- total_cost_usd is a client-side estimate, not billing: 확인됨
- shared prompt cache between concurrent sessions: 확인 안 됨
- budget flag behavior under subscription login: 확인 안 됨 (third-party report only) -> 가정
- local `claude --help` showing the flag: 확인 안 됨 -> check in step 0
- json field names for duration (e.g. duration_ms), exit/result subtype: 가정, verify on first output

## step 0 pre-check (no model call, $0)
1. Test shell: for each of ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL, CLAUDE_CODE_USE_BEDROCK, CLAUDE_CODE_USE_VERTEX: print name + set/unset only (`[ -n "${VAR+x}" ]`), never values.
2. `claude --version` must exit 0. Record version (budget cap enforcement needs v2.1.217+ per docs).
3. `claude --help | grep -i budget` : record whether flag listed (docs say help may omit flags; absence is not a blocker).
4. Child env for all calls: `env -u ANTHROPIC_API_KEY -u ANTHROPIC_AUTH_TOKEN claude ...` (so the run uses subscription login, not API billing).
Stop if: version fails, or key var cannot be removed from child env.

## step 1 parallel check (3 calls)
- Setup: `mkdir -p /tmp/plc/run1/c{1,2,3}` ; cwd per call = own dir (not the repo). Logs in /tmp/plc/run1/.
- Prompt (one line, fixed): "Reply with the single word OK."
- Launch all 3 in background within the same second:
  `(cd /tmp/plc/run1/cN && date +%s.%N > start; env -u ANTHROPIC_API_KEY claude -p "Reply with the single word OK." --model haiku --output-format json --max-budget-usd 0.20 > out.json 2> err.txt; echo $? > exit; date +%s.%N > end) &` then `wait`.
  (macOS date lacks %N: use `python3 -c 'import time;print(time.time())'` or `perl -MTime::HiRes=time -e 'print time'`.)
- Record per call: start, end, exit code, total_cost_usd, duration, stderr text.
- Parallel verdict rule: all 3 exit 0 AND time intervals overlap (max(start) < min(end)). Serialized if intervals are back-to-back (each start ~ previous end) or wall time ~ sum of durations. Fail if any non-zero exit.
- Ledger check after step 1: if sum >= $2.50 or any stop condition, halt (expected sum well under $0.30; 가정).

## step 2 load (2 rounds x 3 = 6 calls)
- Same command, same fixed prompt, dirs /tmp/plc/run2/r{1,2}c{1,2,3}.
- Round 1: launch 3, wait. Check ledger + stop conditions. Round 2: launch 3 only if ledger < $2.50 and no stop condition.
- Per call collect: total_cost_usd, duration (json duration field if present; else wall end-start), exit code.
- Max concurrency observed: sweep over all start/end timestamps (+1 at start, -1 at end), take the peak. Expected 3; <3 means serialization/throttling.
- Cache note: identical prompts may share cache across calls (확인 안 됨); cost variance between calls is not evidence either way.

## step 3 budget ledger (hard)
- Ledger file /tmp/plc/ledger.txt: one `callid cost` line per finished call; sum with awk after every batch.
- Rules: per-call `--max-budget-usd 0.20`; no new launch when sum >= 2.50; worst case in flight at stop = 3 calls x (0.20 + overshoot). Cap can be exceeded per call (docs), so 2.50 threshold leaves $0.50 headroom for 3 in-flight calls (overshoot assumed < $0.30 each... 가정).
- Enforcement of `--max-budget-usd` on subscription login: 가정 until checked. Check: after step 1, if any call cost > 0.20 or json shows subtype error_max_budget_usd, record it; if cost field is 0/absent under subscription, the ledger cannot enforce -> fall back to call-count cap (9 calls x 0.20 = $1.80 worst case, within $3) and report.
- total_cost_usd is a client estimate; real subscription usage is not measured (note in verdict).

## step 4 output
File: /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261029-result-parallel-load.md
- Table: call id | start | end | cost (total_cost_usd) | exit code | parallel with which
- Sum of costs, max concurrency, wall time per round, version, env var status (set/unset only).
- Verdict: parallel works / not, with reason (overlap + all exit 0 + concurrency peak). State separately: evidence vs inference; unverified items (cache sharing, budget enforcement under subscription).

## stop conditions (halt, no further launches, write result with partial table)
- any non-zero exit code
- any rate-limit text in stderr/json (e.g. "rate limit", "429", "usage limit", "overloaded")
- ledger sum >= $2.50 (or the cap logic above reached)
- any step-0 failure

## blockers / open
- none known for planning. Needs main thread to run (this agent must not run claude); suggest haiku-runner executes steps 0-4.
- open: duration/field names in json unverified; subscription behavior of the budget flag unverified.
