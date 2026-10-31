# review: 261029-plan-router-3tier.md
reviewer: opus-architect (design review, improvements only; plan/policy files not edited)
verdict: plan sound in structure (1 file, 6 units, size derived for metrics); 3 corrections required before implementation: sonnet tier = direct delegation (not plan+review), tests are NOT "args only", tag definitions dropped 3 old large items.

## decisions

1. sonnet tier behavior -> (a) direct: Haiku delegates to sonnet-analyst in one foreground round, no Haiku plan, no review step. Reject plan's proposal (b).
   - cost basis (261029-analysis-per-prompt-classify.md): classify call itself is a few % of main cost; real cost is in what the route triggers. Neither option switches the main session (main stays Haiku, pickModel unchanged), so the 8.5만 transition cost does not apply; the difference is hop count only.
   - (b) = Haiku plan write (Haiku main context grows toward 80k rotation) + sonnet-analyst review (reads plan + same files) + Haiku merge + Haiku agents execute + sonnet-reviewer = >=2 Sonnet calls + 3 Haiku steps. (a) = 1 sonnet-analyst call; it already hands mechanical sub-steps to Haiku (CLAUDE.md:11) and does not read files itself (DECISIONS:39).
   - for analysis/review/comparison tasks the plan IS the deliverable: (b) does the work twice (Haiku draft, Sonnet critique). planHint's review-only design exists to keep Opus (expensive) from executing; Sonnet as executor costs the same class as Sonnet as reviewer, so the restriction buys nothing.
   - safety: analysis says under-classification (design work routed low) is the expensive direction. (a) keeps sonnet-analyst's scope-first ESCALATE (CLAUDE.md:9) -> mis-tiered opus work gets escalated. In (b) review mode sonnet-analyst only writes improvements and never escalates -> no safety net.
   - sonnetHint wording (new const, classifier.go): "[routing: sonnet tier. Do not write a plan. In ONE foreground round delegate to sonnet-analyst: user's words quoted + target paths + 'reply: Korean 존댓말'. If it returns ESCALATE: <path>, give that path to opus-architect as is (no Haiku plan step). sonnet-reviewer only before release/merge/submission.]\n"
   - TestSonnetTierHint: contains "sonnet-analyst", "ESCALATE", "opus-architect"; does NOT contain "plan-<topic>" or "write the full design"; sonnet tier task has sonnetHint prefix and not planHint; haiku tier has neither.

2. opus tier -> keep existing plan->review->merge workflow (planHint), not opus-architect direct.
   - reason: it is the user's standing decision (DECISIONS:38, CLAUDE.md:32 "Opus is for design only ... Opus/Sonnet do not execute"); changing it is outside this plan's 비목표 and needs a separate user decision. Cost: Opus reads Haiku's plan + needed files and outputs improvements only (short Opus output, the 5x-priced part) instead of a full plan.
   - refinement A: opus tier review = opus-architect only. Drop "and/or sonnet-analyst" from planHint step (2): sonnet-level work now has its own tier, so a second reviewer is double cost. TestHaikuPlanWorkflow :136 list then checks "opus-architect", "notes/runs/", "Haiku agents" (remove "sonnet-analyst" from that list, or keep it only if planHint keeps the "and/or").
   - refinement B: rename planHint prefix "[routing: large or judgment task." -> "[routing: opus tier (design/judgment)." so CLAUDE.md hint name, metrics `tier`, and hint text agree.
   - exception (no new code): an ESCALATE path from sonnet-analyst already is a scope file -> goes to opus-architect directly (per CLAUDE.md:9), skipping the Haiku plan step. Covered by sonnetHint text above.

3. missing/invalid tag default -> confirm sonnet. Also confirm "unsure -> sonnet" for both haiku/sonnet and sonnet/opus boundaries.
   - evidence: production classifier is Sonnet (conductorModel); 30/30 correct, 0 parse failures (261029-test-router-30-prod-args.md). Haiku-model errors were all small->large (over-escalation), 0 large->small (261029-test-effort-haiku-vs-sonnet.md:53). Missing tag is rare and observed bias is upward, so defaulting to opus would pay an Opus review on every parse failure for little gain.
   - the expensive under-classification case is covered by decision 1's ESCALATE path; this is the condition that makes sonnet default safe. If decision 1 were (b), default should be opus instead.
   - keep the explicit opus trigger list in classifyPrompt (design/architecture, trade-off/recommendation, strategy/priority/roadmap, unknown-cause diagnosis, repeated failure, 'opus로') so "unsure" stays narrow.
   - missing from plan's new definitions (old large list items with no new home) -> add:
     - version bump with exact text, identical-wording replacement -> haiku (matches CLAUDE.md:9 haiku-coder rule; resolves P3 label conflict).
     - release, deploy, merge -> sonnet (needs sonnet-reviewer; not design).
     - new feature or refactor -> sonnet when scope is given; opus when the design is undecided.
     - classifyPrompt :22 "A lookup or edit that is small is sent straight to a Haiku session" -> "...that is tier haiku...".
   - parse: if both `tier:` and old `size:` present, tier wins; size-only -> small=haiku, large=sonnet, SizeDefaulted=true (as plan says).

