# 사용 패턴 분석: 2026-09-01 ~ 2026-10-31

근거 파일: notes/runs/261029-usage-aggregate-0901-1031.md (transcript), notes/runs/261029-usage-aggregate-metrics-0901-1031.md (metrics/설정). 비교 참고: 261029-usage-aggregate-pre1027.md, 261029-usage-aggregate.md.

## 1. 데이터 범위와 제외 항목

- 날짜 기준: **UTC**, 각 레코드의 timestamp 날짜로 필터링했습니다. (KST는 +9시간이라 KST 날짜와 하루 어긋날 수 있습니다.)
- 요청 범위: 09-01 ~ 10-31. **실제 데이터 범위: 2026-09-08T01:20Z ~ 2026-10-29T06:34Z**. 09-01~09-07과 10-30~10-31은 레코드가 없습니다. (오늘이 10-29이므로 10-30, 31은 미래입니다.)
- transcript: jsonl 파일 540개, 전체 49,582줄 중 **유효 레코드 38,134건**(timestamp 있고 기간 내, 모든 type 포함), 세션(고유 sessionId) 175개. 제외: timestamp 없는 레코드 11,448건. 파싱 실패 0건, 기간 밖 0건.
- metrics: 날짜 범위에 드는 파일은 2개뿐(2026-10-28, 10-29). 레코드 **313건**(84 + 229). 10-29 파일은 작성 중이라 스냅샷입니다. 09-01~10-27 metrics 파일은 없습니다.
- 설정(settings.json): **날짜 필터 없음**, model/permission/effort 관련 키만. effortLevel=medium, permissions.defaultMode=auto, model 키 없음, autoMode 존재(내용 미복사). env/토큰 등은 읽지 않았습니다.
- 프롬프트 원문은 포함하지 않았습니다(집계만).

## 2. 사용 패턴

### 날짜 분포 (레코드 수 = 모든 type의 JSON 줄)
- 활동 일수는 기간 중 소수입니다. 09월: 09-08~13, 16, 21, 24, 28, 30 (일별 5~1,349건). 10월 초: 10-02, 03, 06, 07, 09, 10, 24 (일별 22~1,225건).
- 10-27 이후가 폭증: 10-27 2,357 / **10-28 22,084** / 10-29 7,067. 이 3일이 전체의 82.6%(31,508건)입니다.

### 시간대 (UTC, 전체 레코드 수)
- 피크: 01시 5,325, 14시 3,683, 15시 3,375, 09시 3,068, 02시 3,027. 20~22시는 0건, 19시 105건.
- KST로 환산하면(추정) 01Z=10시, 14~15Z=23~24시, 09Z=18시, 02Z=11시 정도로, 오전·저녁·심야 세 덩어리입니다. 일 단위로 몰린 날(10-28)의 영향이 커서 시간대 분포 자체가 그날 패턴에 좌우됩니다(추정).

### 프로젝트 분포 (레코드 수, projects/ 하위 첫 디렉터리)
- claude-harness 18,216 (47.8%), VocalAnalyze 6,289, Documents/univ 5,010, TextGame 3,654, claude-lean 2,748.
- 그 외는 harness 하위 테스트 디렉터리(e2e hang-tui 406, e2e-rebuild 343 등)와 /private/tmp 의 e2e/scratchpad 임시 작업들입니다. 상위 5개 외에는 각 400건 이하입니다.

### 모델 분포 (assistant 레코드 수 / 중복 제거한 message id 수)
- sonnet-5-5 3,414 / 1,932, haiku-5-5 3,326 / 1,798, opus-5-5 1,565 / 845, opus-5 1,330 / 740, sonnet-5 1,155 / 623, fable-5 217 / 109, synthetic 19.
- 스트리밍 중복 때문에 레코드 수가 message id 수의 약 1.8배입니다. 실제 호출 규모는 id 기준(합계 약 6,050)이 맞습니다(추정).
- 세대 구분이 뚜렷합니다. opus-5 / sonnet-5 / fable-5는 10-26 이전에만, sonnet-5-5 / haiku-5-5 / opus-5-5는 거의 10-27 이후에 나옵니다(아래 6절).
- metrics의 라우팅(classify 72건): haiku 50, opus 14, sonnet 8. 즉 하네스 라우터가 haiku로 보낸 비중이 약 69%입니다. 모델 키가 있는 turn 레코드도 haiku 107, opus 14, sonnet 8.

### 도구 사용 (distinct tool_use id 수)
- Bash 3,543 (압도적), Read 583, SubagentHandback 428, Agent 370, Write 178, Edit 118, SendMessage 76, WebFetch 53, WebSearch 36, ScheduleWakeup 24, ToolSearch 16. Grep 5.
- Agent 370 / SubagentHandback 428 / SendMessage 76은 위임 구조 사용이 많다는 뜻입니다. Read/Edit/Write 대비 Bash가 6배 이상이라 Bash로 파일 조작을 하는 비중이 높다고 추정됩니다(추정).

## 3. 비용과 컨텍스트 패턴

