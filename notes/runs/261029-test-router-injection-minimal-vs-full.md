# test: router injection minimal (M) vs full (F), sonnet and haiku (10 prompts)
date: 2026-10-29
repo_edits: none (this file only)
commits: none
raw_outputs: /tmp/router-inj/out/ (not in repo; files <Pn>.<model>.<cond>.json|.err|.rc)
scripts: /tmp/router-inj-scripts/ (driver.sh, job.sh, score.py)
prompts: /tmp/router-test.wLXJCF/in/P1..P10.txt (same as notes/runs/261029-test-router-sonnet-vs-haiku.md)

## conditions
M: user message = router prompt + user prompt (in/M.Pn.txt); cwd = empty mktemp -d under /tmp; --system-prompt "You are a classifier. Follow the instructions exactly."; --strict-mcp-config --max-turns 1 --output-format json --max-budget-usd 0.10
F: user message = full CLAUDE.md (8005 chars) + blank line + router prompt + user prompt (in/F.Pn.txt); same cwd and flags, no --system-prompt override (default system prompt)
env: env -u ANTHROPIC_API_KEY on every call; 120s timeout (perl alarm); retry once on failure; parallel 3

## pre-check
ANTHROPIC_API_KEY: unset
ANTHROPIC_AUTH_TOKEN: unset
ANTHROPIC_BASE_URL: unset
CLAUDE_CODE_USE_BEDROCK: unset
CLAUDE_CODE_USE_VERTEX: unset
claude: 2.1.312 (Claude Code)

## calls
total_calls: 40 planned (sonnet 20, haiku 20); launched 40; completed (rc 0, not is_error) 36
sonnet: 20/20 completed, 0 retries
haiku: 16/20 completed (rc 0); 6 retried (all haiku F: P1, P2, P3, P5, P7, P9 needed 2 attempts); 4 haiku F still error after retry (P1, P3, P5, P9)
budget: sonnet ledger sum 1.1524 (stop rule at 2.50 not triggered)
ledger_total_usd: 1.2015 (sum of total_cost_usd in final json per call; first-attempt cost of the 6 retried haiku calls was overwritten, est. < 0.02)

## table (size; cost usd per call)
| id | exp | sonnet M | sonnet F | haiku M | haiku F |
|----|-----|----------|----------|---------|---------|
| P1 | small | small $0.0759 | small $0.0642 | small $0.0040 | NONE(error_max_turns) $0.0005 |
| P2 | small | small $0.0769 | small $0.0662 | small $0.0041 | NONE(no route lines) $0.0010 |
| P3 | small | small $0.0433 | large $0.0671 | large $0.0026 | NONE(error_max_turns) $0.0034 |
| P4 | large | large $0.0434 | large $0.0646 | large $0.0024 | large $0.0037 |
| P5 | large | large $0.0432 | large $0.0645 | large $0.0025 | NONE(error_max_turns) $0.0005 |
| P6 | large | large $0.0437 | large $0.0655 | large $0.0026 | large $0.0038 |
| P7 | small | small $0.0432 | small $0.0638 | large $0.0024 | large $0.0007 |
| P8 | large | large $0.0434 | large $0.0655 | large $0.0025 | large $0.0037 |
| P9 | small | small $0.0431 | small $0.0648 | small $0.0024 | NONE(error_max_turns) $0.0004 |
| P10 | large | large $0.0436 | large $0.0663 | large $0.0024 | NONE(no route lines) $0.0036 |

## summary
sonnet_M: size_acc 10/10 (100%); parse_fail 0/10; cost_sum 0.4998; avg/call 0.0500
sonnet_F: size_acc 9/10 (90%); parse_fail 0/10; cost_sum 0.6526; avg/call 0.0653
haiku_M: size_acc 8/10 (80%); parse_fail 0/10; cost_sum 0.0279; avg/call 0.0028
haiku_F: size_acc 3/10 (30%) with parse failures counted as wrong; parse_fail 6/10; cost_sum 0.0212; avg/call 0.0021
haiku_F parsed-only: 3/4 (P4, P6, P8 correct; P7 wrong)

## usage (json usage field, summed over 10 calls per cell)
sonnet_M: cache_read 67280; cache_creation 120285
sonnet_F: cache_read 88760; cache_creation 151770
haiku_M: cache_read 67280; cache_creation 120489
haiku_F: cache_read 168751; cache_creation 80015
note: cache_creation is large even in M (system prompt replaced, but claude code tool/context defaults still counted); F adds ~31k sonnet cache_creation over M (about 3.1k per call; CLAUDE.md is ~2k tokens).

## disagreements (size != expected)
- P3 sonnet F: large (kind=edit, topic=new); expected small. sonnet M = small (matches).
- P3 haiku M: large; expected small. haiku F: no route output.
- P7 haiku M: large (kind=edit, topic=new); expected small. haiku F: large (same wrong answer).
- P1, P2, P3, P5, P9, P10 haiku F: no usable route output (see below). P5 expected large.
- M vs F size differences (same model): sonnet P3 (M small, F large); haiku P3 (M large, F none); haiku P1, P2, P9 (M small, F none); haiku P5, P10 (M large, F none).

## haiku F failure modes (not retried beyond the one retry)
- error_max_turns (P1, P3, P5, P9; rc=1 after retry): with CLAUDE.md in context Haiku tried to act (tool-style work) instead of emitting five route lines; --max-turns 1 ended the run with an empty result.
- no route lines (P2, P10; rc=0): Haiku answered the prompt in prose (P10: asked which two approaches) instead of the route format.
- P7 parsed as large (wrong), so haiku F is the only cell where the wrong label appeared with a valid parse as well.

## doubts
- Single run per cell; no repeat, so variance is not measured. Sonnet P3 flip (M small vs F large) and haiku P7 are one-shot results.
- Expected sizes are the user's judgment; P3 "version bump" is listed as large in the classifier rule, so model answers may follow the rule text rather than the label.
- Haiku F failures are with --max-turns 1 and an empty cwd; a larger turn budget or a tool-less setup could change them. Not tested here (spec fixed).
- Costs for the 6 retried haiku calls reflect the second attempt only.
- The router prompt was given as the user message in both conditions (as spec: "router prompt + user prompt"); the production classifier uses it as --system-prompt, so M is not identical to production.
