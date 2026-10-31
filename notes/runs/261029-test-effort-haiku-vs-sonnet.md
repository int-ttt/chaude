# test: router effort, Haiku high/xhigh vs Sonnet low/medium, 30 samples, production args + --effort
date: 2026-10-29
status: done (120/120 completed, 0 call failures, budget stop not triggered)
repo_edits: this note only
commits: none
samples: same 30 prompts and expected sizes as notes/runs/261029-test-router-30-prod-args.md (copied from /tmp/router30/samples.json, unchanged)
raw_outputs: /tmp/router30-effort/out/ (not in repo)
orchestrator: /tmp/router30-effort/run_effort.py (not in repo)
results: /tmp/router30-effort/results.json (not in repo)

## flag check (Step 0)
claude --help line 84: "  --effort <level>                      Effort level for the current session"
accepted values (help text line 85): (low, medium, high, xhigh, max)
used: --effort high | xhigh | low | medium (appended after production args)

## prod args (classifier.go:122-125) + extras
claude -p <in> --model <haiku|sonnet> --output-format json --system-prompt <classifyPrompt, extracted from classifier.go> --tools "" --no-session-persistence --max-budget-usd 0.10 --effort <level>
prefix: env -u ANTHROPIC_API_KEY, perl alarm 120s (subprocess timeout 130s), cwd = fresh temp dir
in: "current session model: sonnet (keep it unless topic is new)\nprevious user prompt: \nuser prompt:\n<prompt>"
haiku model: alias "haiku"; sonnet model: alias "sonnet"
parallel: 3; retry: 1 per failed call (rc!=0 or no size line); calls: 1 repeat per condition

## pre-check
ANTHROPIC_API_KEY: unset
ANTHROPIC_AUTH_TOKEN: unset
ANTHROPIC_BASE_URL: unset
CLAUDE_CODE_USE_BEDROCK: unset
CLAUDE_CODE_USE_VERTEX: unset

## summary table
| condition | model | effort | calls | size accuracy | large->small | small->large | parse failures | retries | total cost USD | mean cost/call USD | mean output tokens |
|-----------|-------|--------|-------|---------------|--------------|--------------|----------------|---------|----------------|--------------------|--------------------|
| H-high    | haiku  | high   | 30 | 28/30 | 0 | 2 | 0/30 | 0 | 0.019818 | 0.000661 | 597.6 |
| H-xhigh   | haiku  | xhigh  | 30 | 28/30 | 0 | 2 | 0/30 | 0 | 0.025474 | 0.000849 | 1024.6 |
| S-low     | sonnet | low    | 30 | 30/30 | 0 | 0 | 0/30 | 0 | 0.176307 | 0.005877 | 111.7 |
| S-medium  | sonnet | medium | 30 | 30/30 | 0 | 0 | 0/30 | 0 | 0.177057 | 0.005902 | 114.2 |

## misses
- H-high: P3 (expected small, "버전 문자열을 0.7.3으로 올려줘") -> large; P7 (expected small, "config 파일 한 줄 주석 추가") -> large
- H-xhigh: P2 (expected small, "parser.go의 이 함수 이름만 바꿔줘") -> large; P3 -> large
- S-low, S-medium: none

## cost ledger (USD)
calls: 120 (30 x 4 conditions), attempts: 120, retries: 0
sum of total_cost_usd over all attempt outputs: 0.398657
per-condition sums: H-high 0.019818, H-xhigh 0.025474, S-low 0.176307, S-medium 0.177057
budget guard: running sum + 0.05 per launch checked against 2.50; never triggered (max spent 0.398657)
total_estimated_usd: 0.398657

## doubts
- Single run per condition (1 repeat each). Haiku misses are on 2 of 30 samples per effort level; n=30 is too small to separate H-high from H-xhigh.
- P3 expected label "small" conflicts with classifyPrompt rule ("version bump" listed under large), as noted in 261029-test-router-30-prod-args.md. Haiku's "large" on P3 follows the prompt rule; scored as expected label per spec.
- Haiku errors are all small->large (over-escalation); none large->small.
- Earlier Haiku default (prior run, 3 repeats) scored 29, 27, 28 of 30; these effort levels land in the same range (28/30). No effort-specific gain in accuracy is visible here.
- Sonnet low and medium are both 30/30 and about 8.9x more expensive per call than Haiku high (both low and medium); output tokens are much lower for Sonnet (about 112) than for Haiku at high (598) or xhigh (1025).
- Haiku xhigh costs about 28% more per call than Haiku high and uses about 71% more output tokens, with no accuracy gain.
- Effort values were not validated against the API beyond successful calls (all 120 returned rc=0 with a size line).
