# 계획서: caveman 릴레이 흐름 제안 (입력·읽기·쓰기 3경로 평가)
date: 261030
status: 제안 원문 저장 + 단계별 판정 + 리뷰 개선 병합 (실행 전 아님)
related: notes/runs/261030-plan-caveman-relay.md (읽기만. 수정 없음)
review_merged: notes/runs/261030-plan-caveman-relay-flow-review.md (imp_1, imp_2, imp_4, imp_5 병합)
archive: notes/archive/261030/261030-plan-caveman-relay-flow.orig.md (병합 전 원본)

## 1. 사용자 제안 (원문 요지)
### 1.1 입력 흐름
flow_input: 사용자 입력 -> (opus 또는 haiku -> 토큰·사용량 절감 번역 -> opus) -> caveman -> haiku

### 1.2 읽기 흐름
flow_read: opus -> haiku-scout/haiku-researcher -> 토큰 절감 번역 -> opus
(원문 주석 삭제. 3.2절 개정안으로 대체)

### 1.3 쓰기 흐름
flow_write: opus -> haiku용 최적화 프롬프트 작성 -> haiku에 전달 (쓰기 에이전트는 별도 클라이언트로 맥락 유지, 단발 작업은 일회성 에이전트) -> 요약 반환 -> opus
(원문 반환 방식은 3.3절 개정안으로 대체)

## 2. 기존 계획과의 관계
existing_plan: notes/runs/261030-plan-caveman-relay.md (상태 merged)
existing_flow: 사용자 -> 메인 Haiku(맥락 압축) -> opus-architect(caveman 출력) -> 메인 Haiku(존댓말 번역) -> 사용자
overlap: 입력 흐름의 "번역 -> opus -> caveman -> haiku" 부분은 기존 조건 B·C와 같은 구조
conflict_1: 기존 계획 10절 4행 "에이전트 호출은 opus-architect만". 읽기 흐름(haiku-scout/haiku-researcher)과 쓰기 흐름(haiku 에이전트)은 이 제약과 충돌. 사용자 결정 항목 -> 7절
conflict_2: 기존 계획 2절 "압축·번역은 메인 Haiku가 직접 수행". 입력 흐름의 압축 주체 선택은 메인 Haiku 고정으로 확정 (3.1절)
note: 충돌 해소 여부는 사용자 결정 필요. 본 문서는 판정만 기록

## 3. 단계별 판정
### 3.1 입력 흐름
verdict: 조건부 유망 -> 범위 축소. 기존 계획 조건 B·C에 흡수
decision: 압축 주체 = 메인 Haiku 고정. opus 압축 옵션 삭제. 별도 실험 없이 기존 계획 조건 B·C로 대체하며 중복 실행 금지
gain: Opus 입력 토큰 감소 (입력 단가는 출력보다 낮으므로 절감 폭 작음. 캐시 적중 시 더 작음)
cost: 압축 주체 Haiku 토큰 추가, 번역 단계 추가 지연
risk_1: 압축 주체를 opus로 두면 Opus 입력 절감 효과가 상쇄됨. 메인 Haiku 고정으로 제외
risk_2: 번역 단계가 "토큰 절감 번역"이면 사실·경로·수치 탈락 (기존 계획 risk_1과 동일)
risk_3: 입력 측 압축과 출력 측 caveman을 함께 쓰면 원 입력과 최종 답의 대응 추적이 어려움
judge_point: 원 입력 대응 추적은 기존 계획 조건 B·C 측정으로 확인

### 3.2 읽기 흐름 (개정안)
verdict: 동의. 세 흐름 중 절감 대비 위험이 가장 낮으므로 1순위
flow_read_revised: opus -> haiku-scout/haiku-researcher (RETURN verbatim <=4000자, 모든 주장에 경로:행 표기, 미발견 항목 명시) -> opus
change_1: 원안 무제한 조건 삭제. 문자 상한 4000자 + 근거 형식 고정으로 교체
change_2: 별도 "토큰 절감 번역" 단계 삭제. scout가 처음부터 terse English key: value로 반환 (중계 1단계 감소)
gain: Opus가 원본 파일을 직접 읽지 않으므로 Opus 입력 토큰 감소. 탐색 작업을 저가 모델로 이동
cost: scout/researcher 호출 토큰, Opus 측 결과 재검증 토큰
risk_1: 상한 4000자 내에서도 원본 요약과 길이가 비슷해질 수 있음. 8절 측정 지표로 확인
risk_2: haiku-scout가 놓친 파일·행을 Opus가 모르고 넘어갈 위험 (근거 누락). 경로:행 필수로 완화
risk_3: 기존 계획 10절 제약(opus-architect만 호출)과 충돌. 7절 결정 항목
judge_point: 모든 주장에 경로:행이 없으면 반려. 미발견 항목 명시 여부로 누락 점검

### 3.3 쓰기 흐름 (개정안)
verdict: 조건부 실현 가능 (원안 "비추천" 수정). 단, 반환을 요약만으로 두는 원안은 여전히 비추천
flow_write_revised: opus -> haiku용 최적화 프롬프트 작성 -> 실행 주체 선택 -> 반환 -> opus
execution_choice:
  - 단발 작업: haiku-coder 일회성 호출 유지
  - 연속 작업: mode f 작업 공간 재사용 (claude -p --resume, 20턴·40k 토큰·유휴 270초에서 교체)
