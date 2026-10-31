# 계획서: 읽기 결과 전체 caveman화 평가
date: 261030
status: 사전 판정 (실행 전 아님)
related: notes/runs/261030-plan-caveman-relay-flow.md (읽기만. 수정 없음)

## 1. 질문 (원문 요지)
question: 읽기 결과 제공(read results returned to opus)도 전부 caveman화할지 평가
current_read_flow: opus -> haiku-scout/haiku-researcher -> 절감 번역(줄 수 제한 없음) -> opus
alt_read_flow: opus -> haiku-scout/haiku-researcher (결과 전체를 caveman 형식으로 반환, 번역 단계 없음) -> opus

## 2. 평가 항목
### 2.1 절감 대상
target: Opus 입력 토큰 (읽기 결과가 Opus 맥락에 들어가는 양)
note: 원본 파일을 Opus가 직접 읽지 않으므로 절감 폭은 결과 길이에 비례
### 2.2 근거·경로·수치 누락 위험
risk_1: caveman 압축 시 경로:행, 수치, 고유명사 탈락 가능
risk_2: 미발견 항목 표기 생략 시 Opus가 누락을 인지하지 못함
risk_3: 줄 수 제한 없음 조건은 절감 효과 상쇄 (원본 요약과 길이 유사)
mitigation: 모든 주장에 경로:행 필수, 미발견 항목 명시 필수, 수치는 원문 그대로 유지
### 2.3 번역 단계 제거 효과
effect_1: 중계 1단계 감소 -> Haiku 토큰 감소, 지연 감소
effect_2: 번역 단계에서 생기던 사실 변형 위험 감소 (scout가 직접 caveman 출력)
effect_3: 번역 단계 제거 시 검증 지점 1개 감소 -> 품질 점검은 metric_4로 보완 필요
### 2.4 실현 가능성
feasibility: 조건부 가능
reason_1: scout/researcher 반환 형식은 프롬프트로 지정 가능 (RETURN 형식 지시)
reason_2: 반환 형식 강제는 haiku-checker 검사 또는 Opus 재검증으로만 확인 가능 (코드 강제 아님)
reason_3: 기존 계획 10절 제약(opus-architect만 호출)과 충돌. 읽기 흐름 적용 시 사용자 결정 필요 (flow 문서 7절 decision_1)
### 2.5 판정
verdict: 전체 caveman화 자체는 비추천. 단, 번역 단계 제거 + 상한 문자 수 + 경로:행 형식 고정 조합은 가능
recommended_form: haiku-scout 반환을 terse English key: value, 4000자 상한, 근거는 경로:행 필수, 미발견 항목 명시
reason: 전체 caveman은 근거 필드까지 압축되어 검증 불가. 형식 고정 방식이 절감과 검증을 함께 만족

## 3. 측정 지표 (notes/metrics/*.jsonl, turnspend 기준)
metric_1: 읽기 흐름별 Opus 입력 토큰
metric_2: 읽기 흐름별 Haiku 토큰 합 (scout, researcher, 번역 단계 포함 여부 구분)
metric_3: Opus 재조회 수 (결과 부족으로 재호출한 횟수)
metric_4: 근거 누락 건수 (haiku-checker 표본 검사. 경로:행 누락 또는 원문 불일치)
judge_rule: 채택은 metric_1 절감이 metric_2 + metric_3 증가분을 상회하고, metric_4가 기준 이하일 때만

## 4. 사용자 결정 항목
decision_1: 읽기 결과 caveman 형식 범위 (전체 caveman 또는 형식 고정 4000자 상한) -> 권고: 형식 고정
decision_2: 10절 제약 개정 여부 (flow 문서 7절과 동일)
decision_3: 번역 단계 완전 제거 vs 샘플 검증 단계 유지

## 5. 다음 단계
next_1: 형식 고정안으로 소규모 시험 (표본 N건, metric_1~4 측정)
next_2: 10절 제약 개정 전에는 opus-architect 경유로만 시험
next_3: 전체 caveman 안은 metric_4 결과가 나온 뒤에만 재검토
