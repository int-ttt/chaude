# 261029 parallel claude CLI test

precheck: ANTHROPIC_API_KEY unset, ANTHROPIC_AUTH_TOKEN unset, ANTHROPIC_BASE_URL unset, CLAUDE_CODE_USE_BEDROCK unset, CLAUDE_CODE_USE_VERTEX unset; claude --version 2.1.312 (PASS)
workdir: /tmp/partest.qY1zxF (not in repo)
cmd: env -u ANTHROPIC_API_KEY claude -p "Reply with the word OK only." --model haiku --output-format json --max-turns 1 --max-budget-usd 0.20 --strict-mcp-config (timeout 120s via perl alarm)

baseline:
  start: 1793270842.103 (epoch s)
  end: 1793270845.171
  rc: 0
  total_cost_usd: 0.0042356
  duration_api_ms: 1751
  result_len: 2
  is_error: false

parallel (3 processes, own subdir each):
  p1: start 1793270848.135, end 1793270851.262, rc 0, cost 0.00254976, api_ms 1729, result_len 2
  p2: start 1793270848.138, end 1793270851.139, rc 0, cost 0.00254976, api_ms 1475, result_len 2
  p3: start 1793270848.142, end 1793270851.070, rc 0, cost 0.00254976, api_ms 1529, result_len 2

checks:
  all rc 0: yes
  overlap (latest start 848.142 < earliest end 851.070): yes
  combined wall: 3.127 s (min start to max end)
  sum of durations: 9.059 s (3.127 + 3.000 + 2.932)
  0.6 x sum: 5.435 s
  combined <= 0.6 x sum: yes

cost:
  baseline: 0.0042356
  parallel sum: 0.0076493
  running total: 0.0118849 (limit 2.50 stop, 3.00 budget, not reached)

verdict: PASS (parallel works)
retries: 0
repo edits: none, commits: none
