# 261031 답변: 구성 A vs 구성 B 토큰 소모 비교 (집계 결과)

작성: 2026-10-31 (스냅샷, 실행 중인 세션 포함). 읽기 전용 집계. 저장소 파일 수정 없음. 스크립트는 /tmp/tokcmp/tokcmp.py (임시).
입력: /Users/leekwangseok/.claude/projects/-Users-leekwangseok-PycharmProjects-claude-harness/ 아래 루트 transcript 174건 (top-level *.jsonl) + 서브 transcript 1,281건 (<sid>/subagents/agent-*.jsonl, meta.json 포함).
출력 원본: /tmp/tokcmp/out.json (keys: groups, composite, boot, sessions).

## 1. 집계 단위와 구성 정의
- 단위: 루트 작업 1건 = 루트 transcript + 같은 <sid>/subagents 아래 모든 서브 transcript (parentAgentId 트리 전체).
- 구성 판정 (개선안 1 기준, 트리 기준):
  - A_strict: opus-architect가 있고 sonnet-* 없음.
  - A_review: sonnet-* 없음, 서브 존재 (opus-architect 또는 메인→haiku 단독). 개선안 정의 A = (메인 또는 opus-architect) → haiku-*.
  - B_strict: sonnet-* 있고 opus-architect 없음 (메인→sonnet→haiku).
  - B_incl_MIX: B_strict + MIX (sonnet과 opus 둘 다 있음).
  - MIX: sonnet과 opus 둘 다 있음.
  - NOSUB: 서브 없음 86건, 집계 제외.
- 시기: 루트 첫 timestamp의 UTC 날짜. pre = 10-28·10-29, post = 10-30·10-31 (사용자 지정 v0.8.2 10-30 기준). 주의: git bd950b7(0.8.2)은 커밋 시각 2026-10-31 15:50 +0900, 컷 날짜는 사용자 지정값을 그대로 씀.
- 토큰: input, output, cache_creation, cache_read 4종 합. "포함" = 4종 합, "제외" = 캐시 읽기 제외(input+output+cache_creation).
- dedup: transcript 파일별 message.id 첫 등장만 사용. 중복 미제거 값(raw)도 병기.
  - 검증: 같은 파일 내 중복 레코드 9,287건, usage 값이 다른 중복 0건. 파일 간 중복 message.id 0건. 따라서 첫 등장 기준과 전체 중복 제거 결과가 같음.

## 2. 구성별 결과 (dedup 기준, 토큰)

| 구성 | 루트 n (pre/post) | 서브 호출 수 | 합계 포함 | 합계 제외 | 루트당 중앙값 포함 | 루트당 중앙값 제외 | 호출당 평균 포함 | 캐시읽기 비중 | raw 합계 포함(중복 포함) |
|---|---|---|---|---|---|---|---|---|---|
| A_strict | 3 (2/1) | 25 | 4,975,266 | 912,916 | 2,363,109 | 254,747 | 199,011 | 81.7% | 9,112,497 |
| A_review | 25 (22/3) | 49 | 7,096,541 | 1,484,387 | 96,461 | 22,225 | 144,827 | 79.1% | 12,974,944 |
| B_strict | 34 (27/7) | 308 | 51,956,716 | 9,680,198 | 939,135 | 146,837 | 168,691 | 81.4% | 102,294,230 |
| B_incl_MIX | 63 (38/25) | 1,232 | 242,005,687 | 38,122,676 | 1,681,943 | 422,491 | 196,433 | 84.3% | 481,105,451 |
| MIX | 29 (11/18) | 924 | 190,048,971 | 28,442,478 | 4,204,211 | 839,495 | 205,681 | 85.0% | 378,811,221 |

- 출처: /tmp/tokcmp/out.json composite 키 (예: composite.A_review|all), 세션별 값은 out.json sessions (구성·시기 재계산에 사용).
- 중앙값 pre/post: A_review 88,455.5 / 246,557, B_strict 822,847 / 1,154,414, B_incl_MIX 1,299,115 / 3,301,113.
- A_strict post는 1건 (n=1), pre는 2건.

## 3. 판정 (개선안 10 기준: 중앙값 비 B/A의 부트스트랩 95% CI, 10,000회, seed 42. CI가 [0.9, 1.1] 안이면 비슷함, 밖이면 다름, 걸치면 판정 불가)

