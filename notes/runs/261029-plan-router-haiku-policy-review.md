# review: 261029-plan-router-haiku-policy.md
date: 2026-10-29
reviewer: opus-architect (design review only; plan/policy files not edited, nothing implemented)
verdict: plan direction OK; change the pass gates, eval set size, tie-break, rollback prompt coupling, and step order as below.

1. Gate 1 (zero non-haiku sent to haiku): keep it as a hard gate, but count it correctly.
   - why keep: the costs are lopsided. A leak sends design or multi-file work to a Haiku session with no plan/sonnet hint, so the answer is wrong. Over-classifying only costs money, and the planHint/sonnetHint path absorbs it.
   - unit: count distinct prompts, not calls. If any of the 3 runs routes a non-haiku prompt to haiku, that prompt counts as a leak. The 3 calls on one prompt are correlated, so 300 calls do not show a 1% bound.
   - what a 0 means (rule of three, 95% upper bound per prompt is about 3/n): plan's 50 non-haiku prompts -> 0 leaks only shows <=6%. 100 distinct non-haiku -> <=3%. Decision: at least 100 distinct non-haiku prompts.
   - label noise: boundary items between haiku and sonnet get the label sonnet (same as the tie-break) and stay in gate 1. Pre-marked boundary items are capped at 10% of the set. Labels must not change after any model call. If the Sonnet baseline also routes an item to haiku, report it, but it still counts.
   - add as a reference metric (not a gate): the same leak count for the Sonnet baseline. If Sonnet also leaks, the label or the rule set is the problem, not Haiku.

2. Gate 2 (tier accuracy within 2%p of Sonnet): sample per tier.
   - problem: with 100 prompts, 1 prompt = 1%p and the binomial SE at about 97% is about 1.7%p, so a 2%p gap cannot be resolved. With sonnet at 20 (plan) or 4 (old set), the sonnet tier is mostly guesswork.
   - decision: eval set of 150 prompts, haiku 50 / sonnet 50 / opus 50. Absolute floor is 30 per tier (1 error = 3.3%p). The 50 is chosen so that 1 error = 2%p.
   - drop the "50 fixed + 50 보조" split. All prompts are labeled, frozen and scored. Zero overlap with the old 30-sample set (it becomes the dev set, see 3).
   - gate, count-based and paired on the same prompts:
     a. overall: Haiku errors (mean of the 3 runs) <= Sonnet errors + 2%p of 150 (= 3 prompts).
     b. per tier: Haiku errors in that tier (run mean) <= Sonnet errors in that tier + 2 prompts (4%p of 50). This stops a weak sonnet tier from hiding inside the overall average.
     c. opus->sonnet under-classification (the other quality-losing direction) <= Sonnet baseline + 2 prompts.
   - Sonnet baseline: 1 run, as in the plan. Its run-to-run noise is accepted. Report it as a reference metric.
   - cost check: Sonnet 150 x ~$0.0045 = ~$0.68; Haiku new prompt 450 x ~$0.0003 = ~$0.14; extra arm in item 3 adds ~$0.14; total ~$1.0, under the $3 cap.
   - consistency gate 3 (>=90% of prompts with the same tier in all 3 runs) stays, computed over the 150 prompts.

