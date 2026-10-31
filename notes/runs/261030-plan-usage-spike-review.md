# review: notes/runs/261030-plan-usage-spike.md
reviewer: opus-architect | date: 261030 | mode: review only, not executed
verdict: 방향은 맞으나 증거 대부분이 사용자 구간 밖. 검증 단계(§3)와 완료 기준(§6)을 구간 데이터 확보 중심으로 재배치해야 함.

## R1 [critical] 구간 커버리지를 검증 전제조건으로 격상
- where: §0, §3 (new step 0), §6
- basis: 사용자 구간 KST 00:20-05:20 = UTC 10-29 15:20-20:20 (300분). metrics 마지막 ts 16:36:27Z -> 구간 중 76분(약 25%)만 존재, 16:36-20:20Z 224분은 metrics 0행. 2026-10-30.jsonl 없음.
- change: §3 맨 앞에 step 0 추가 "구간 커버리지 확정": (a) 구간 내 metrics 행 수와 분 단위 커버리지(%)를 표로 기록, (b) 16:36Z 이후 metrics가 끊긴 원인 확인(하네스 종료? 15:09/15:11Z exit 1 직후 다른 실행 경로로 전환? 파일명 날짜 기준 UTC vs local 버그?) — 해당 코드: metrics 파일명/append 경로(repl.go metrics writer), (c) 보조 출처로 구간 사용량 복원: ~/.claude/projects/<project-slug>/*.jsonl transcript의 message.usage(input/cache_creation/cache_read/output, model, timestamp), notes/LEDGER.md (ledger.py 기록), notes/metrics/2026-10-28.jsonl.
- acceptance: 구간 300분 중 사용량 출처가 확보된 분(%)을 출처별로 명시. 70% 미만이면 원인 결론을 "추정"으로 표기.

## R2 [critical] 가설 근거가 구간 밖임을 명시하고 분리
- where: §1-2, §1-3, §2, §4 우선순위
- basis: H1 rotate 11건 UTC 00:07-01:00 = KST 09:07-10:00; H2 12.79 USD 턴 12:14Z = KST 21:14; 최대 비용 UTC 10-11시 = KST 19-20시. 모두 사용자 구간 밖. 구간 내 확인된 metrics 비용은 UTC 15-16시 합 약 10.46 USD뿐.
- change: §1/§2 각 항목에 tag 추가 "in-window | out-of-window". H1/H2/S1/S2는 "상시 낭비 요인"으로 재분류하고, "폭증 원인"으로는 R1 step 0 결과로 확인된 가설만 인정. §4 우선순위 "S2 -> S3 -> S1"은 R1 결과 전까지 보류 표기.

## R3 [high] 가설 H6 추가: 하네스 metrics 밖의 사용(대화형 Claude Code 세션/하위 에이전트)
- where: §2 (H6 신규), §3 (step 6 확장), §6
- basis: 10-29 notes/runs/261029-* 분석/탐색 파일이 20개 이상 생성됨(git status) -> 같은 시기 대화형 세션에서 opus/sonnet 하위 에이전트가 다수 실행됐을 가능성. 이 사용은 notes/metrics에 기록되지 않음. H5(Task usage 부모 포함 여부)와는 별개.
- change: H6 "구간 사용량의 상당 부분이 metrics 미기록 세션(대화형 Claude Code, Agent tool 하위 에이전트)에서 발생". 검증: transcript jsonl을 timestamp로 구간 필터 -> session/model/agent별 토큰 합계와 FACTS.md 단가로 USD 환산. 해결 후보에 "S7 대화형 세션 계측(ledger.py 또는 별도 hook에 usage 기록)" 추가.

## R4 [high] "사용량" 지표를 5h 한도 %로 우선 가정하고 확인
- where: §8 -> §3 step 1, §6
- basis: 구간 길이가 정확히 5시간이고 상태줄에 "5h rate usage"가 표시됨(commit e3c499d). 사용자가 본 "폭증"은 USD가 아니라 5h 한도 %일 가능성 높음. 현재 계획의 지표는 total_cost_usd 기준.
- change: §3 step 1에 "상태줄 5h rate usage 값의 출처와 로그 여부 확인(statusline 스크립트/settings.json)" 추가. 기록이 없으면 앞으로 5h %를 metrics에 남기는 작업을 S3에 포함. §6 완료 기준에 "동일 KST 00:20-05:20 창에서 5h 한도 % 비교" 추가.

## R5 [medium] step 1의 담당 오류: 사용자 질문은 scout가 할 수 없음
- where: §3 step 1, §5-4
- basis: haiku-scout는 읽기 전용이며 사용자와 대화하지 않음.
- change: step 1을 둘로 분리. (a) 사용자 질문(메인 세션이 전달): "폭증을 확인하신 날짜(10-29 새벽 또는 10-30 새벽)와 화면(상태줄 5h %, claude.ai 사용량, 콘솔 요금)" — §8에 이동. (b) haiku-scout: 두 후보 창 모두 기계적으로 집계 — 후보A UTC 10-28 15:20-20:20 (2026-10-28.jsonl), 후보B UTC 10-29 15:20-20:20 (2026-10-29.jsonl + transcript). 사용자 답을 기다리지 않고 병렬 수행.

## R6 [medium] 완료 기준과 재측정의 비교 창 고정
- where: §6, §5-11
- basis: "24시간 비용 감소"는 낮 시간대(UTC 10-11시 229 USD)가 지배하므로 새벽 구간 개선 여부를 보여주지 못함. 또 metrics가 끊기면 비교 불가.
- change: §5-11을 "다음 KST 00:20-05:20 창 + 24시간 전체" 두 값으로 측정. 사전 조건: 해당 창의 metrics 커버리지 >=95% (R1 원인 수정 후). §6에 "구간 metrics 행 누락 0분" 추가.

## R7 [low] null 비용 추정 방법 명시
- where: §3 step 5, S3
- basis: null 23건은 duration 300002ms이고 첫 행 기준 result 토큰도 0 -> metrics만으로 비용 추정 불가.
- change: step 5에 "해당 session의 transcript에서 같은 시각 assistant message.usage를 찾아 FACTS.md 단가로 환산" 명시. S3의 '추정비용'도 같은 방법으로 정의.

## R8 [low] 코드 줄번호 고정 위험
- where: §1-4, §5-6
- basis: repl.go 883-887 등 줄번호는 S3 구현(5-5) 이후 이동함.
- change: 줄번호 대신 식별자 병기(opusSonnetThreshold, idleDeadline, MaxAutoRotations, 회전 판정 함수명). 5-6은 5-5 이후 줄번호 재확인.

## ordering (revised §3/§5)
1. R1 step 0 커버리지/출처 확보 + R5(b) 두 후보 창 집계 [haiku-scout, 병렬]
2. R5(a) 사용자 질문 [메인 전달, 병렬]
3. 기존 step 2-6 (in-window 데이터 우선) + H6 transcript 집계 [haiku-scout]
4. step 7 기여도 표 (in/out-of-window 분리 열, 출처 커버리지 %) [sonnet-analyst]
5. 이후 §5-4 ~ §5-11 유지, §5-11은 R6 기준

## must NOT change
- §7 롤백 원칙, "확정 전 해결 코드 금지", metrics 필드 추가만 허용.