- 토큰은 assistant 레코드의 usage 합입니다. dedup(message id당 첫 레코드) 기준 전체: output 약 4.82M, cache_read 약 811.9M, cache_creation 약 34.0M, 비캐시 input은 약 1.3만으로 거의 0입니다. 즉 입력의 사실상 전부가 캐시를 거칩니다.
- 모델별 dedup 합 cache_read: opus-5 284.9M, sonnet-5 264.2M, fable-5 82.2M, sonnet-5-5 78.0M, opus-5-5 54.2M, haiku-5-5 48.4M.
- 호출당 평균 cache_read(dedup cache_read ÷ message id 수, 추정 계산): opus-5 약 385k, sonnet-5 약 424k, fable-5 약 754k, sonnet-5-5 약 40k, opus-5-5 약 64k, haiku-5-5 약 27k. 10-26 이전 모델은 호출당 컨텍스트가 훨씬 컸습니다.
- 달러 비용: transcript에는 비용 필드가 없어 알 수 없습니다. metrics의 cost_usd 합은 **82.61달러**(숫자값 194건, null 6건 제외 → 하한). 태그별(model 키 기준): haiku 69.79, sonnet 3.81, opus 7.40. 이 비용은 10-28~29 이틀치뿐입니다. haiku 태그 비용이 비중상 큰 것은 conductor 결과가 haiku 키로 묶인 영향일 수 있으나 확인하지 못했습니다(추정).
- metrics 토큰(result_*, conductor 결과 200건): input 642, output 178,983, cache_creation 1,255,410, cache_read 12,255,800. 처리 시간 합 7,355,790ms(약 2.04시간, 171건).
- 하네스 동작: rotate 15건(closed 14, drained 1), rotations 합 183, delegations 합 91, conductor=True 147 / False 44.

## 4. 10-27 이후와 09-01~10-26 비교

(이번 집계의 분할 값이며, 이전 요약 pre1027과 레코드 수 6,626건이 일치해 재확인되었습니다.)

| 항목 | 09-01~10-26 | 10-27~10-31(실제 ~10-29) |
|---|---|---|
| 레코드 | 6,626 | 31,508 |
| user / assistant | 1,605 / 2,799 | 6,255 / 8,227 |
| 세션 수 | 6 | 169 |
| 활동 일수 | 18일 | 3일 |
| 모델 | opus-5, sonnet-5, fable-5 (소량의 5-5) | sonnet-5-5, haiku-5-5, opus-5-5 |
| Bash tool_use | 1,069 | 2,474 |
| Agent / SubagentHandback | 0 | 370 / 428 |
| metrics 레코드 | 0 | 313 |

- 10-27 이후가 레코드의 82.6%이며, 일평균으로는 약 10,500건 대 약 370건(18일 기준)으로 약 28배입니다(추정 계산).
- 이전 구간은 큰 컨텍스트의 소수 세션(6개)이 장시간 긴 호출을, 이후 구간은 서브에이전트가 많은 짧은 세션(169개)을 만들었습니다. 이는 위임(Agent) 구조 도입과 일치합니다.
- 이전 구간 Bash 1,069 중 상당수가 ScheduleWakeup(24)을 쓰는 반복/대기형 작업이었을 가능성이 있습니다(추정).
- 세션 수: 이전 요약 pre1027은 7이라고 했으나 이번 집계는 6입니다. 이전 요약은 uuid 중복 제거를 했고, 이번에는 sessionId 없는 레코드 9건이 있어 세션 계산 방식 차이로 보입니다(추정).

## 5. 해석 (모두 추정)

1. 사용이 10-27 이후로 급격히 집중되었습니다. 이는 하네스(claude-harness) 개발 작업과 에이전트 위임 구조가 본격화된 시기와 맞습니다. 사용자의 이전 평소 패턴이 아니라 이 3일을 기준으로 전체를 판단하면 왜곡됩니다.
2. 10-26 이전은 호출당 컨텍스트가 매우 커서(호출당 cache_read 약 400k 이상) 비용 압박이 컸고, 이후 작은 컨텍스트로 쪼개 위임하는 방식(호출당 약 30~60k)으로 바뀐 것으로 보입니다. 호출 수는 늘었지만 호출당 컨텍스트는 크게 줄었습니다.
3. 모델 선택이 haiku 중심(라우팅 약 69%)이면서, 총 cache_read 기준으로는 아직 이전 세대 opus/sonnet 세션이 후반 구간 전체보다 큰 비중(662.8M 대 149.2M)입니다.
4. 도구는 Bash 중심이라, 읽기·편집을 별도 도구로 쓰는 비중은 낮습니다.

## 6. 불확실한 점

- 09-01~09-07 레코드가 없는 것은 실제 미사용인지, transcript가 정리/삭제된 것인지 알 수 없습니다. timestamp 없는 11,448건(전체의 23%)이 제외되었는데, 이들이 어느 날짜 활동인지 알 수 없습니다.
- 이전 구간 18일은 6개 세션으로 구성되어 표본이 작습니다. 이전 구간 비교는 신뢰도가 낮습니다.
- 레코드 수는 type 혼합(attachment, api-request 등 포함)이라 "활동량" 대용치일 뿐입니다. message id 기준이 더 정확하나 날짜별로 집계하지 않았습니다.
- 달러 비용은 이틀치 metrics뿐이며, null 6건 제외로 하한입니다. 10-27 transcript와 metrics의 시간 겹침 구간도 하루(10-27)는 metrics가 없습니다.
- 10-29 데이터와 metrics 파일은 작성 중이라 스냅샷입니다.
- 모델 이름(claude-sonnet-5-5 등)은 기록된 문자열 그대로이며, 요금 정보는 이 분석에서 사용하지 않았습니다.
- UTC 기준이므로 KST 해석은 모두 환산 추정입니다.
- 호출당 평균 컨텍스트는 dedup cache_read ÷ id 수로 계산한 근사치입니다. message id 첫 레코드의 usage가 최종값이 아닐 수 있습니다(스트리밍 중간값 가능).
