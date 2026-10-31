# 261031 measure-results-2
plan: notes/runs/261031-measure-plan-tasks.md (written before runs); raw: /tmp/m261031/*.txt|json (temp, not in repo); scoring detail: notes/runs/261031-haiku-compare-m261031.md

## 1 resident-session limit
user claim: 기본 4, 최대 8
code: internal/harness/dispatch.go:12 DefaultMaxWorkers = 3; repl.go:1091 flag -max-workers default 3; env CHAUDE_MAX_WORKERS; only floor >=1 enforced (dispatch.go:16-30); no upper bound 8
DECISIONS.md/FACTS.md: no record of worker count (0 hits)
verdict: 불일치. 기본 3, 상한 8 없음. checkWorkerCap(dispatch.go:35-44) 비-test 호출자 없음 (dispatch_test.go:87,91만) -> 상한이 실제로 강제되는지 미확인
4 / 8 elsewhere: tui.go:1185 maxPanel=4, tui.go:838 maxPop=8 (UI 상수, 워커 수 아님, 미정밀 검증)
pickChild(dispatch.go:61): key=(cwd,model)당 child 1개 재사용 판정 (live warm -> actUseLive, parked warm -> actResume, else actStartNew); 개수 상한 아님
code not changed

## 2 test runs (caveman B·C), cwd /tmp/m261031, claude -p --tools "" --max-budget-usd 1 --no-session-persistence
cost per call (usd): T1-A opus 0.1951 out1147 | T1-C opus 0.0455 out1513 + haiku 0.0059 | T1-B haiku-compress 0.0010 + opus 0.1929 out2104 + haiku 0.0020 | T2-A opus 0.0364 out1129 | T2-C opus 0.0469 out1610 + haiku 0.0012 | T2-B compress 0.0010 + opus 0.1829 out1834 + haiku 0.0019
total: 0.7127 (cap 10, per-run cap 1 not reached)
opus output change vs A: T1 C +32%, B +83%; T2 C +43%, B +62% -> 절감 기준 30% 미달 (오히려 증가)
scoring (haiku-checker string match, A baseline): 근거 누락 T1-B 2, T1-C 3, T2-B 2, T2-C 1; 권고 부분일치 T1-B, T2-B; 환각 후보 T1-B (IDE 문장), T2-B, T1-C (270k 원인 추정), T2-C (89-90%)
verdict per plan: 기각 (근거 누락 >=1, 절감 미달)
uncertainty: n=1 per cell, no A repeat (noise baseline unknown); cost confounded by cache state (T1-A cold 0.1951 vs T2-A warm 0.0364; 비용 비교 무효); ~20k system prompt tokens remain with --tools ""; blind labels not used; scoring by haiku-checker, not sonnet-reviewer; inputs were fact packets, not file-reading tasks

## 3 M5 explanation
compares: 상주 child(한 프로세스가 맥락 유지, 후속 턴은 cache_read) vs 매번 새 생성(턴마다 cache_creation 재발생)
proxy used: haiku 첫 턴 cache_creation 중앙 27,492 vs 이후 2,941; opus 26,382 vs 3,322; 비용/턴 중앙 첫 0.0087 vs 이후 0.0392; background $0.129 (3.1s) vs foreground $0.128 (55s)
not direct because: 모든 데이터가 상주 방식에서 나옴; fresh 대조군 없음; "첫 턴 vs 이후 턴"은 세션 내 위치 차이이지 방식 차이 아님; 이후 턴 비용이 더 큰 건 맥락 누적 영향이라 교란; background/foreground는 방식이 아닌 실행 경로 차이
fields to add (metrics writer): repl.go:920-924 turn map (and background variant repl.go:425): child_mode ("resident"|"fresh"), pick_action (actUseLive|actResume|actStartNew from switchchild.go:155 pickChild result), child_turn_idx (child 내 턴 순번), child_age_s (child 생성 후 경과 s), child_session (session id), child_spawned_this_turn (bool). turnspend.go spawns only counts Agent/Task tool_use, not child process starts -> do not reuse
then M5 = group by child_mode: cache_creation, cost_usd, duration_ms per turn at equal child_turn_idx / ctx band. needs fresh-mode runs (flag to force actStartNew) to have data

## 4 user decision pending
workers default 3 vs 4 and cap 8: change default/add upper bound, or accept 3?
