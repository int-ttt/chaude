# review: 261031-plan-print-vs-repl-decision.md
reviewer: opus-architect, 261031
plan: notes/runs/261031-plan-print-vs-repl-decision.md

## recommendation (first)
추천: 제품 경로는 기존 상주 stream-json(-p) 자식을 유지하고, REPL(대화형 TTY) 자식은 채택하지 않으며 측정·비교용 실험 코드로만 남긴다.
status_change: "검토 전 (추천 보류)" -> "추천: -p 유지" (REPL 비용 미측정이지만 결론을 뒤집을 수 있는 경우가 좁음; 아래 reversal_conditions 참고)
consistent_with: notes/DECISIONS.md:47 (interactive TUI child against, resident stream-json keep)

## is the 2.13x gap structural
arithmetic: 166,481 - 78,290 = 88,191; 88,191 / 4 turns = 22,048 per turn ~= measured base-context diff ~22k (REPL ~41.5k vs -p ~19.5k)
conclusion: gap is ~100% explained by fixed per-request base context (system prompt/tools re-sent on every API call). structural, not noise.
implication_1: ratio is turn-count dependent (falls toward 1 as conversation history dominates); absolute gap grows linearly = +22k x API requests (tool-use rounds count as separate requests, so real sessions multiply it beyond user turns).
implication_2: plan should state the gap as "요청당 고정 +약 22k 토큰", not "2.13배"; 2.13x is valid only for this 4-turn run.
implication_3: extra base context is mostly cache_read on warm turns (0.1x input price) -> dollar gap is likely much smaller than 2.13x while cache is warm; it becomes cache_creation (1.25x/2x) after TTL expiry or first turn. so $1.4 estimate (0.654877 x 2.1) assumes proportional scaling and is probably an overestimate. treat as upper-bound guess, not a fact.
implication_4: +22k fixed context eats headroom under the Haiku 100k threshold (FACTS.md:10, 5x price above 100k) -> earlier crossing on long Haiku sessions.
is_it_a_reason_to_reject: alone, weak (cost delta may be small in dollars). combined with the reliability/signal items below, sufficient. token gap is a tiebreaker, not the main reason.

## key trade-offs
t1_cost: -p cheaper per request by fixed ~22k tokens; dollars measured only for -p (0.654877). REPL dollar delta unmeasured, likely < proportional estimate.
t2_reliability_and_policy: REPL needed (a) stripping 9 inherited env markers incl CLAUDE_CODE_CHILD_SESSION (policy fit unverified, plan:51), (b) trust dialog default "No, exit" -> Down+Enter keystroke hack, (c) turn-1 180s timeout FAIL on first attempt, (d) exit code not captured. -p needs none of these (core.go:143-154 resident stdin/stdout, no PTY).
t3_signals: -p stream-json gives total_cost_usd, usage, rate_limit_event, permission/notification events as structured JSON; REPL must scrape TTY output or session JSONL (which is disabled unless markers stripped). losing these breaks usagebar/ratestatus/turnspend metrics (DECISIONS.md:47 reason 2).
t_for_REPL (fairness): REPL matches what a human sees (same system prompt, interactive-only features, slash commands); only advantage relevant if a feature exists only in TUI. none identified in plan.

## unmeasured (keep listed in plan)
u1: REPL dollar cost and per-turn split input / cache_creation / cache_read / output (plan has totals only). source needed: REPL session JSONL usage fields.
u2: content of the extra ~22k (system prompt vs tool definitions vs output style/IDE context).
u3: confound: marker stripping applied only to REPL; -p run not repeated with identical env. effect on prompt size unknown.
u4: REPL 5h rate-limit data path (rate_limit_event or equivalent).
u5: behavior beyond 4 turns and after cache TTL expiry (when +22k becomes cache_creation every time).
u6: REPL exit code; policy boundary of env marker stripping.
u7: whether 5h usage-limit accounting weights cache_read tokens (affects whether +22k costs quota even when cheap in dollars).

## improvements to the plan
i1: replace "2.13배" headline with "요청당 고정 +약 22k (4턴 기준 2.13배)" and add the 88,191 = 4 x ~22k arithmetic.
i2: mark $1.4 as upper-bound estimate; if needed, price REPL from per-turn split x FACTS.md pricing instead of x2.1 scaling. agent: haiku-scout extracts usage split from notes/runs/261031-result-repl-vs-print-cost.md or REPL JSONL; no new run unless split absent.
i3: do NOT schedule further REPL runs by default; only run u3/u5 if user wants REPL kept as candidate. if run: haiku-runner, same env for both modes, >=10 turns plus one turn after TTL expiry.
i4: add reversal_conditions (below) so the decision is not re-litigated without new evidence.
i5: after user confirmation, append decision line to notes/DECISIONS.md (haiku-batch-worker): "[arch] 261031 print-vs-repl: keep resident stream-json (-p); REPL not adopted, experiment only. basis: +22k/request structural, env-marker strip + trust-dialog hack, loss of stream-json cost/rate signals. review: notes/runs/261031-plan-print-vs-repl-decision-review.md"
i6: resolve branch-name conflict (interactive-child-compare vs decision) explicitly; see user question.

## reversal_conditions
r1: a required feature exists only in the interactive TUI and cannot be reached via -p/stream-json.
r2: Anthropic policy/terms make -p resident use non-compliant while interactive use is allowed.
r3: measured REPL dollar cost (u1) and quota impact (u7) both within ~10% of -p over >=10 turns AND REPL runs without env-marker stripping or keystroke hacks.

## must NOT change
- internal/harness/core.go NewSession args (-p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model M)
- product code must not ship env-marker stripping for REPL children; measurement scripts only.

## question only the user can answer
q1: REPL 실험 코드를 브랜치(interactive-child-compare)에 비교용으로 남길지, 정리하고 -p 경로만 유지할지.
