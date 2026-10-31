# review: 261031-plan-router-tier.md
date: 2026-10-31
verdict: needs-revision (options only; no threshold basis, no priority order, no rollback, no steps)
scope: improvements only; plan not executed

## 1) context threshold basis (plan §3, §4-A: "N 토큰" undefined)
- add basis: FACTS.md:10 Haiku 5.5 price tier at 100k. vs Sonnet 5.5 (FACTS.md:9): <=100k Haiku ~20x cheaper (0.10 vs 2 input, 0.50 vs 10 output); >100k only ~4x (0.50 vs 2, 2.50 vs 10). -> proposed cut: estimated working context <=100k tokens = "short".
- add cap note: session rotation 150k (DECISIONS.md:46) -> threshold must stay < 150k; CLAUDE.md:28 still says 80k/270k (stale), fix separately, not in this plan.
- unit: tokens only (drop "파일 1개 이하" as primary; file count = secondary signal). estimate = prompt chars/4 + bytes of explicitly named paths/4, computed in code (classifier step), not by model judgment.
- evidence step before fixing N: run scripts/usage_report.py over notes/metrics/2026-10-29..31.jsonl; report per-model tokens bucketed by input ctx (<=50k, 50-100k, >100k). user observation ("짧은 컨텍스트에서 사용량 적음") is unmeasured (plan §5); N stays provisional until this table exists.

## 2) classifier (Sonnet) role (plan §4-C)
- recommend: keep classifier = Sonnet (DECISIONS.md:32, :38; user principle). replace option C with option D:
  - D: deterministic pre-filter in code before the classifier call (file per DECISIONS.md:32: internal/harness/classifier.go — existence unverified, check first). if est. ctx <=N AND no review/design keyword -> skip classifier, route haiku. else -> Sonnet classifier as today.
  - gain: saves the Sonnet call exactly on short prompts (the target case) without changing "classifier is Sonnet" when it runs.
- keep C as fallback only if D's skip rate is low (<30% of prompts in metrics); C needs explicit user approval.
- classifier error/timeout -> current behavior (no silent haiku default for long ctx).

## 3) conflict with task-type rules (plan §3 "Anything needing judgment")
- add explicit precedence (top wins):
  1. forced --model sonnet|opus (DECISIONS.md:38) — unchanged
  2. pre-release/merge/send review -> sonnet-reviewer, dispatched by main thread (LESSONS.md:5) — never downgraded by length
  3. design / multi-file code / synthesis of many sources -> sonnet-analyst/opus regardless of length
  4. short ctx (<=N) + lookup / mechanical / single-file short judgment -> haiku-* (extends DECISIONS.md:35)
  5. else -> current routing
- option B wording: "짧은 판단" must be bounded objectively: single file or <=N tokens AND answer <=~20 lines AND no code edit across files. otherwise rule 3.
- escalation kept: haiku fails twice -> sonnet-analyst (existing). count these as a quality metric.
- apply scope (plan §5 last q): recommend chaude conductor path first (code-enforced, measurable); general CLAUDE.md Routing text only after metrics pass.

## 4) rollback (missing in plan)
- flag: --no-short-haiku (default off = new behavior on), or env CHAUDE_SHORT_HAIKU=0; same pattern as --no-category-rotate / --idle-rotate 0=off (DECISIONS.md:21, :29).
- CLAUDE.md Routing edit (if any) in one separate commit -> `git revert <sha>`.
- record decision line in notes/DECISIONS.md with date + flag name.
- rollback trigger (check after 1 week of notes/metrics): haiku->sonnet escalation rate >15% of short-routed tasks, OR sonnet-reviewer reject rate up vs baseline, OR total cost not down.

## 5) missing plan sections (add before implementation)
- ordered steps per file: (a) metrics bucket table, (b) classifier.go pre-filter + flag, (c) unit test: short prompt skips classifier, review keyword still calls it, forced model wins, (d) DECISIONS.md entry, (e) optional CLAUDE.md Routing text.
- acceptance: sonnet token share down vs 10-29..31 baseline; no change in reviewer dispatch count for release/merge tasks.
- must NOT change: sonnet-reviewer pre-release rule, forced --model, v0.8.2 sonnet/opus IO block (DECISIONS.md:43), rotation 150k.

## open_q (user only)
- N 초기값 100k 토큰 기준에 동의하시는지 (metrics 확인 전 잠정값)
- 짧은 입력에서 classifier(Sonnet) 호출 생략(D안)을 "classifier는 항상 Sonnet" 원칙의 예외로 승인하시는지
