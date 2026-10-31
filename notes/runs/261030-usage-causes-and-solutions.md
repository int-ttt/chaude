# usage causes and solutions (261030)
## code change (facts)
changed: internal/harness/repl.go (setRate logs {"stage":"rate","five_hour":v} to metrics when five_hour utilization changes; fields lastFiveHour, fiveHourSeen), internal/harness/ratestatus_test.go (TestSetRateLogsOnFiveHourChange: same value twice=1 row, different=2 rows)
verify: gofmt clean; go vet ./internal/harness/ exit 0; go build ./... exit 0; go test ./internal/harness/... ok chaude/internal/harness 0.616s
caveats: only unifiedWindows.five_hour path (no top-level fallback like fmtRateStatus); raw utilization logged (not normalized %); new fields unguarded like r.rate, setRate also called from live child goroutine (livechild.go:42), -race not run; no commit
## measurement (facts)
script: /tmp/claude-usage/measure.py (outside repo); m0 argv NOT recorded; m1 args assumed: ~/.claude/projects/*claude-harness*/ + notes/metrics (metrics files=2 matched m0)
M0 (Oct29 17:33 UTC): TOTAL 10-29 msgs=3491 usd=49.58 | 10-30 msgs=758 usd=4.29 | sessions=31 sum_of_per_session_max=40.0882 | files scanned 571 (subagent 497) | spawns 10-29 384, 10-30 113
M1: TOTAL 10-29 msgs=3491 usd=49.58 | 10-30 msgs=839 usd=4.88 | sum_of_per_session_max=40.6317 | files 585 (subagent 511)
M1-M0: 10-29 unchanged (+0); 10-30 +81 msgs +0.59 usd; sum_of_per_session_max +0.5435 (session fff5ed0a gained metrics rows n=1 -> 14); +14 transcript files
what counts: usd = token-based estimate from transcripts using hardcoded PRICES table (opus 4/5/8/0.20/20, sonnet 2/2.5/4/0.1/10, haiku 0.10/0.125/0.20/0.01/0.50; haiku >100k total uses HAIKU_BIG); deduped by message id. sum_of_per_session_max = sum over 31 sessions of max cost_usd in notes/metrics (harness-reported cumulative).
largest rows 10-29 (estimate): sonnet-analyst 20.68, opus-architect 13.40, haiku-scout 0.57
## causes
fact: 10-29 spend concentrated in sonnet-analyst (20.68) and opus-architect (13.40) subagents, ~34 of 49.58 usd (~69%)
fact: subagent spawn volume high (384 on 10-29, 113 on 10-30); 497-511 of 571-585 transcript files are subagents
estimate: each spawn pays a cold first-turn cache creation (script computes mean/median per subagent first cache_creation; value not extracted here) -> many spawns multiply cache-write cost
estimate: expensive-model orchestration layers (opus/sonnet analysts delegating to haiku) cost more than the haiku work they delegate
## solutions ranked by expected effect (all inference, not measured)
1 reduce sonnet-analyst/opus-architect invocations (route only complexity>=3 there; direct haiku/main for simple) - largest share
2 cut spawn count (batch several files per scout/coder, reuse same instance) - cuts cold cache writes
3 keep prompts/briefs small and stable to keep cache hits (cache TTL observe exists in 0.8.0)
4 use the new stage=rate metric rows to correlate 5h utilization with turns/spawns (measurement enabler, no direct saving)
## unresolved
U1: 3.34 usd (per-session max, delta) vs 40.0882/40.6317 (31-session sum of per-session max): 3.34 appears in neither m0 nor m1; cannot reconcile. Largest per-session max-min values: 83fc90cf 12.7792 (n=49), 430496f9 4.0812 (n=8), 17d8f798 2.9193 (n=19). Needs the source of the 3.34 figure (which command/session/window).
U2: transcript estimate (49.58+4.29=53.87 over two days) vs metrics sum 40.09: different bases (token-price estimate vs harness cost_usd, different session coverage); gap 13.78 not attributed
U3: m0 argv unrecorded, so M0 vs M1 comparability assumed (file/metrics counts consistent)
U4: measure.py metrics path filter inconsistent ("notes/metrics" vs "/metrics/", lines 55/57); hours shown in local time
U5: no race run on setRate; per-spawn first cache_creation numbers not extracted

