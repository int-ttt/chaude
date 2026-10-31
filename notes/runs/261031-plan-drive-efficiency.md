# 계획서: 구동 방식별 효율 비교 (a/b/c)

date: 2026-10-31
status: 계획 초안 (판단 미확정)
repo_edits: none

## 1. 질문 원문

"직전 테스트 비교: (a) opus→haiku×N, (b) opus→sonnet→haiku×N, (c) sonnet→haiku×N; 3회 반복, 회차마다 haiku 에이전트 +1; 어떤 방식이 더 효율적인지 판단 및 추천"

## 2. 측정 결과 (직전 보고 값)

요청 파일: /tmp/drive/opus_request_1.md, opus_request_2.md, opus_request_3.md (round 1·2·3)
결과 파일 (내용 모두 OK): /tmp/drive/a_{회차}_{k}.txt, b_*.txt, c_*.txt

| 방식 | 회차 | 호출 수 | 호출당 토큰 | 호출당 duration |
|---|---|---|---|---|
| (a) opus→haiku×N | 1 | 1 | 약 7,267~7,769 (회차 범위) | 약 6.2~8.6초 (회차 범위) |
| (a) opus→haiku×N | 2 | 2 | 〃 | 〃 |
| (a) opus→haiku×N | 3 | 3 | 〃 | 〃 |
| (c) sonnet→haiku×N | 1 | 1 | 약 7,086~7,207 (회차 범위) | 약 4.3~4.9초 (회차 범위) |
| (c) sonnet→haiku×N | 2 | 2 | 〃 | 〃 |
| (c) sonnet→haiku×N | 3 | 3 | 〃 | 〃 |
| (b) opus→sonnet→haiku×N | 1~3 | - | 미실행 | 미실행 |

- 회차별 호출 수 합계: (a) 6, (c) 6 (1+2+3).
- 범위 중앙값 기준 대략 비교: 토큰 (c) 약 -5% (a 대비), duration (c) 약 -38% (a 대비).
- 주의: 위 수치는 직전 보고 값이며, /tmp/drive/ 결과 파일에는 OK 텍스트만 있고 토큰·시간 값은 없음. 원본 로그 대조 필요.

## 3. 판단 기준

- 효율 정의: 작업당 토큰과 시간 (작업 = haiku 호출 1건 완료).
- 동률 기준: 두 방식 차이가 ±5% 이내면 동률로 본다.
  - 토큰: (a)와 (c) 범위 중앙값 차이가 약 5% 수준이라 경계에 걸림 → 동률 가능성 있음.
  - duration: 차이가 약 38%로 ±5%를 크게 넘음 → (c)가 빠름.
- 표본 한계: 회차당 호출 수 합계 6·6·6 (총 6건/방식 수준), 회차 간 범위가 겹침. 통계적 유의성 검정 불가. 결론은 잠정치.
- 판단 보류 조건: 토큰 차이가 ±5% 경계 안이면 시간 기준으로만 추천 가능.

## 4. 질문 목록

(b) 미실행 사유:
- 훅 SUB_CALLERS (.claude/hooks/agent-model-pin.py 27행): sonnet-reviewer, sonnet-analyst, opus-architect 가 서브에이전트일 때 haiku-* 외 호출 차단 (45-50행 deny).
- 75-78행: agent_id 가 있고 kind 가 haiku-* 가 아니면 deny. sonnet이 haiku를 부르는 것은 허용 경로지만, opus 호출은 sonnet에서 차단됨 (요청 파일 note: "sonnet cannot call opus (hook)").

질문:
1. (b)의 opus→sonnet 단계가 훅에서 막히는 것이 의도된 정책인가, 아니면 우회가 필요한 제약인가?
2. haiku-prompter 40행 예외 (`_target in (sonnet-analyst, sonnet-reviewer, opus-architect)` 또는 `haiku-*`)가 sonnet→opus 호출의 우회 경로가 되는가? 확인 필요.
3. /tmp/drive/b_*.txt (OK 파일 6개) 가 존재한다. (b)가 실제 미실행인지, 메인 스레드가 대신 쓴 것인지 확인 필요.
4. 토큰·duration 값의 원본 출처(로그 파일)는 어디인가?

## 5. 실행 단계 제안

1. 질문 4개에 답하여 (b) 실행 가능 여부와 측정값 출처 확정.
2. 판단 확정: 토큰 ±5% 동률 기준 적용 후 효율 방식 결정.
3. 추천 문장 작성 (notes/ 내 결과 문서 또는 사용자 보고용).
4. 추가 측정이 필요하면 측정 항목:
   - 호출당 입력/출력 토큰 분리 (현재 합산치만 있음)
   - 호출당 duration (start/end date +%s 기록)
   - 회차 반복 수 증가 (현재 3회, 표본 6건) → 최소 5회 이상 권장
   - (b) 실행 가능 시 동일 조건으로 3 방식 전부 측정

## 6. 저장소 파일 수정

없음. 본 파일 하나만 작성.
