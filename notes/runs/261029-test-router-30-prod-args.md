# test: router 30 samples, production args (classifier.go classify())
date: 2026-10-29
status: done (Sonnet 30/30 completed, Haiku 90/90 completed)
repo_edits: none
commits: none
samples_src: P1-P10 from notes/runs/261029-test-router-sonnet-vs-haiku.md (prompt text from prior setup); N1-N10 small, L1-L10 large (new)
pastes_check: notes/pastes/578df68f.md has no user-written sample prompts (subagent hand-back text only; not used)
raw_outputs: /tmp/router30/out/ (not in repo)
orchestrator: /tmp/router30/run.py (not in repo)

## prod args (classifier.go:122-125)
claude -p <in> --model <sonnet|haiku> --output-format json --system-prompt <classifyPrompt extracted from classifier.go, 1902 chars> --tools "" --no-session-persistence
extra: env -u ANTHROPIC_API_KEY, --max-budget-usd 0.10, 120s timeout (perl alarm; subprocess timeout 130s), cwd = fresh temp dir
in: "current session model: sonnet (keep it unless topic is new)\nprevious user prompt: \nuser prompt:\n<prompt>" (no candidates, no prior prompt)
haiku model: alias "haiku" (exact id not pinned)
parallel: 3; retry: 1 per failed call (rc!=0 or no size line)

## pre-check
ANTHROPIC_API_KEY: unset
ANTHROPIC_AUTH_TOKEN: unset
ANTHROPIC_BASE_URL: unset
CLAUDE_CODE_USE_BEDROCK: unset
CLAUDE_CODE_USE_VERTEX: unset

## expected + results
| id | expected | sonnet | haiku r1 | haiku r2 | haiku r3 | haiku consistent |
|----|----------|--------|----------|----------|----------|------------------|
| P1 | small | small | small | small | small | yes |
| P2 | small | small | small | large | small | NO |
| P3 | small | small | large | large | large | yes (all wrong) |
| P4 | large | large | large | large | large | yes |
| P5 | large | large | large | large | large | yes |
| P6 | large | large | large | large | large | yes |
| P7 | small | small | small | large | large | NO |
| P8 | large | large | large | large | large | yes |
| P9 | small | small | small | small | small | yes |
| P10 | large | large | large | large | large | yes |
| N1 | small | small | small | small | small | yes |
| N2 | small | small | small | small | small | yes |
| N3 | small | small | small | small | small | yes |
| N4 | small | small | small | small | small | yes |
| N5 | small | small | small | small | small | yes |
| N6 | small | small | small | small | small | yes |
| N7 | small | small | small | small | small | yes |
| N8 | small | small | small | small | small | yes |
| N9 | small | small | small | small | small | yes |
| N10 | small | small | small | small | small | yes |
| L1 | large | large | large | large | large | yes |
| L2 | large | large | large | large | large | yes |
| L3 | large | large | large | large | large | yes |
| L4 | large | large | large | large | large | yes |
| L5 | large | large | large | large | large | yes |
| L6 | large | large | large | large | large | yes |
| L7 | large | large | large | large | large | yes |
| L8 | large | large | large | large | large | yes |
| L9 | large | large | large | large | large | yes |
| L10 | large | large | large | large | large | yes |

## summary
sonnet_completed: 30/30 (calls: 30; retries: 0)
sonnet_size_accuracy: 30/30
sonnet_large_to_small: 0 (pass)
sonnet_parse_failures: 0/30
sonnet_call_failures: 0/30

haiku_calls: 90 (30 prompts x 3 repeats); retries: 1
haiku_size_accuracy: r1 29/30, r2 27/30, r3 28/30 (total 84/90)
haiku_consistency (all 3 repeats agree): 28/30 (P2, P7 disagree)
haiku_large_to_small: 0 per repeat, 0 total (pass)
haiku_parse_failures: 0/90
haiku_call_failures: 0/90 (final)

haiku_errors (all small->large, none large->small):
- P3 (expected small, "버전 문자열 올려줘"): wrong in all 3 repeats
- P2: wrong in r2
- P7: wrong in r2, r3

## cost ledger (USD)
sonnet_total_usd: 0.220862 (30 calls, all completed)
haiku_total_usd: 0.079737 (90 calls, as logged)
haiku_retry_fail_unlogged_usd: 0.156819 (N6 r1 attempt 1: rc=1, subtype error_max_budget_usd, "Reached maximum budget ($0.1)"; reported total_cost_usd 0.156819; my script logged 0 for rc!=0, corrected here)
total_estimated_usd: 0.457418
budget_stop: not triggered (Sonnet running sum stayed far below 1.00)

## doubts
- P3 expected label "small" conflicts with classifyPrompt rule ("version bump" is listed under large). Haiku's "large" matches the prompt rule; scored as expected label per spec. Sonnet also said small on P3, so the rule and Sonnet disagree too; treat as label issue.
- Haiku's N6 r1 first attempt consumed $0.157 against a $0.10 cap, which the cap did not stop as expected; cause not investigated.
- Single run per prompt set; Haiku variance observed (P2, P7). n=30 is small for the consistency number.
- The expected sizes for P1-P10 are the prior test's labels; N1-N10 and L1-L10 are new labels written before any call.