## 최종 갱신 (10-30)
setRate: was unlocked (HEAD), now rateMu guards r.rate, lastFiveHour, fiveHourSeen in setRate (repl.go:205-212) and usageLine read (repl.go:623-625); r.metrics.Log called after unlock; no nested lock; new fields no longer unguarded
measure.py: no five_hour path (five_hour only in internal/harness/repl.go setRate and ratestatus.go), metrics filter fixed (line 55 now "/metrics/" matches line 57)
U1: 3.34 = plan-usage-window per-session max for the Oct 29 window (delta 3.18; naive sum 11.18), 13.78 = 53.87 (transcript token-price est, 10-29+10-30 m0) minus 40.09 (metrics per-session-max sum) = different bases/coverage, not comparable, not reconciled
ac45291bcac49b3fc: 확인 불가 (0 hits in notes/)
checks: gofmt -l internal/harness: no output, exit 0; go vet ./...: exit 0; go build ./...: exit 0; go test -count=1 ./internal/harness/...: ok chaude/internal/harness 0.590s, exit 0; go test -count=1 -race ./internal/harness/...: ok chaude/internal/harness 1.807s, exit 0; measure.py ast.parse: ok (not run)

## 2026-10-30 보강 (자율 실행 결과)

- 확인된 사실: setRate(repl.go)는 기존에 r.rate, lastFiveHour, fiveHourSeen을 잠금 없이 갱신했고, idle 고루틴(repl.go:277), 턴 경로(repl.go:546), parkedIdleEvent(livechild.go:42)에서 동시에 호출될 수 있었습니다. rateMu 뮤텍스를 추가해 보호했고(metrics.Log는 잠금 밖에서 호출), usageLine 읽기도 복사본을 쓰게 했습니다. gofmt, go vet, go build ./..., go test ./internal/harness/..., go test -race ./internal/harness/... 모두 통과했습니다 (세부: notes/runs/261030-agent-ratemu-lock.md).
- 3.34 USD의 정체: 2026-10-29T15:20Z~17:03Z 창에서 세션 4개(bfc29c97 1.393, 82e672e7 0.9393, e66c18aa 0.8036, 0653d3a3 0.2004)의 cost_usd 최댓값 합(3.336)입니다. cost_usd는 프로세스 누적값이라 단순 합은 11.18 USD로 과대 계산됩니다. 같은 필터를 지금 파일 끝까지 적용하면 5.16 USD(7세션)가 되어, 3.34는 17:03Z 시점 스냅샷입니다. 기존 문서의 '27행'은 재계산 결과 22행으로 맞지 않아 미해결입니다.
- 13.78 USD의 정체: 측정된 값이 아니라 뺄셈 결과입니다. 트랜스크립트 토큰x단가 추정(10-29 49.58 + 10-30 4.29 = 53.87)에서 하니스 보고 cost_usd의 세션별 최댓값 합(40.09, 31세션, 이틀 전체)을 뺀 차이입니다. 3.34와는 기준과 기간이 달라 서로 대응되지 않습니다. 차이의 원인(단가표 가정, 서브에이전트 포함 범위, 세션 포함 범위)은 가설일 뿐 검증하지 못했습니다.
- measure.py 보완: 경로 필터(mtime 기반 건너뛰기)가 불필요/모순이어서 제거하고 메시지 타임스탬프로만 날짜를 거르게 했으며, stage=rate의 five_hour 시계열 섹션과 argv 헤더를 추가했습니다. m2 결과(notes/runs/261030-usage-m2.txt): 10-29 msgs=3406 usd=49.25, 10-30 msgs=917 usd=5.17, sessions=33, per-session-max 합 40.80. m1(49.58)과 10-29 값이 달라진 것은 필터 변경 영향으로 보이며 원인 확정은 못 했습니다.
- five_hour 기록: 코드는 값이 바뀔 때만 {stage:rate, five_hour:v}를 기록하도록 되어 있으나, 아직 새로 빌드한 바이너리로 세션을 돌리지 않아 metrics에 해당 행이 0개입니다. 따라서 사용량 %와 비용을 잇는 시계열은 아직 없고, 서브에이전트 비중이 원인이라는 판단은 여전히 추정입니다.
- 해결안 우선순위(추정): 1) five_hour 기록을 실제 세션에서 수집해 % 대비 USD 관계 확인, 2) 서브에이전트 모델 등급(Sonnet/Opus 하위 호출) 사용 억제, 3) 백그라운드 에이전트 장기 실행 점검.
- 미해결: 13.78 차이 귀속, 27행 vs 22행 불일치, five_hour 실측.
