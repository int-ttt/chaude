date: 2026-11-01
task: 효율 테스트 핑퐁 게임 (plain vs harness)
model: opus (main), claude-haiku-5-5 (subagent, harness only)
runs: 2 per method, 3 prompts each (sequential, --resume for p2,p3)
raw: notes/runs/pingpong-raw/r{1,2}-{plain,harness}-p{1,2,3}.json
games: notes/runs/pingpong-raw/games-r{1,2}-{plain,harness}/pong.py
status log: logs/pingpong-bench.status

## method
prompts:
  p1: 터미널 핑퐁게임 만들어줘
  p2: 공 속도 좀 빠르게 하고 점수 표시해
  p3: q 누르면 종료되게
plain: cwd /tmp/bench-plain (empty), claude -p --model opus --output-format json --permission-mode acceptEdits
harness: cwd /tmp/bench-harness (copy of CLAUDE.md, .claude/ (agents, hooks, settings.json), notes/ skeleton without runs/pastes/research/metrics/deprecated), same flags
harness setup: platform_setup/install.sh has no target-dir option, so the copy was made by hand
sums: per run = sum of 3 claude calls; tokens from modelUsage (inputTokens, outputTokens, cacheReadInputTokens, cacheCreationInputTokens); cost = total_cost_usd; turns = num_turns; duration = duration_ms
caveat-env: --permission-mode acceptEdits (no bypass); Bash smoke-tests were denied in -p mode for plain r1 (permission_denials) and likely elsewhere; global ~/.claude config not isolated

## per-run sums (raw modelUsage)
r1-plain: calls 3, turns 17, dur_s 96.4, cost 1.3104
  opus: in 40, out 21616, cache_read 483143, cache_create 97658
r2-plain: calls 3, turns 27, dur_s 144.0, cost 1.4471
  opus: in 62, out 32148, cache_read 861644, cache_create 78944
r1-harness: calls 3, turns 3, dur_s 20.7, cost 0.9005
  opus: in 52, out 7687, cache_read 530570, cache_create 74778
  haiku: in 80, out 51022, cache_read 665507, cache_create 80102
r2-harness: calls 3, turns 6, dur_s 185.6, cost 0.6806
  opus: in 24, out 5968, cache_read 290255, cache_create 55885
  haiku: in 106, out 70662, cache_read 790650, cache_create 102507

## means (n=2)
plain: opus in 51, out 26882, cache_read 672393.5, cache_create 88301, total 787627.5, cost 1.37875, turns 22, dur_s 120.2
harness opus: in 38, out 6827.5, cache_read 410412.5, cache_create 65331.5, total 482609.5
harness haiku: in 93, out 60842, cache_read 728078.5, cache_create 91304.5, total 880318
harness all: total 1362927.5, cost 0.79055, turns 4.5, dur_s 103.2

## diff (harness vs plain, mean)
cost USD: -42.7%
opus total tokens: -38.7%
opus output: -74.6%
opus cache_read: -39.0%
all-model total tokens: +73.0% (harness subagent haiku tokens add to total)

## verify (headless pty smoke, compile + run + q exit)
games-r1-plain: compile OK, exit 0, score text found
games-r2-plain: compile OK, exit 0, score text found
games-r1-harness: compile OK, exit 0, score text found
games-r2-harness: compile OK, exit 0, score text found
plain games have a menu: smoke sent '1' first

## doubts
n=2 per method, small
cost depends on cache state (cache_read dominates cost)
harness settings include effortLevel low and subagent haiku; plain has no CLAUDE.md
game quality not measured beyond smoke test
score-text check is a regex, crude
