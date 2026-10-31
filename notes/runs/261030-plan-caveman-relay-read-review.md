# 검토: 261030-plan-caveman-relay-read.md
date: 261030
reviewer: opus-architect
scope: 개선점만 (실행 없음). 원 계획서 수정 없음

## 1. 판정 (전체 caveman 반환 vs 기존 절감 번역 후 전달)
verdict: 원 계획 결론(전체 caveman 비추천, 형식 고정 terse English + 경로:행)에 동의. 단 절감 근거와 측정 설계는 수정 필요
verdict_1: 전체 caveman은 경로·수치보다 한정어(only, except, not, 미확인)·검색 범위·잘림 여부를 먼저 잃음 -> Opus가 누락을 인지 못함. 비채택
verdict_2: "번역 단계 제거로 중계 1단계 감소"(2.3 effect_1)는 과대 추정 가능성 큼. 관측상 scout가 같은 실행 안에서 한국어 요약을 직접 작성함(별도 hop 없음) -> 실제 절감원은 hop 감소가 아니라 언어(한국어->영어)와 형식(산문->key: value)
verdict_3: 반환 형식은 프롬프트로만 통제되므로 "강제"는 불가. 대신 누락이 기계적으로 드러나는 필수 필드 스키마로 "위반 탐지 가능"하게 만드는 것이 현실적 목표

## 2. 누락 위험 재정리 (원 2.2 보완)
risk_paths_numbers: 낮음. RETURN에 "경로:행, 수치, 식별자 원문 그대로" 지시 시 대체로 유지됨
risk_qualifiers: 높음. caveman 압축은 부정·조건·예외어를 먼저 탈락시킴 (예: "36번만 해당" -> "36 해당")
risk_coverage: 높음. 무엇을 검색했고 무엇을 안 봤는지가 빠지면 "없음"과 "안 봄"이 구분 안 됨
risk_truncation: 중간. 4000자 상한 시 scout가 문맥을 임의로 생략함 (본 검토 중 실관측: scout가 DECISIONS.md 25·29·30·19행 문맥을 상한 때문에 생략하고 Doubts에만 기재)
risk_language_drift: 중간. DECISIONS.md:36 규칙 때문에 영어 지시에도 한국어 산문으로 반환됨 (본 검토 중 실관측: scout 2건 모두 한국어 머리말 포함)

## 3. 개선점
### improvement_1: DECISIONS.md:36 충돌 해소 단계 누락 (필수, 최우선)
problem: DECISIONS.md:36 = 11개 .claude/agents/*.md 전부 "Final message = only a brief result summary (Korean 존댓말...)" 한 줄 보유. terse English 반환과 정면 충돌. 계획서에 이 충돌·수정 대상 파일 없음
step: 사용자 결정 항목에 decision_4 추가 -> "Opus/에이전트가 호출한 읽기 작업의 반환은 terse English key: value, 사용자 게시용 요약만 한국어" 예외 허용 여부
step_if_yes: haiku-coder가 /Users/leekwangseok/PycharmProjects/claude-harness/.claude/agents/haiku-scout.md, haiku-researcher.md 의 해당 줄 바로 뒤에 예외 1줄 추가 (나머지 9개 파일 변경 금지)
wording_draft: "Exception: when the brief says RETURN: schema, return only the schema below in terse English (Korean values verbatim); no Korean preface."
check: haiku-checker가 2개 파일에 예외 줄 존재, 나머지 9개 파일 무변경 확인
also: DECISIONS.md에 [agents] 261030 항목으로 예외 기록

### improvement_2: 반환 스키마를 "위반 탐지 가능" 형태로 고정 (원 2.4 reason_2 보완)
problem: recommended_form이 "terse English key: value, 4000자, 경로:행, 미발견 명시"까지만 정의. 프롬프트 통제 전제에서 위반을 사후에 기계적으로 잡을 필수 필드가 없음
schema (모든 읽기 RETURN에 동일 문구로 첨부):
  - claim 줄: "<path>:<line> | <사실 또는 원문 인용>" (경로:행 없는 claim 금지)
  - searched: 검색한 파일/패턴 목록
  - not_found: 찾았으나 없는 항목 (없으면 "none")
  - not_checked: 범위 밖이라 안 본 항목 (없으면 "none")
  - truncated: yes|no, yes면 omitted: 생략한 행 번호
  - qualifiers 원문 유지 규칙: only, except, not, unless, 미확인은 압축 금지
detection: 위 5개 키(searched, not_found, not_checked, truncated, claim 줄 1개 이상) 중 하나라도 없으면 형식 위반 1건으로 집계. 정규식 검사로 충분 (haiku-checker 표본 또는 오프라인 스크립트)
note: 이는 강제가 아니라 탐지. 위반 시 대응은 재호출 1회로 제한 (metric_3에 합산)

### improvement_3: 측정 지표에 기준선·표본·임계값·수집 경로 명시 (원 3절 보완)
problem: judge_rule의 "metric_4가 기준 이하"에 수치 없음. 기준선(현행 한국어 요약) 측정 단계 없음. turnspend/metrics jsonl이 에이전트별로 토큰을 구분하는지 미확인
metric_1_redefine: Opus에 들어가는 읽기 결과 크기 = handback 메시지 문자 수와 토큰 수 (handback 본문이 그대로 Opus 맥락에 들어감)
metric_2_source: Agent 결과의 subagent_tokens, tool_uses, duration_ms (Agent 도구 반환에 이미 노출됨. 본 검토 실측 예: scout 2건 = 10684 / 23278 subagent_tokens)
precheck: haiku-scout가 /Users/leekwangseok/PycharmProjects/claude-harness/internal/harness/turnspend.go 와 notes/metrics/2026-10-30.jsonl 에서 subagent_type별 토큰 필드 존재 여부 확인. 없으면 metric_2는 Agent 반환값을 수동 기록
sample: 동일 읽기 과제 N=10 (파일 1개 조회 4, 다중 파일 grep 4, 웹 조회 2)를 현행안과 스키마안 각 1회씩 = 20회
thresholds_draft (사용자 확정 필요):
  - metric_1: 스키마안이 현행 대비 handback 토큰 30% 이상 감소
  - metric_3: 재호출 비율 현행 대비 증가 0~1건 이내 (N=10 기준)
  - metric_4: 근거 누락(경로:행 없음 또는 원문 불일치) 0건, 형식 위반 1건 이하
  - qualifier_loss: haiku-checker가 원문 대조로 한정어 탈락 0건
order: precheck -> 기준선(현행) 10회 -> improvement_1 적용 -> 스키마안 10회 -> 비교 -> 사용자 판단

## 4. 기타 지적 (경미)
- 2.3 effect_2 "번역 단계의 사실 변형 위험 감소": 별도 번역 hop이 없다면 근거 없음. 번역 주체(scout 자신 vs 별도 Haiku)를 계획서 1절에 명시할 것
- 2.2 risk_3과 recommended_form(4000자 상한) 관계: 상한은 절감과 잘림을 동시에 유발 -> truncated/omitted 필드 없이 상한만 두면 risk_truncation 증가
- 5절 next_3 "전체 caveman 재검토": qualifier_loss 지표가 0건이 아니면 재검토 불필요로 조건 명시 권장

## 5. 변경 금지
- 원 계획서 /Users/leekwangseok/PycharmProjects/claude-harness/notes/runs/261030-plan-caveman-relay-read.md
- haiku-scout.md, haiku-researcher.md 외 9개 에이전트 파일의 DECISIONS.md:36 규칙 줄
- flow 문서 notes/runs/261030-plan-caveman-relay-flow.md