3. Prompt tuning: constraints, and order relative to the test set.
   order (decision: tune first, then fix the test set):
     a. tune only on the dev set = old 30 prompts (notes/runs/261029-haiku-router-30-samples-list.md), Haiku 3 runs per draft.
     b. freeze the prompt: record its sha256 in the test file header. No edits after this point.
     c. a separate agent (haiku-drafter, fresh instance) writes the 150-prompt test set and labels. It sees only the tier definitions in this review/plan and never the prompt text, so it cannot write easy prompts that just match keywords.
     d. one run. No re-tuning against test results. If the run fails: roll back. Any later re-tune needs a new test set (at least the failed tier's 50 replaced).
   the new prompt must keep:
     - output: exactly 5 lines in the order topic/tier/kind/worker/task; fixed values only; "nothing else". Keep "Always output topic, tier, kind and worker."
     - every semantic rule of the current tier lists, with no silent drops. The plan's draft drops these, and they must come back: "version bump with exact text" (haiku), "one-step edit of one file with a clear scope" (haiku), "clear multi-step task" and "comparison summary" (sonnet), "new feature or refactor with a given scope" -> sonnet vs "whose design is open" -> opus, "repeated failure", "'opus로'". Add a table of old phrase -> new phrase to the test file.
     - no-match default = sonnet, never haiku. Haiku only on an explicit haiku-rule match. The draft's "first match, opus->sonnet->haiku" leaves no-match undefined, which is a leak risk.
     - tie-break: keep the current semantics exactly: "unsure haiku/sonnet -> sonnet; unsure sonnet/opus -> sonnet". Decision: do NOT adopt the draft's "always higher". It changes sonnet/opus routing (more opus = Haiku plan + Opus review rounds) and mixes a second variable into the model test. Safety only needs "unsure -> never haiku", which the current rule already gives.
     - topic: "When unsure, same." plus the existing new/same definitions.
     - kind/worker: keep "A lookup or edit that is tier haiku is sent straight to a Haiku session" and "worker only for kind edit, else new". Fix only the missing space in "session.worker:".
     - task suffix: 'reply: Korean 존댓말, ... before replying' byte-exact.
     - examples: one per tier, Korean, typical cases (not boundary), new sentences not in the dev or test set.
     - rules in English, Korean trigger words included in brackets (설계, 트레이드오프, 추천, 전략, 원인, 계획, 'opus로'), because inputs are Korean.
     - length: the -30% target is a reference metric, not a gate. Saving ~150 tokens on a ~$0.0003 Haiku call is negligible. Never cut a rule to hit the length.
   - add test arm (c): Haiku + current prompt, 3 runs, on the same 150 (~$0.14). It separates the model effect from the prompt effect and gives a validated fallback. If (c) passes and the new prompt (b) does not beat it on gates 1-3, ship Haiku with the current prompt.
   - extra gate 4 item: the task suffix appears byte-exact in 100% of parsed haiku-run tasks. If not, see item 5 (code-side suffix).
   - runner drift: /tmp/router3t/run.py calls claude directly, not the chaude binary. It must load the prompt from the built code (e.g. a test helper that writes classifyPrompt to a file when an env flag is set) and check the sha256 from step b. Hand-copying the prompt into the runner is not allowed.

4. Rollback via CHAUDE_ROUTER_MODEL: confirmed as planned only. It does not exist yet.
   - fact: grep finds no CHAUDE_ROUTER_MODEL anywhere in internal/harness. The model is the const classifier.go:14 `conductorModel = "sonnet"`, used at :141. conductor_test.go:112 pins it to "sonnet". So today a rollback is a code revert. It becomes a config change only after change 1 ships.
   - decision: implement change 1 first, as its own step (default still sonnet, behavior unchanged), so the rollback path exists before the Haiku default goes live. Required details:
     a. the env value is trimmed and lowercased; allowed values are haiku|sonnet. An empty or invalid value -> routerModelDefault (not hard-coded "haiku" as the plan says), plus a one-time stderr warning. Otherwise a typo like "sonet" during a rollback would silently stay on haiku.
     b. couple prompt to model: keep the current prompt verbatim as classifyPromptSonnet and add classifyPromptHaiku. Select by the resolved model. Then CHAUDE_ROUTER_MODEL=sonnet restores exactly the validated pre-change behavior. The plan as written would run Sonnet with the untested Haiku prompt on rollback.
     c. read it in the chaude process (os.Getenv; cleanEnv only affects the child). Document that a running chaude must be restarted to pick it up.
     d. metrics stage=classify: add router_model and router_prompt (e.g. "haiku/v2", "sonnet/v1") so post-rollout cost and accuracy can be split.
     e. tests: replace conductor_test.go:112 with a table: unset -> default; "sonnet"/"SONNET " -> sonnet; "haiku" -> haiku; "opus"/"x" -> default; prompt selected matches model.
   - flipping the default to haiku after the gates pass is a one-line const change (routerModelDefault). Policy docs (CLAUDE.md:32, DECISIONS.md new line) change only after the pass, as the plan says.

5. 5-line output format: keep it now; decide later.
   - why: (a) changing model + prompt + format at once mixes up which change caused a result, and the gates could not attribute a failure; (b) a one-line format needs the parser (classifier.go:75-132) and worker/task handling rewritten, plus new parser tests, all outside a model-switch change; (c) the saving is small: the label prefixes are about 10 output tokens.
   - the real output cost is the long task suffix that the model retypes on every call, which is also a drift risk for Haiku. That is the next candidate and does not depend on the line format: the harness appends the suffix in code (parser adds it if it is missing), and the prompt asks only for the terse task. Do this as a follow-up only if the suffix gate in item 3 fails or the post-rollout metrics show output tokens dominating. Otherwise revisit the format after 1-2 weeks of router_model metrics.

## revised order
1. change 1 (env + two prompt constants + metrics fields + tests), default sonnet -> go test ./internal/harness.
2. tune the Haiku prompt on the dev set (30) -> freeze with sha256.
3. separate agent writes and freezes the 150-prompt test set (50/50/50, boundary <=10%) -> optional user label check (plan open q4) before any model call.
4. run: Sonnet+current x1, Haiku+new x3, Haiku+current x3, cap $3.
5. gates 1-4 (as revised) -> pass: routerModelDefault="haiku" (+ prompt choice per item 3) + docs; fail: stay on sonnet, record in LESSONS.md.

## [haiku] steps
- haiku-coder: classifier.go + conductor_test.go change 1 per item 4 a-e (one instance).
- haiku-drafter (fresh, no prompt text in brief): the 150-prompt test set per items 2-3.
- haiku-runner: the run in step 4, using the sha256-checked prompt dump.
