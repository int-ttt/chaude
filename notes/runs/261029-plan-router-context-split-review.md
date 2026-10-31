# review: 261029-plan-router-context-split.md
date: 2026-10-29
reviewer: opus-architect
scope: improvements only; plan not edited; no source change

## new evidence (gathered for this review, 0 API calls)
- E1 neither earlier test used production args. test-sonnet-vs-haiku (/tmp/router-test.wLXJCF/run_one.sh): router prompt in user msg, default system prompt, default tools, --max-turns 1, --strict-mcp-config. test-injection F: same + CLAUDE.md in user msg. production classify() (classifier.go:122-125): --system-prompt classifyPrompt, --tools "", no --max-turns, no --strict-mcp-config, cwd os.TempDir(), cleanEnv(0).
- E2 haiku F failure modes (error_max_turns = tried to use tools; prose answer) both need tools or the default system prompt. Production has neither, so the F result says nothing about production Haiku.
- E3 CLAUDE.md reach (A1): no ~/.claude/CLAUDE.md; no CLAUDE.md in $TMPDIR (/var/folders/rb/.../T/) or its parents (/var/folders, /var). Project CLAUDE.md cannot reach the router through the files on disk.
- E4 production classify metrics (notes/metrics/*.jsonl, stage=classify, n=103, errors 0): result_ctx median 19,602 tokens (cache_read median 18,519, cache_creation median 1,108); cost_usd median $0.0079, mean $0.0254 (outliers up to 174k ctx / $0.132 on 10-28, cause unknown). classifyPrompt ~3.7k chars plus input is ~1-2k tokens, so about 17-18k tokens per call come from somewhere else even with --system-prompt and --tools "".
- E5 ~/.claude/settings.json: hooks {} , mcpServers none, enabledPlugins = pyright-lsp, session-report, serena. Plugins can add MCP tools (serena), hooks, skills. These are the likely source of E4, not CLAUDE.md.
- E6 claude 2.1.312 --help: --setting-sources (user,project,local), --disable-slash-commands, --strict-mcp-config, --exclude-dynamic-system-prompt-sections exist; --max-turns is not listed but works (earlier tests got error_max_turns). --bare needs ANTHROPIC_API_KEY (OAuth never read) -> do NOT use; it conflicts with cleanEnv (billing).
- E7 plan cost estimate is off: production Sonnet router ~$0.008/call (E4), not $0.05. The $0.05 came from the test setup with default tools.

## decisions
1. (1) split work: defer U1 (isolation flags, MkdirTemp). The CLAUDE.md part is not needed: A1 holds by code (E1) and by disk (E3). First step = Step 0 baseline: 30 samples x {sonnet, haiku} with exact production classify() arguments. Keep U1 only as a conditional step, justified by the ~18k-token unexplained context (E4), not by CLAUDE.md. Run U1 only if Step 0b (below) shows a flag removes most of that context. Even if Haiku is not adopted it may still be worth doing: cache_read 18.5k x $0.30/M ~ $0.0056 = ~70% of the Sonnet router cost per call. After U1, re-run the same 30 x 2 with the U1 args.
   - how to guarantee identical args: replace U4 (shell script copying the args, which will drift) with a Go live test that calls classify() itself, e.g. internal/harness/router_eval_test.go TestRouterEval, gated CHAUDE_LIVE=1, reading the frozen eval set and writing a results jsonl to notes/runs/. Only prerequisite: router model selectable without changing defaults = do U2 first (routerModel(), env CHAUDE_ROUTER_MODEL, default "sonnet"). U2 is the only code change before Step 0.
   - eval set format must carry per sample: id, cur (current session model), lastUser (previous prompt, can be ""), prompt, expected size, expected kind; cands nil. Without lastUser/cur, the plan's input differs from production in.
2. (2) minimum conditions:
   - P-S: production args, sonnet, 30 prompts, 1 run (~$0.25).
   - P-H: production args, haiku, 30 prompts, 3 runs (~$0.2 total). Repeats are cheap and give the only variance signal; score per prompt by majority, and also report how many prompts were 3/3 consistent.
   - M control: no new calls. Compare P-H on the P1-P10 subset with the existing haiku M 8/10 and sonnet M 10/10 (test-injection file). Run M-haiku x30 (~$0.09) only if P-H fails the gate, to tell whether the cause is prompt placement (system vs user msg, A4) or the model's limit.
   - drop condition F entirely (not a production path, E2).
   - Step 0b (leak attribution, ~4 haiku calls): 1 call with production args + --output-format stream-json --verbose and read the init event (tools, mcp_servers, slash_commands/skills, plugins). Then 1 call each adding --strict-mcp-config, --setting-sources "", --disable-slash-commands. Record result ctx for each.
   - budget: cap $1.00 (was $2.50); stop on cumulative total_cost_usd.
3. (3) threshold: "Haiku >= 2 of 30 below Sonnet -> keep Sonnet" is not justified as the only gate. At n=30 one run, 28/30 has a 95% CI of about 78-99%, so a 1-2 point gap is noise and the rule ignores which direction the error goes. Replace with (all must hold to switch; else Sonnet stays, U2 default unchanged):
   - a. under-routing (expected large -> Haiku small, majority vote) = 0. This is the harmful direction: a large task skips planHint. Earlier Haiku errors (P3, P7) were all small->large, which only costs plan overhead.
   - b. size accuracy, majority vote: Haiku >= Sonnet - 1.
   - c. consistency: >= 27/30 prompts with the same Haiku size in 3/3 runs.
   - d. parse failures <= 1 across 90 Haiku calls (production fails open: classify err -> line sent unrouted, route() returns line), and each one is counted as wrong in b.
   - kind accuracy and topic: report only, not a gate. kind matters only for the worker pick on small edit. topic only changes the result when cur=opus (pickModel).
   - third rollback rule (U1 vs "9-10/10 earlier"): change the baseline to the Step 0 P-S result. The 10-sample number came from non-production args.
   - the final switch stays a user decision (open question 1). A pass only makes the switch allowed.
4. (4) verify A1 and A2 first, in this order:
   - A1 (CLAUDE.md does not reach the router): already supported at 0 cost (E1 code, E3 disk). Closing check, no extra call: in Step 0, router result_ctx with production args should stay around 19.6k (E4). Adding a test CLAUDE.md copy to $TMPDIR is not needed. The TestClassifyArgs check that the args contain no CLAUDE.md string is enough as a regression guard.
   - A2 (global settings/plugins inject context into -p): reword the assumption to "enabled plugins (serena MCP, session-report, pyright-lsp) and skills listing add ~17-18k tokens" (E4, E5). Verify with Step 0b: the init event lists what is loaded, then measure which single flag drops result_ctx toward ~2k. Also check that --setting-sources "" still authenticates through OAuth/keychain and does not change the model or env settings (rc 0, same `model` in the result json).
   - A4 is answered by Step 0 itself (P-H vs existing M on P1-P10). A5 should be reworded: the CLAUDE.md cause is already ruled out for production, so the question is now "Haiku limit vs plugin/skills noise", answered by P-H before and after U1. A6 (--max-turns 1) is moot with --tools "": do not add the flag in the baseline.

## other improvements
- order: U2 -> eval set frozen -> Step 0b -> Step 0 (P-S, P-H) -> decide U1 -> (U1 + U3 + re-run) -> user decision. U3 depends on U1's buildRouterArgs(); if U1 is skipped, U3 shrinks to testing that classify() still uses --system-prompt and --tools "" (still needs a small args builder).
- U1 MkdirTemp per call: low value (E3); if kept, reuse one dir per process, not one per call (latency and cleanup on crash).
- 30-sample source (open question 2): metrics rows do not include prompt text, so the new 20 must be written by hand or taken from the transcripts. Keep small:large 15:15, and include >= 5 large prompts that sound small (short wording, multi-file effect). These test the gate in 3a.
- record per call: model, run#, size, kind, topic, parse ok, cost, result_ctx, duration_ms. The results table should separate sonnet and haiku cost (the plan mixes them).
- outlier rows on 10-28 (ctx 174k, $0.132): find out if they are pre-`--tools ""` code before using mean cost anywhere; use the median.
