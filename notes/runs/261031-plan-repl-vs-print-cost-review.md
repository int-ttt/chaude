# review: 261031-plan-repl-vs-print-cost.md

reviewer: opus-architect
date: 2026-10-31
verdict: NOT executable as written. Missing: how to drive the REPL, auth/billing bucket, cache isolation, 100k definition + enforcement, single measurement source, subagent scope.
scope: improvements only; plan not executed.

## P1 (blocking)

1. driving_A: plan gives haiku-runner no way to run an interactive REPL (Bash has no TTY).
   - fix: run A inside tmux: `tmux new-session -d -s costA 'claude --model claude-opus-5-5'`; send prompt via `tmux send-keys -t costA '<prompt>' Enter`; detect end by polling transcript for final assistant message with stop_reason end_turn; then `send-keys '/exit' Enter`.
   - B: `claude -p --model claude-opus-5-5 --output-format stream-json --verbose '<prompt>'`.
   - both: pin full model ID (not alias), same effort setting, same cwd.

2. auth_bucket: billing.md:10 "In non-interactive mode (-p), the key is always used when present." If ANTHROPIC_API_KEY is set in the env, B bills at API rates and A uses subscription -> not comparable. TTL also differs (prompt-cache-ttl-clear.md:25: 1h on subscription within plan, 5m on API key/credits).
   - fix: pre-run step: `env | grep -c ANTHROPIC_API_KEY` must be 0 for both, record auth mode in run log (`auth: subscription|api_key`).
   - record cache TTL actually granted per run from usage.cache_creation.ephemeral_5m_input_tokens vs ephemeral_1h_input_tokens.

3. cache_isolation (parallel distorts cache): prompt cache is shared per account by exact prefix (tools + system + CLAUDE.md). Parallel start -> both write the same prefix (double cache_creation) or the later one reads the earlier one's cache (cache_read on turn 1) -> order-dependent, not a mode difference.
   - fix (chosen): do NOT run the measured pair in parallel. Design:
     a. warm-up: one trivial prompt in each mode first (e.g. "ok라고만 답해"), discard its numbers -> both measured runs start warm.
     b. measured runs sequential, order alternated (A,B then B,A), N>=3 pairs.
     c. gap between runs < TTL (so warm state holds); record seconds between runs.
   - keep ONE separate parallel pair only to answer item 6 (duplicate billing): compare sum of turn-1 cache_creation in parallel pair vs sequential pair.
   - alt considered: cold-vs-cold (wait > TTL, up to 1h on subscription) -> rejected: slow and 1h wait makes N>=3 impractical.

4. limit_100k undefined: plan does not say what 100k means or who enforces it.
   - define two values, measured per API request from usage:
     - context_size = input_tokens + cache_read_input_tokens + cache_creation_input_tokens of the latest request.
     - cumulative = sum of (input + cache_read + cache_creation + output) over all requests.
   - "100k 한도" = context_size <= 100000 (per-request). Ask user if cumulative was meant (see question).
   - enforcement: neither mode has a hard 100k stop by flag (verify; settings autoCompactWindow is 150000 per DECISIONS.md:46 and applies to interactive). Runner enforces: poll transcript every 5s, if context_size > 100000 -> kill session (A: tmux kill-session; B: kill pid), record `limit_hit: yes, at_request: N`.
   - do not change settings.json to enforce (plan scope = no changes); if a temporary setting is needed, use a separate fixture dir.
   - check FACTS.md:6-12 Opus row: if Opus has a >100k price tier like Haiku (FACTS.md:10), per-request context_size decides tier -> report it per request.

5. measurement_source: plan lists metrics but no source. A and B must be parsed by the SAME parser from the SAME source.
   - source: session transcript JSONL under ~/.claude/projects/<cwd-slug>/<session-id>.jsonl (B also writes one unless --no-session-persistence; do not pass that flag). Subagent transcripts: same dir, subagents/ subfolder -> include.
   - dedupe assistant lines by message.id (one API response can span multiple lines with repeated usage) -> count usage once per message.id.
   - B cross-check only: stream-json final `result` event (usage, total_cost_usd, num_turns, duration_ms). total_cost_usd is a client estimate (billing.md:11) -> label 추정.

