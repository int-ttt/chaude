# 계획서: 읽기 결과 전체 caveman화 평가
date: 261030
status: 사전 판정 (실행 전 아님)
related: notes/runs/261030-plan-caveman-relay-flow.md (읽기만. 수정 없음)
review: notes/runs/261030-plan-caveman-relay-read-review.md (개선점 병합본. 원본 백업 notes/archive/261030/261030-plan-caveman-relay-read.before-merge.md)

## 1. 질문 (원문 요지)
question: 읽기 결과 제공(read results returned to opus)도 전부 caveman화할지 평가
current_read_flow: opus -> haiku-scout/haiku-researcher -> 절감 번역(줄 수 제한 없음) -> opus
alt_read_flow: opus -> haiku-scout/haiku-researcher (결과 전체를 caveman 형식으로 반환, 번역 단계 없음) -> opus

## 2. 평가 항목
### 2.1 절감 대상
target: Opus 입력 토큰 (읽기 결과가 Opus 맥락에 들어가는 양)
note: 원본 파일을 Opus가 직접 읽지 않으므로 절감 폭은 결과 길이에 비례
### 2.2 근거·경로·수치 누락 위험
risk_1: caveman 압축 시 경로:행, 수치, 고유명사 탈락 가능 (실측 위험 낮음: RETURN에 "경로:행, 수치, 식별자 원문 그대로" 지시 시 대체로 유지)
risk_2: 미발견 항목 표기 생략 시 Opus가 누락을 인지하지 못함 ("없음"과 "안 봄" 구분 불가) -> 위험 높음
risk_3: 줄 수 제한 없음 조건은 절감 효과 상쇄 (원본 요약과 길이 유사)
risk_4: 한정어(only, except, not, unless, 미확인) 압축 시 탈락 -> 위험 높음 (예: "36번만 해당" -> "36 해당")
risk_5: 4000자 상한 시 scout가 문맥을 임의 생략 (실관측: DECISIONS.md 25·29·30·19행 문맥 생략, Doubts에만 기재) -> 위험 중간
risk_6: DECISIONS.md:36 규칙("Final message = 한국어 존댓말 요약")으로 영어 지시에도 한국어 산문 반환 (실관측: scout 2건 모두 한국어 머리말 포함) -> 위험 중간
mitigation: 모든 주장에 경로:행 필수, 미발견·미확인 항목 명시 필수(2.6 스키마), 수치·한정어는 원문 그대로 유지
### 2.3 번역 단계 제거 효과
note_correction: 번역 단계 제거로 "중계 1단계 감소"는 과대 추정. 관측상 scout가 같은 실행 안에서 한국어 요약을 직접 작성함(별도 hop 없음). 번역 주체(scout 자신 vs 별도 Haiku)는 1절에 명시 필요.
effect_1: 절감원은 hop 감소가 아니라 (a) 언어(한국어 -> 영어), (b) 형식(산문 -> key: value)이다. Haiku 토큰·지연 감소 폭은 이 두 요인으로만 추정하고 hop 감소분은 계산에 넣지 않음
effect_2: 사실 변형 위험 감소는 별도 번역 hop이 실제로 존재할 때만 성립. 별도 hop 없으면 근거 없음
effect_3: 번역 단계 제거 시 검증 지점 1개 감소 -> 품질 점검은 metric_4로 보완 필요
### 2.4 실현 가능성
feasibility: 조건부 가능
reason_1: scout/researcher 반환 형식은 프롬프트로 지정 가능 (RETURN 형식 지시)
reason_2: 반환 형식 강제는 코드 강제가 아니므로 불가. haiku-checker 검사 또는 Opus 재검증으로 위반을 탐지만 가능 (2.6 스키마의 필수 필드 누락으로 탐지)
reason_3: 기존 계획 10절 제약(opus-architect만 호출)과 충돌. 읽기 흐름 적용 시 사용자 결정 필요 (flow 문서 7절 decision_1)
reason_4: DECISIONS.md:36 충돌. 11개 에이전트 .claude/agents/*.md 전부 한국어 최종 반환 규칙 보유. 예외 허용 여부는 사용자 결정 (4절 decision_4)
### 2.5 판정
verdict: 전체 caveman화 자체는 비추천. 단, 번역 단계 제거 + 상한 문자 수 + 경로:행 형식 고정 조합은 가능
verdict_1: 전체 caveman은 한정어·검색 범위·잘림 여부를 먼저 잃음 -> Opus가 누락을 인지 못함. 비채택
recommended_form: haiku-scout 반환을 terse English key: value, 4000자 상한, 2.6 필수 필드 스키마 준수
reason: 전체 caveman은 근거 필드까지 압축되어 검증 불가. 형식 고정 + 필수 필드가 절감과 검증을 함께 만족
### 2.6 반환 스키마 고정 (모든 읽기 RETURN에 동일 문구로 첨부)
claim_line: "<path>:<line> | <사실 또는 원문 인용>" (경로:행 없는 claim 금지)
searched: 검색한 파일/패턴 목록
not_found: 찾았으나 없는 항목 (없으면 "none")
not_checked: 범위 밖이라 안 본 항목 (없으면 "none")
truncated: yes|no. yes면 omitted: 생략한 행 번호 명시
qualifier_rule: only, except, not, unless, 미확인은 압축 금지 (원문 유지)
detection: 위 필수 키(searched, not_found, not_checked, truncated, claim_line 1개 이상) 중 하나라도 없으면 형식 위반 1건. 정규식 검사로 충분 (haiku-checker 표본 또는 오프라인 스크립트)
note: 강제가 아니라 탐지. 위반 시 재호출 1회로 제한, metric_3에 합산

## 3. 측정 지표 (notes/metrics/*.jsonl, turnspend 기준)
metric_1: Opus에 들어가는 읽기 결과 크기 = handback 메시지 문자 수와 토큰 수 (handback 본문이 그대로 Opus 맥락에 들어감). 흐름별 비교
metric_2: 읽기 흐름별 Haiku 토큰 합. 출처는 Agent 결과의 subagent_tokens, tool_uses, duration_ms (Agent 도구 반환에 이미 노출됨. 실측 예: scout 2건 = 10684 / 23278 subagent_tokens). scout, researcher, 번역 단계 포함 여부 구분
metric_3: Opus 재조회 수 (결과 부족·형식 위반으로 재호출한 횟수)
metric_4: 근거 누락 건수 (haiku-checker 표본 검사. 경로:행 누락, 원문 불일치, 한정어 탈락, 2.6 필수 필드 누락)
precheck: haiku-scout가 internal/harness/turnspend.go 와 notes/metrics/2026-10-30.jsonl 에서 subagent_type별 토큰 필드 존재 여부 확인. 없으면 metric_2는 Agent 반환값을 수동 기록
sample: 동일 읽기 과제 N=10 (파일 1개 조회 4, 다중 파일 grep 4, 웹 조회 2)을 현행안과 스키마안 각 1회씩 = 20회
thresholds_draft (사용자 확정 필요):
  - metric_1: 스키마안이 현행 대비 handback 반환 크기 30% 이상 감소
  - metric_3: 재호출 비율 현행 대비 증가 0~1건 이내 (N=10 기준)
  - metric_4: 근거 누락 0건, 형식 위반 1건 이하
  - qualifier_loss: haiku-checker 원문 대조로 한정어 탈락 0건
judge_rule: metric_1 절감이 metric_2 + metric_3 증가분을 상회하고, metric_4 및 qualifier_loss가 임계값 이내일 때만 채택
order: precheck -> 기준선(현행) 10회 -> DECISIONS.md:36 예외 적용(decision_4 승인 시) -> 스키마안 10회 -> 비교 -> 사용자 판단

## 4. 사용자 결정 항목
decision_1: 읽기 결과 caveman 형식 범위 (전체 caveman 또는 형식 고정 4000자 상한) -> 권고: 형식 고정
decision_2: 10절 제약 개정 여부 (flow 문서 7절과 동일)
decision_3: 번역 단계 완전 제거 vs 샘플 검증 단계 유지
decision_4: DECISIONS.md:36 한국어 반환 규칙 예외 허용 여부. 허용 시 haiku-scout.md, haiku-researcher.md 에만 "Exception: when the brief says RETURN: schema, return only the schema in terse English (Korean values verbatim); no Korean preface." 1줄 추가, 나머지 9개 에이전트 파일 무변경, DECISIONS.md에 [agents] 261030 예외 기록. 권고: 허용 여부 사용자 확정 필요

## 5. 다음 단계
next_1: 형식 고정안으로 소규모 시험 (표본 N=10, 2절 metric_1~4 및 qualifier_loss 측정)
next_2: 10절 제약 개정 전에는 opus-architect 경유로만 시험
next_3: 전체 caveman 안은 metric_4 및 qualifier_loss 결과가 임계값을 넘을 때만 재검토 (0건이면 재검토 불필요)
