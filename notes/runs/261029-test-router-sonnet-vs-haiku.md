# test: router sonnet vs haiku (10 prompts)
date: 2026-10-29
repo_edits: none
commits: none
raw_outputs: /tmp/router-test.wLXJCF/out/ (not in repo)

## pre-check
ANTHROPIC_API_KEY: unset
ANTHROPIC_AUTH_TOKEN: unset
ANTHROPIC_BASE_URL: unset
CLAUDE_CODE_USE_BEDROCK: unset
CLAUDE_CODE_USE_VERTEX: unset

## calls
total_calls: 20 (10 prompts x sonnet, haiku)
failed_calls: 0 (exit code 0 on all; no retries used)
parallelism: 3
timeout: 120s
ledger_total_usd: 0.5466
budget_limit_usd: 2.50 (stop rule not triggered)

## table
| id | expected | sonnet size | haiku size | sonnet kind | haiku kind | sonnet topic | haiku topic | sonnet $ | haiku $ | rc s/h | agree |
|----|----------|-------------|------------|-------------|------------|--------------|-------------|----------|---------|--------|-------|
| P1 | small | small | small | edit | edit | new | new | 0.084876 | 0.002987 | 0/0 | yes |
| P2 | small | small | small | edit | edit | new | new | 0.087324 | 0.002471 | 0/0 | yes |
| P3 | small | small | large | edit | edit | new | new | 0.043463 | 0.002686 | 0/0 | NO |
| P4 | large | large | large | other | other | same | new | 0.043623 | 0.002293 | 0/0 | yes |
| P5 | large | large | large | other | other | new | new | 0.043417 | 0.002553 | 0/0 | yes |
| P6 | large | large | large | other | other | same | new | 0.044575 | 0.002469 | 0/0 | yes |
| P7 | small | small | large | edit | edit | new | new | 0.043413 | 0.002458 | 0/0 | NO |
| P8 | large | large | large | other | other | new | new | 0.043551 | 0.002432 | 0/0 | yes |
| P9 | small | small | small | other | other | new | new | 0.043333 | 0.002489 | 0/0 | yes |
| P10 | large | large | large | other | other | same | new | 0.043649 | 0.002551 | 0/0 | yes |

## summary
sonnet_size_accuracy: 10/10
haiku_size_accuracy: 8/10
size_disagreements (sonnet vs haiku): P3, P7 (haiku said large; expected small)
kind_disagreements: none
topic_disagreements: P4, P6, P10 (sonnet same, haiku new); not part of the score
sonnet_cost_total_usd: 0.5212 (sum of 10 sonnet calls; approx)
haiku_cost_total_usd: 0.0249 (sum of 10 haiku calls; approx)
total_cost_usd: 0.5466
note: sonnet cost per call ~0.043-0.088 (includes cache creation); haiku ~0.002-0.003.
note: P2 sonnet task text said the function target is unspecified; haiku also said unspecified. Both correct to flag.
note: P9 haiku task text adds "ask which one first" although prompt gives no test name; not scored.

## doubts
- "route tag" interpreted as kind (lookup|edit|other) since the parser has no separate route label; model target is haiku/sonnet by policy, not classifier output.
- Expected sizes are the user's judgment; P3 (version bump) is "large" per classifier rule ("version bump" listed under large), so the Haiku result may match the prompt rule while disagreeing with the expected label. Score as specified.
- Single run per call, no repetition; variance not measured.
