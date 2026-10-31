# review: 261031-plan-io-design-refine.md
reviewer: opus-architect, 2026-10-31. improvements only; plan not executed.
sources: plan:1-73, DECISIONS.md:40-47, LESSONS.md:3, repl.go:27-28,1068, usagebar.go:13, tui.go:240

## verdict
- plan direction (keep 0.8.x, partial reinforcement, no input compression) OK.
- rotation section is not executable as written: conflicts with newest decision + code; no implementation step; risks under-estimated.

## C. DECISIONS conflict: YES
- DECISIONS.md:41 (2026-10-29) = Haiku 80k / Opus+Sonnet 270k.
- DECISIONS.md:46 (2026-10-30, newer) = 150k unified for Haiku/Sonnet/Opus mains, explicitly "replaces 2026-10-29 80k/270k". user: "세션 컨텍스트 상한을 150K로 확정".
- code follows :46: repl.go:28 opusSonnetThreshold = 150_000; repl.go:1068 -threshold default 150_000; usagebar.go:13 ctxBarMax = 150_000.
- plan error: plan:3 and risk #4 label 150k as "구/옛값". chronology reversed: 80k/270k is the older value. a plan cannot replace a user decision.
- fix: plan:3, plan:35-36, risk #4 -> "80k/270k = proposed revert of DECISIONS.md:46; pending user decision". add step 0 (below). until decided, effective value = 150k.
- also :40 says Opus/Sonnet mains should not switch by threshold at all; plan should state which of :40/:41/:46 it intends to supersede.

## top 3 improvements
1. step 0 = user decision on threshold (150k per :46 vs 80k/270k). block steps 4 and 6 on it. ask with cost numbers (see R7) not just preference.
2. add an implementation step for rotation (currently only "reflect in DECISIONS"): exact touch points listed in E below. without it, plan step 4 "80k 적용 검증" verifies something that does not exist in code.
3. re-estimate risks conservatively (R3 raise, add R7-R10) and reorder measurement: passive/zero-cost measurements first, caveman B/C with n>=3 and pre-registered rubric.

## R. risk re-estimate (conservative)
- R3 (Haiku tier): rotation check runs after an answer (repl.go:1068 "rotate after an answer"), so context overshoots by one full turn. 80k leaves only 20k margin to the 100k tier (FACTS.md:10); one turn with 3-4 scout returns (4000 chars each) + file reads can exceed it. under current 150k (:46) Haiku spends 80k-150k past the tier every long session -> L: 높음 (확실 if FACTS.md:10 is correct), I: 높음. 
- R3b: verify FACTS.md:10 (Haiku 5x above 100k) against official pricing source (haiku-researcher) before using it as a basis; whole Haiku threshold argument rests on it.
- R7 (new) cache-miss cost scales with threshold: any cache miss (1h TTL expiry, idle, system prompt/CLAUDE.md change, model switch) rewrites full context at 1h write $8/MTok -> Opus 270k ~= $2.16 per miss, 150k ~= $1.20 (vs whole A-run $0.54). L: 중, I: 높음. mitigation exists (idle rotate 3599s, commit 5e415b5) but must be re-checked for 270k.
- R8 (new) per-turn cache read scales with context: higher threshold = more read cost per call; lower = more rebuild writes. threshold should come from break-even, not fixed: rotate when (ctx - base) * read_price * expected_remaining_calls > base * write_price. needs base prompt size (measure M7).
- R9 (new) 270k unreachable/conflicting: settings autoCompactWindow 150000 (:46, unchanged) may auto-compact before 270k rotation fires; ctxBarMax 150_000 bar overflows; Opus long-context premium above 200k (if applicable to model) must be checked in FACTS/billing.md. L: 높음 (config conflict certain), I: 중.
- R10 (new) more frequent rotation (Haiku 80k) = more handoffs: LESSONS.md:3 "rotate must not handoff/overwrite a pending import snapshot" exposure rises; handoff overview cap 3,000 chars (Haiku) loses task state. L: 중, I: 중.
- R1/R5: keep, but note single A run cannot estimate variance at all; all caveman savings ($0.048, 9%) are within unknown noise.

## M. measurement order (revised)
1. M2, M5, M6-latency first: passive, $0, derive from existing notes/metrics/*.jsonl (rotate events carry rotate_at_tokens; metricscmd.go:73). no new runs.
2. M7 (new): per-turn context delta distribution (p50/p95/max) per model; overshoot past threshold at rotation; base prompt size after rotation; first-call cache write after rotation. feeds R3/R8 and threshold choice.
3. M1 caveman B/C: n>=3 per condition (A included), explicit budget set by user (prior stop at $1 cap), rubric for 근거·조건부 누락 written and saved before running.
4. M3, M4 after M1 (only relevant if caveman adopted / read cap questioned).
5. M6 billing (5h usage vs API price ratio, billing.md:18): separate, low priority.

## E. execution steps (revised)
0. ask user: threshold 150k (:46) or 80k/270k; Opus threshold rotate or not (:40). show R7/R8 numbers.
1. passive measurements M2/M5/M6/M7 (haiku-scout or haiku-runner over metrics jsonl). 
2. caveman M1 with user budget.
3. if threshold changes, haiku-coder spec, touch points: repl.go:28 opusSonnetThreshold; repl.go:1068 -threshold default; usagebar.go:13 ctxBarMax -> per-model max; tui.go:240 display; settings.json autoCompactWindow (must be > Opus threshold or rotation never fires); child rotation for haiku-coder in livechild.go (scout first: plan:37 child 80k has no code match); CLAUDE.md:28 stale text; DECISIONS.md new entry superseding :46.
4. rotation logic improvements (independent of value): pre-emptive check ctx + p95_turn_delta >= threshold (Haiku hard ceiling 100k); hysteresis to avoid back-to-back rotation; never rotate while import snapshot pending (LESSONS.md:3); log overshoot in rotate event.
5. tests: drain_test/usagebar_test style unit tests for per-model threshold, pre-emptive check, pending-import guard; haiku-checker runs go test ./internal/harness/ -run 'Rotate|Usage' .
6. re-evaluate R1-R10 table with measured values; update plan status.

## must NOT change (keep from plan)
- input: raw passthrough, no compression hop.
- read cap 4000 chars default.
- resident stream-json child (no interactive TUI child; consistent with DECISIONS.md:47).
- caveman applied once at final answer only, E-ID/confidence tags kept.
