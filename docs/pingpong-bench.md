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
cost USD: $1.379 → $0.791 (-42.7%)
total tokens: +73.0% (including Haiku)

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

## effort 재측정 (2026-11-02): harness 메인 Opus effort unset / low / medium
date: 2026-11-02
scope: harness만 재측정 (2 runs x 3 prompts per level). plain 수치는 위 2026-11-01 결과 재사용 (plain 방식 변경 없음)
method: 동일 프롬프트·동일 run 수·동일 flags. harness 복사본(/tmp/bench-harness)의 .claude/settings.json에서 effortLevel만 바꿈 (unset = 키 삭제). 저장소 .claude/settings.json은 medium 그대로 (변경 없음)
raw: notes/runs/pingpong-raw/effort-{unset,low,medium}-r{1,2}-p{1,2,3}.json, games: notes/runs/pingpong-raw/games-effort-{level}-r{n}/pong.py
log: notes/runs/261102-effort-pingpong.md, status: logs/pingpong-effort.status
sums: 기존과 같음 (3 claude calls per run, modelUsage 합, total_cost_usd, num_turns, duration_ms)

## per-level means (n=2, 3프롬프트 합)
| effort | Opus 토큰 합 | Haiku 토큰 합 | 총 토큰 (Haiku 포함) | 비용 USD | turns | duration s |
|---|---|---|---|---|---|---|
| unset | 460,764 | 811,684 | 1,272,448 | 0.861 | 5.0 | 140.8 |
| low | 405,140 | 678,075 | 1,083,215 | 0.689 | 5.0 | 110.8 |
| medium | 501,455 | 576,726 | 1,078,181 | 0.786 | 4.5 | 93.2 |
| plain (2026-11-01, 재사용) | 787,628 (Opus only) | - | 787,628 | 1.379 | 22 | 120.2 |

per-run cost USD: unset 0.9544 / 0.7684, low 0.6894 / 0.6881, medium 0.7493 / 0.8229
per-run turns: unset 4 / 6, low 6 / 4, medium 6 / 3
per-run duration s: unset 101.6 / 180.0, low 137.2 / 84.5, medium 164.2 / 22.3

## diff vs plain (mean)
cost: unset -37.5%, low -50.0%, medium -43.0%
total tokens (incl Haiku): unset +61.6%, low +37.5%, medium +36.9%

## reading
- 이번 측정에서 가장 싼 것은 low였습니다. medium은 기존 측정(low, 비용 0.791)과 거의 같은 비용이었고, low 재측정(0.689)과도 차이가 작습니다.
- unset이 세 설정 중 비용·총 토큰 모두 가장 큽니다 (n=2라 확정 불가).
- 비용 분산이 큽니다 (medium r2 0.823 vs r1 0.749). 차이 해석은 표본이 작아 조심해야 합니다.
- 재측정 low의 cost (0.689) vs 2026-11-01 low (0.791): 같은 설정인데 약 15% 차이. 캐시 상태 영향으로 보입니다.

## quality / smoke (static, not pty)
games-effort-{level}-r{n}: 6개 모두 py_compile OK, q 종료 키 문자열 있음.
score 표시: 넓은 검색(score|점수)은 medium-r2 를 제외한 5개에서 줄이 잡혔고, 엄격한 검색(addstr에 score 표시)은 unset-r2, medium-r1 만 잡혔습니다. medium-r2 는 score|점수 텍스트가 0줄입니다. 점수 표시 충족 여부는 pty 확인이 필요합니다.
permission_denials: unset 3 + 7, low 6 + 4, medium 6 + 0. 기존 harness: r1 0, r2 10. 실행 중 Bash 등이 거부된 횟수라 작업 조건이 실제와 다를 수 있습니다.

## doubts
n=2 per level, 범위로만 해석
cache 상태에 따라 비용이 크게 움직입니다 (같은 low가 0.79 vs 0.69)
plain은 재측정하지 않았습니다 (재사용)
score 확인은 정규식이라 부정확합니다. pty 스모크는 이번에 하지 않았습니다.

