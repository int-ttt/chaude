# 261031 검토: plan-token-compare 개선안

대상: notes/runs/261031-plan-token-compare.md (실행하지 않음, 개선안만)
형식: 번호 | 근거 경로:행 | 수정 제안

## (a) 구성 정의와 훅 허용 경로
1 | plan:12-13, hook:45-49, LESSONS.md:5 | 구성 A/B를 모델명이 아니라 호출 트리의 루트로 다시 정의: A = (메인 또는 opus-architect) → haiku-*, B = 메인 → sonnet-analyst/sonnet-reviewer → haiku-* (opus→sonnet은 v0.8.2 이후 경로가 없음).
2 | plan:22, hook:45 | "opus-architect(서브) → sonnet 거부" 근거는 SUB_CALLERS에 opus-architect가 들어 있어야 성립함. hook 1-29행의 SUB_CALLERS·BUILTIN 정의를 읽고 표에 행번호를 추가.
3 | DECISIONS.md:35, DECISIONS.md:43 | 10-29에는 서브→sonnet-reviewer 호출이 허용됐고 10-30 v0.8.2에서 번복됨. out.txt 기간(out.txt:4, 10-28~31)이 정책 변경을 가로지르므로 v0.8.2 커밋 시각 전/후로 transcript를 나누고, 실측 구성 B(opus→sonnet)가 변경 전 데이터에 있는지 확인하는 단계를 추가.
4 | weekly-usage:13-16, out.txt:131 | 메인 턴의 96%가 haiku 모델이므로 구성 A의 "opus"가 메인인지 opus-architect인지 구분. 메인 모델(stage=turn의 model)을 구성 정의의 축으로 함께 기록.

## (b) 비교 지표
5 | plan:31-33, plan:41-53 | 4종 단순 합계는 캐시 읽기가 지배함(캐시 읽기 비중: haiku-* 75.0% = 61,810,931/82,450,673, sonnet-* 82.5% = 39,317,369/47,650,120). "비캐시 합계(입력+출력+캐시 생성)"를 별도 열로 추가(haiku-* 20,639,742, sonnet-* 8,332,751, 비율 40.4%).
6 | FACTS.md:6-10 | 토큰 수와 사용량은 단가가 달라 다름(sonnet은 haiku 대비 입력·출력 약 20배, 캐시 읽기 10배). FACTS.md 열 정의대로 종류별 단가 가중 합계를 보조 지표로 추가하고, 100k 초과 단가 적용 여부는 out.txt:13(서브 ctx>100k 0건)을 근거로 명시.
7 | plan:76-88 | sonnet-* 대 haiku-* 에이전트 타입 비교는 구성 비교가 아님(B의 비용에는 sonnet 아래 haiku 호출이 포함됨). 비교 단위를 "루트 작업 1건 = 루트 에이전트 + 모든 하위 transcript 합계"로 바꾸고, 타입 비교는 참고로 강등.
8 | out.txt:9 | 호출당 분포가 한쪽으로 크게 치우침(scout 평균 70,232, 중앙값 42,249.5, 최대 630,590). 평균 대신 중앙값과 사분위 범위를 주 지표로 쓰고, 턴 수(msgs_ded) 차이는 "assistant 메시지 1건당 비캐시 토큰"으로 보정한 값을 함께 기록.
9 | out.txt:131 | 루트(메인 174 transcript, opus-architect 79건)의 자체 토큰을 구성 합계에 포함. 지금 표는 하위 에이전트만 합산해 구성 A의 opus 비용이 빠짐.

## (c) ±10% 판정 기준
10 | plan:74, out.txt:9 | 분포 꼬리가 길어 표본 평균의 흔들림이 10%를 넘을 수 있음. 루트 작업 단위 비율의 부트스트랩 95% 신뢰구간이 [0.9, 1.1] 안이면 "비슷함", 밖이면 "다름", 걸치면 "판정 불가"로 바꾸고 표본 수 n을 같이 기록.
11 | plan:75, plan:82 | "합계 비율"(57.8%)은 호출 수(895 대 287)에 좌우되고 효율을 나타내지 않으므로 판정 조건에서 빼고 참고값으로 둠.
12 | plan:74 | 토큰 기준과 단가 가중 기준의 판정이 갈릴 수 있으므로 두 판정을 따로 표기하고, 사용자 질문("토큰 소모")에 대한 주 판정은 토큰 기준이라고 명시.

## (d) 집계 기준 차이
13 | plan:36, plan:64, plan:105 | dedup 방식에 따라 비율이 57.8%(dedup)와 46.9%(raw)로 11%p 달라져 ±10% 기준보다 큼. raw/dedup 배수도 타입마다 다름(haiku-* 2.22, sonnet-analyst 1.80). 판정 전에 out.txt 생성 스크립트의 dedup 키(예: message.id)를 확인하는 단계를 맨 앞에 둠.
14 | plan:68, weekly-usage:16, weekly-usage:32 | 턴 레코드 모델별 수치는 메인 턴 모델이라 서브 구성과 대응하지 않음. 판정 근거에서 제외한다고 4절에 명시.
15 | usage-distribution:6, weekly-usage:10, weekly-usage:44 | 세션 최댓값 합(62.71 USD)과 cost_usd 합계(602.78 USD)는 누적값/턴별값 여부가 미확인이라 서로 섞지 않음. 비용 수치는 이 비교에서 사용 금지로 명시.
16 | out.txt:4, plan:102-104, weekly-usage:41 | 모든 수치를 같은 UTC 구간으로 자름: transcript별 첫 ts로 날짜를 붙이고, 10-31은 집계 기준 시각(out.txt 생성 시각)을 기록해 스냅샷임을 표시. 두 답변 파일 간 레코드 1~12건 차이(10-29~31 구간)는 원인 미확인으로 남기고 판정에 쓰지 않음.

## (e) 실행 단계 검증 가능성
17 | plan:93-94 | 1단계 "haiku-scout로 표 재정리"는 수기 전사 오류 위험이 있음. 원본 transcript jsonl에서 python3 -I 스크립트로 직접 재집계하고, 스크립트는 scripts/ 아래(예: scripts/token_compare.py), 출력은 notes/runs/261031-token-compare-data.json으로 고정.
18 | plan:4, plan:35 | /tmp/scoutagg/out.txt와 그 생성 스크립트는 재부팅 시 사라질 수 있음. 판정 전에 notes/runs/ 또는 scripts/로 복사하고 경로를 계획서에 기록.
19 | plan:95, plan:89 | 3단계 수용 조건을 숫자로 정함: A/B/미분류 transcript 수의 합 = 1,262, parentAgentId 없는 532건(1,262-730)의 처리 규칙(루트 메인으로 간주 여부)을 명시, 미분류 비율이 기준(예: 20%)을 넘으면 "구성 단위 미확인" 유지.
20 | plan:93-97 | 순서 변경: dedup 키 확인(13) → 재집계 스크립트(17) → 정책 전/후 분할(3) → 트리 분리(19) → 판정(10). 각 단계 뒤에 haiku-checker가 3-1 표 합계(1,262건, 82,450,673, 47,650,120)와 정수 단위로 일치하는지 확인.
21 | plan:97 | 판정 파일 경로를 notes/runs/261031-answer-token-compare.md로 고정하고, 결과 문서에 [확실]/[잠정]/미확인 표기를 지표마다 붙이도록 명시.

항목 수: 21