6. subagent_scope: project CLAUDE.md forces delegation, so an Opus main spawns Haiku/Sonnet agents; hooks (.claude/hooks/*) inject context in both modes. "Both Opus" is then false for total cost.
   - fix: report per-model breakdown (Opus main / Haiku / Sonnet). Decide one of:
     a. run in project dir as-is (realistic, includes hooks+subagents), or
     b. run in a clean fixture dir (/tmp/costcmp-<date>/ with a copy of the task inputs only) -> measures mode difference only.
   - recommended: b for the mode comparison; a optional as second experiment. Ask user (see question).

## P2 (correctness of comparison)

7. same_start_state: list in run log: git rev (`git rev-parse HEAD`), dirty file count, cwd, CLAUDE.md sha1, settings.json sha1, claude --version, model ID, permission mode. Must be identical A vs B.
8. permissions: -p cannot answer permission prompts (tool silently denied) while REPL blocks waiting -> different behavior. fix: same `--permission-mode` and same `--allowedTools` list on both; prefer a read-only task prompt (plan scope is analysis only, and parallel edits on one tree would conflict).
9. equal_work: cheaper run may have just done less. Add acceptance criterion for the task output (e.g. same answer file sections present) and record `task_ok: yes|no` per run; compare cost only for runs with task_ok=yes.
10. hidden_calls: REPL may make side calls (title/summary, Haiku helpers) that -p does not. Count them as separate requests by model; do not drop them.
11. single_vs_multi_turn: one prompt = one turn hides the real difference (REPL keeps context; -p per turn reloads via --resume and pays cache_read/creation again). Ask user; if multi-turn, B uses `claude -p --resume <id>` per follow-up, same follow-up texts as A.

## P3 (metric list -> can it answer "cost of each")

current 8 items cannot produce a cost per run. Replace section 3 with:
- per run, per model: requests, input_tokens, output_tokens, cache_read, cache_creation_5m, cache_creation_1h, peak context_size, cumulative tokens, compactions (count of compact events in transcript), duration_s, retries/errors (API error lines), task_ok.
- cost_usd_equiv (추정) = sum over models of tokens x FACTS.md:6 price columns (input / 5m write / 1h write / cache read / output); cite FACTS.md line + date.
- subscription view: 5h usage % delta via usagepoll (`claude -p /usage`) before/after. Valid only for sequential runs with nothing else running (aggregate, not attributable in parallel). Note: the /usage poll itself is a -p call; take it outside the measured window.
- item 5 (세션 간 컨텍스트 재사용) -> redefine: turn-1 cache_read_input_tokens > 0 in the measured run (after warm-up this is expected; report the value).
- item 6 (병렬 중복 과금) -> redefine: parallel pair turn-1 cache_creation sum minus sequential pair turn-1 cache_creation sum.
- item 7 (100k 도달) -> peak context_size and limit_hit from P1.4.
- final table: rows = metric, columns = A mean, B mean, B-A, B/A, N; each cell tagged 측정 or 추정.

## P4 (process)

12. approval: a subagent cannot receive user approval mid-run. Split into two dispatches: (1) haiku-runner preflight only -> writes notes/runs/261031-cost-preflight.md (commands, env check, estimate); main asks user; (2) after user approval, new haiku-runner dispatch executes.
13. estimate method: base on warm-up run usage (measured) x number of runs x 1.5 margin; state it as 추정.
14. log format: run-A/B files as key: value per run (run_id, mode, order, start/end ISO time, auth, model, fields from P3), plus raw usage per request as a table. Add notes/runs/261031-cost-summary.md for the final table.
15. parser: put the transcript parser in a throwaway script under /tmp (not the repo, plan scope = no repo changes); record its path and sha1 in the summary.

## open questions (user only)
- Q1: 100k = 요청 1회의 컨텍스트 크기 상한인가요, 세션 전체 누적 토큰 상한인가요?
- Q2: 프로젝트 환경 그대로(훅·서브에이전트 포함) 비교할까요, 깨끗한 임시 폴더에서 모드 차이만 비교할까요?
- Q3: 단일 프롬프트 1턴 비교인가요, 여러 턴 대화 비교인가요?
- Q4: 병렬 실행은 중복 과금 확인용 1쌍만 두고, 본 측정은 순차 실행으로 바꿔도 될까요?

## must NOT change
- no edits to repo code, settings.json, CLAUDE.md, hooks during the experiment.
- original plan file stays as is; this review is advisory.