4. policy -> append a new line to notes/DECISIONS.md; do not edit line 38. Confirmed: header says "One line each. Newest at bottom." and existing entries supersede by new lines (e.g. [conductor] "Supersedes conductor-session rule").
   - suggested line: "- [routing] 261029 user: <user's words verbatim> -> classifier tags tier haiku|sonnet|opus (size kept as derived metric: haiku=small, sonnet|opus=large). haiku: Haiku agents direct; sonnet: Haiku delegates directly to sonnet-analyst (no plan; ESCALATE -> opus-architect); opus: planHint workflow (Haiku plan -> opus-architect review -> merge -> Haiku agents). Missing/invalid tier -> sonnet. Supersedes the size part of the [routing] 261029 Haiku-fixed entry. Tests: TestParseRouteSize, TestHaikuPlanWorkflow, TestSonnetTierHint."
   - CLAUDE.md:32 phrases to change (all on line 32):
     a. "the classifier is always Sonnet and only tags size" -> "the classifier is always Sonnet and only tags tier (haiku|sonnet|opus)"
     b. "Large/judgment/design task (arrives with a `[routing: large ...]` hint) → workflow:" -> "Opus-tier task (design/architecture/trade-off/strategy/unknown-cause diagnosis/repeated failure; arrives with a `[routing: opus tier ...]` hint) → workflow:"
     c. "(2) in one foreground round Haiku hands that path to `opus-architect` (design review) and/or `sonnet-analyst`" -> "(2) in one foreground round Haiku hands that path to `opus-architect` (design review)" (only if refinement 2A approved)
     d. insert before "Small tasks": "Sonnet-tier task (analysis, review, comparison, multi-file change, scoped multi-step, release/merge; arrives with a `[routing: sonnet tier ...]` hint) → Haiku delegates it directly to `sonnet-analyst` in one foreground round, no plan; `ESCALATE: <path>` → `opus-architect` with that path."
     e. "Small tasks: Haiku does them via Haiku agents directly." -> "Haiku-tier tasks: Haiku does them via Haiku agents directly."
     unchanged: "Opus is for design only, never main.", "Forced `opus로`/`sonnet으로` (`--model`) is kept as is.", last sentence; CLAUDE.md:9, :12 (agree with plan line 39).
   - non-policy text the plan missed (code, same commit, no approval needed): repl.go:898 flag help "a one-shot Sonnet classifier tags size and adds the plan->review->execute hint to large tasks" -> "tags tier (haiku|sonnet|opus); sonnet tier -> sonnet-analyst hint, opus tier -> plan->review->execute hint"; repl.go:60 comment "large work:" -> "opus-tier work:"; classifier.go :30-31 planHint comment "large task" -> "opus-tier task".
   - not to change: .claude/agents/opus-architect.md:22 and sonnet-analyst.md:23 "Plan review" paragraphs (still valid; sonnet one just becomes rarely used).

5. unverified items -> only conductor_test.go must be read (by the implementer, lines 22-141) before coding; metricscmd.go needs no read. Checked now:
   - A1 Size outside classifier.go: resolved. Only conductor_test.go uses rr.Size/SizeDefaulted/planHint; other `.Size()` hits are os.FileInfo (drain.go:65, compress_archive.go:41-42). No hook/script reads "size"/"size_defaulted".
   - A2 metricscmd.go: resolved, no read needed. CSV header (metricscmd.go:31) has no size/kind field; it switches on "stage" only. Adding `tier` and keeping derived `size` breaks nothing.
   - A5 tests: plan step 4 "기대값 동일, 인자만 변경" is WRONG. Must-read and rewrite:
     - TestParseRouteSize :27-30 cases (missing -> sonnet, "Small." -> haiku, medium -> sonnet) and :47-51: asserts missing size on haiku main gets planHint -> after change missing tier gets sonnetHint; flip assertion (planHint absent, sonnetHint present).
     - TestMainRouteSwitchPolicy :103 asserts classifyPrompt contains "size: small|large" -> fails after prompt change; change to "tier: haiku|sonnet|opus". Inputs :88-93 use `size:` lines: rewrite to `tier:` (old-format still parses, but tests should exercise the new format; keep one size-compat case in TestParseRouteSize).
     - TestHaikuPlanWorkflow :119 and :121 use `size: large` with plan=true -> under compat that maps to sonnet -> planHint false -> test fails. Rewrite with `tier: opus` (plan=true) and add `tier: sonnet` (plan=false, sonnetHint=true) cases.
     - TestClassifyPromptSizeRule :55: replace both strings ("tier: haiku|sonnet|opus", new unsure rule wording) - plan already says so.
     - TestClassifierLive :65-72 (not in plan): checks model only (always haiku) -> unaffected; optional: add a tier field to each case (tradeoff/strategy -> opus, pushback/correction -> opus or sonnet, two mechanical -> haiku) to reuse it as a 6-prompt smoke test.
   - DECISIONS append-only: confirmed (see 4).

## other improvements
- order: (1) relabel 30 samples into a new labels file BEFORE any live call: reuse P1-P10/N1-N10/L1-L10 prompts from 261029-test-router-30-prod-args.md; split L*/large P* into sonnet/opus; P3 -> haiku; add new prompts to reach >=10 opus and >=10 sonnet. (2) code units 1-6 + tests (haiku-coder, one brief, spec = plan + this review). (3) go test narrow then ./... (4) 30-sample live run (haiku-runner). (5) CLAUDE.md/DECISIONS edits after user approves wording (haiku-batch-worker, exact text from section 4).
- acceptance add: opus->sonnet misses <=2/10 (the expensive direction; recoverable via ESCALATE), opus->haiku 0, sonnet->haiku <=1; plus 0 missing tier.
- classifyPrompt growth: keep the rewrite within ~+300 chars of current length (per-call cost; no length test exists for it).
- metrics: add `tier`; keep `size` derived and `size_defaulted` key name for continuity (no rename).
- pickModel: forced opus main + topic new + tier haiku -> delegate (same as old small); sonnet tier on forced opus/sonnet main -> no hint (hints attach only on Haiku main). State this in a TestMainRouteSwitchPolicy case.