return_required_fields:
  - 변경 파일 경로 목록
  - git diff --stat
  - 검사 명령과 PASS/FAIL
  - 작업 공간 id
removed: 요약만 반환하는 원안 문구 삭제 (검증 불가)
via_subagent_normal: 불가. 일반 서브에이전트는 매 호출 새 컨텍스트로 시작 (notes/research/subagent-context-model.md:9). Agent 경로로는 맥락 유지 불가
via_fork: 불가. 부모 모델을 상속하고 부모 맥락 전체를 상속하므로 Haiku 전환·토큰 절감 불가. 호출 방식 본문 미확인 (research:11 unverified)
via_harness_mode_f: 가능, 이미 구현됨. claude -p 자식을 stream-json으로 띄우고 --resume <session_id>로 세션 이어감 (core.go:145-152). [ws:<id>] 줄을 haiku로 라우팅, 저장된 ws 세션이 있으면 --resume로 재개 (orchws.go:78-101). 회전 기준 20턴 / 40k 토큰 / 유휴 270초 (orchws.go:31-43), 인계 상한 1500 (orchws.go:35)
new_client: 불필요. 별도 클라이언트를 새로 만들 필요 없음
boundary: Opus(Agent 도구만 보유)는 [ws:<id>] 줄을 직접 낼 수 없음. 라우팅 주체는 메인 세션/하네스
risk_2_revised: 세션 수·캐시 미스 증가 위험은 ws 회전 기준(유휴 270초 < 캐시 TTL 5분)으로 제한되어 하향
risk_3: 일회성 에이전트 맥락 부족으로 재작업 발생 가능. 재작업 횟수를 9절 지표로 측정
risk_4: 쓰기 실패 시 원인 추적 경로는 ws id와 검사 결과로 유지
doubt_1: 재사용·retire 동작은 코드 읽기 기준. 실행 확인 안 됨
doubt_2: mode f는 --clients-file 설정 시에만 live 자식을 둠 (repl.go:204-208). 기본 실행에서 켜져 있는지 확인 필요
judge_point: 반환 4개 필드가 모두 있어야 채택

## 4. 트레이드오프 요약
- 절감 대상: 입력 흐름은 Opus 입력 토큰, 읽기 흐름은 Opus 입력 토큰(원본 읽기 생략), 쓰기 흐름은 Opus 출력 토큰
- 공통 비용: 번역·압축 단계의 Haiku 토큰과 지연
- 공통 품질 위험: 근거 누락, 불확실 표현 약화, 설명 규칙 위반 (기존 계획 risk_1~3과 동일)
- 단가 주의: Opus 출력 단가가 가장 높으므로 출력 절감이 가장 큰 효과. 입력 절감은 효과가 작음
- 판정 기준: 쓰기 흐름은 mode f 재사용 확인 후 실험 가능 (보류 해제 조건)

## 5. 비용·품질 위험 정리
cost_risk: 중계 단계가 늘어날수록 Haiku 토큰이 누적됨. 단계 수 3개 이상이면 절감분 상쇄 가능성 높음
quality_risk: 단계마다 근거 누락이 누적됨. 최종 Opus 판단 근거가 원문과 다를 수 있음
limit: 본 문서는 제안에 대한 사전 판정임. 수치 판정은 8절 측정 지표로 실행 후 가능

## 6. 다음 단계 (제안)
next_1: 입력 흐름은 기존 계획의 조건 B·C에 흡수 (중복 실행 금지)
next_2: 읽기 흐름을 1순위 과제로 실행. 10절 제약 개정(7절 결정) 전에는 opus-architect 경유로만 시험
next_3: 쓰기 흐름은 mode f 작업 공간 재사용 확인(doubt_2) 후 해제 가능. 확인 전까지 보류
next_4: 압축 주체는 메인 Haiku 고정으로 확정. 기존 계획 조건 B·C 실행 후 판정

## 7. 사용자 결정 항목 (기존 계획 10절에서 분리)
decision_1: 기존 계획 10절 4행 "에이전트 호출은 opus-architect만" 제약 유지 여부
  - 유지 시: 읽기·쓰기 흐름 불가. Opus가 직접 읽거나 쓰지 않는 안은 보류
  - 개정 시 후보 문구: "Opus가 직접 읽거나 쓰지 않는다. 읽기는 haiku-scout, 쓰기는 haiku-coder 또는 mode f 작업 공간"
decision_2: 쓰기 흐름에서 Opus가 [ws:<id>] 라우팅 주체가 될지, 메인 세션/하네스가 맡을지
decision_3: 입력 흐름 압축 주체 확정 (메인 Haiku 고정 권고)

## 8. 측정 지표 (흐름별, notes/metrics/*.jsonl 및 turnspend 기준)
metric_1: 흐름별 Opus 입력 토큰 / Opus 출력 토큰
metric_2: 흐름별 Haiku 토큰 합 (scout, researcher, coder, 번역·압축 단계 포함)
metric_3: 재작업 횟수 (쓰기 흐름: 검사 FAIL 후 재호출 수, 읽기 흐름: Opus 재조회 수)
metric_4: 근거 누락 건수 (haiku-checker 표본 검사. 경로:행 누락 또는 원문 불일치 건수)
judge_rule: 채택은 metric_1 절감이 metric_2 + metric_3 증가분을 상회하고, metric_4가 기존 계획 기준 이하일 때만