| 비교 (B/A 중앙값 비) | 시기 | 포함 | CI 95% | 판정 |
|---|---|---|---|---|
| A_review vs B_strict | 전체 | 4.6 이상 | 9.74 [6.64, 16.24] | 다름 |
| A_review vs B_strict | 전체 | 제외 | 6.61 [4.04, 11.53] | 다름 |
| A_review vs B_strict | pre | 포함 | 9.30 [4.55, 17.45] | 다름 |
| A_review vs B_strict | post | 포함 | 4.68 [0.39, 25.80] | 판정 불가 |
| A_review vs B_incl_MIX | 전체 | 포함 | 17.44 [12.32, 42.60] | 다름 |
| A_strict vs B_strict | 전체 | 포함 | 0.40 [0.27, 7.17] | 판정 불가 |
| A_strict vs B_strict | 전체 | 제외 | 0.58 [0.22, 2.81] | 판정 불가 |
| MIX vs B_strict | 전체 | 포함 | 0.22 [0.15, 0.36] (MIX 대비 B 약 0.22배) | 다름 |

- 주 판정 한 줄: 개선안 기준으로 '다름' (A_review 대 B_strict, 전체, 포함, 중앙값 비 9.74, CI [6.64, 16.24]).
- 단, 엄밀한 opus-haiku 구성 A_strict는 3건뿐이라 CI가 걸치며 '판정 불가'. 따라서 "opus-haiku 대 opus-sonnet-haiku 비슷함/다름"은 현 데이터로 확정 불가.
- A_review의 실체: 메인 직속 haiku-* 호출이 주가 되는 세션들이 포함됨 (OTHER 22건, A_strict 3건). 상세 분해는 미실행.

## 4. 근거 수치 3개 (출처)
1. 중앙값 비 B_strict/A_review, 포함 기준 9.74, CI [6.64, 16.24] (out.json composite의 median_t4; CI는 out.json sessions로 재계산. out.json boot에는 A_strict 대 B_strict만 있음).
2. B_strict 합계: raw 102,294,230 대 dedup 51,956,716, 배수 1.97 (out.json groups.B_strict|all raw_sum, ded_sum). 중복 미제거 시 B가 약 2배로 과대.
3. 캐시 읽기 비중: B_strict 81.4%, A_review 79.1% (out.json cache_read_share). 포함 합계는 캐시 읽기가 지배하므로 제외 값도 함께 봐야 함.

## 5. 확인 못 한 항목 / 한계
- 구성 A(opus-architect)의 표본 3건: 통계 판정 불가. 10-30 이후 A_strict 1건뿐.
- 시기 컷: UTC 날짜 기준. KST 기준 또는 0.8.2 실제 커밋 날짜(10-31) 기준이면 pre/post 구성이 바뀔 수 있음 (미확인). 10-31 시작 루트 9건 중 B 2건, A 0건.
- 스냅샷 변동: out.txt(17:11)의 서브 1,262건 대비 현재 1,281건 (19건 증가). 실행 중 세션 포함, 수치는 재실행 시 달라질 수 있음.
- MIX 29건은 sonnet과 opus를 함께 씀. 그중 opus→sonnet 경로(현행 훅에서 금지) 세션은 2건(out.json sonnet_under_opus_sessions=2). 정책 전 데이터로 추정되나 시기별 분리 미확인.
- 호출 수 정의: 서브 transcript 수. 루트 수와 별도 표기. 메인 턴의 개수(msgs_ded)는 out.json에 포함.
- 미계산: 단가 가중 합계, cost_usd, 에이전트 타입별 분해 (brief 범위 밖). 메인 모델 분포는 out.json composite.<키>.main_models에 기록됨 (A_review|all: haiku 13, sonnet 5, opus 5, null 2).
- NOSUB 86건 (서브 없음, 메인 단독 토큰): 구성 비교에서 제외. (usage 없는 레코드의 개수는 이 집계에서 따로 세지 않음.)
- parent 대상 누락 0건, meta.json 누락 0건, 파일 간 중복 message.id 0건 (out.json parent_missing, meta_missing, cross_file_dup_ids).

## 6. 변경 사항
- 신규 파일 1개: notes/runs/261031-answer-token-compare.md (이 파일). 저장소 다른 파일 수정 없음.
- 임시: /tmp/tokcmp/tokcmp.py, conflicts.py, out.json.
