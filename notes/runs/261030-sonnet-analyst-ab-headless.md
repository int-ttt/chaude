task: create scripts/ab_headless.py (run, report, selftest) per plan v2 sections "tool: scripts/ab_headless.py" + "review merged"
changed: scripts/ab_headless.py (new, 280 lines, untracked)
cmd: python3 scripts/ab_headless.py selftest
out: selftest ok (rc=0)
cmd: python3 -m py_compile scripts/ab_headless.py
out: (no output, rc=0)
cmd: python3 scripts/ab_headless.py report /tmp/abfake.jsonl --a A --b B
out:
arm A: n=2 errors=0 correct=2 unique_sha=2
  cost_usd mean=0.11 sd=0.0141421 min=0.1 max=0.12
  num_turns mean=2.5
  total_cache_creation_tokens=110
  model claude-haiku-5-5 mean: cacheCreation=55 cacheRead=105 costUSD=0.11
arm B: n=2 errors=0 correct=1 unique_sha=2
  cost_usd mean=0.21 sd=0.0141421 min=0.2 max=0.22
  num_turns mean=4.5
  total_cache_creation_tokens=250
  model claude-haiku-5-5 mean: cacheCreation=125 cacheRead=205 costUSD=0.21
ratio b/a: cost_mean=1.90909 cache_creation_total=2.27273
mann-whitney exact two-sided p (total_cost_usd)=0.333333
fake input: /tmp/abfake.jsonl (4 rows, no warmup)
check: MW 2v2 hand-computed = 2/6 = 0.333333 (match)
not run: run subcommand against real claude (not executed)
doubts: idx is 1-based; error rows cost=0; claude --version record and early-stop rules not implemented (orchestration, not in tool spec)
