# 계획서: caveman 릴레이 흐름 제안 (입력·읽기·쓰기 3경로 평가)
date: 261030
status: 제안 원문 저장 + 단계별 판정 (실행 전 아님)
related: notes/runs/261030-plan-caveman-relay.md (읽기만. 수정 없음)

## 1. 사용자 제안 (원문 요지)
### 1.1 입력 흐름
flow_input: 사용자 입력 -> (opus 또는 haiku -> 토큰·사용량 절감 번역 -> opus) -> caveman -> haiku

### 1.2 읽기 흐름
flow_read: opus -> haiku-scout/haiku-researcher -> 토큰 절감 번역 (줄 수 제한 없음) -> opus

### 1.3 쓰기 흐름
flow_write: opus -> haiku용 최적화 프롬프트 작성 -> haiku에 전달 (쓰기 에이전트는 별도 Claude Code 클라이언트로 맥락 유지, 단발 작업은 일회성 에이전트) -> 변경 요약과 방법 맥락만 -> opus

## 2. 기존 계획과의 관계
existing_plan: notes/runs/261030-plan-caveman-relay.md (상태 merged)
existing_flow: 사용자 -> 메인 Haiku(맥락 압축) -> opus-architect(caveman 출력) -> 메인 Haiku(존댓말 번역) -> 사용자
overlap: 입력 흐름의 "번역 -> opus -> caveman -> haiku" 부분은 기존 조건 B·C와 같은 구조
conflict_1: 기존 계획 10절 4행 "에이전트 호출은 opus-architect만". 제안의 읽기 흐름(haiku-scout/haiku-researcher)과 쓰기 흐름(haiku 에이전트)은 이 제약과 충돌
conflict_2: 기존 계획 2절 "압축·번역은 메인 Haiku가 직접 수행". 제안의 입력 흐름은 압축 주체로 opus 또는 haiku를 선택하게 함. 주체 선택 기준 미정
note: 충돌 해소 여부는 사용자 결정 필요. 본 문서는 판정만 기록

## 3. 단계별 판정
### 3.1 입력 흐름
verdict: 조건부 유망
gain: Opus 입력 토큰 감소 (입력 단가는 출력보다 낮으므로 절감 폭 작음)
cost: 압축 주체 Haiku 토큰 추가, 번역 단계 추가 지연
risk_1: 압축 주체를 opus로 두면 Opus 입력 절감 효과가 상쇄됨 (압축에 Opus 토큰 사용). haiku로 두면 근거 누락 위험 증가
risk_2: 번역 단계가 "토큰 절감 번역"이면 사실·경로·수치 탈락 (기존 계획 risk_1과 동일)
risk_3: 입력 측 압축과 출력 측 caveman을 함께 쓰면 원 입력과 최종 답의 대응 추적이 어려움
judge_point: 압축 주체 선택 기준(비용 우선 vs 품질 우선)을 먼저 정해야 함

### 3.2 읽기 흐름
verdict: 조건부 유망 (줄 수 제한 없음 조건이 절감 효과를 약화시킬 수 있음)
gain: Opus가 원본 파일을 직접 읽지 않으므로 Opus 입력 토큰 감소. 탐색 작업을 저가 모델로 이동
cost: scout/researcher 호출 토큰, 결과 번역 토큰, Opus 측 결과 재검증 토큰
risk_1: "줄 수 제한 없음"이면 번역본 길이가 원본 요약과 비슷해져 절감 폭이 작아질 수 있음
risk_2: haiku-scout가 놓친 파일·행을 Opus가 모르고 넘어갈 위험 (근거 누락)
risk_3: 기존 계획 10절 제약(opus-architect만 호출)과 충돌. 채택 시 제약 개정 필요
judge_point: 읽기 결과에 파일 경로·행 번호를 필수 필드로 두지 않으면 검증 불가
suggestion: 번역 출력에 줄 수 상한 대신 근거 행 형식(E-ID, 경로:행) 고정을 검토

### 3.3 쓰기 흐름
verdict: 비추천 (가장 위험)
gain: 단순 편집 작업을 Haiku로 옮겨 Opus 출력 토큰 감소 가능
cost: 최적화 프롬프트 작성 Opus 출력, Haiku 작업 토큰, 별도 클라이언트 유지 비용, 변경 요약 전달 토큰
risk_1: 변경 요약과 방법 맥락만 Opus에 전달하면 Opus가 실제 변경을 검증하지 못함 (diff 미전달 시 검증 누락)
risk_2: 별도 Claude Code 클라이언트 맥락 유지는 세션 수 증가, 캐시 미스 증가, 관리 비용 증가
risk_3: 일회성 에이전트는 맥락 부족으로 재작업 발생 가능. 재작업 비용이 절감분을 넘을 수 있음
risk_4: 쓰기 작업 실패 시 원인 추적 경로가 끊김 (요약만 남으므로)
judge_point: 최소 조건은 변경 파일 경로 목록, diff 또는 해시, 테스트 결과를 Opus에 전달하는 것
suggestion: 쓰기 흐름은 1회 실험 대상에서 제외하고, 읽기 흐름 결과 확인 후 재검토

## 4. 트레이드오프 요약
- 절감 대상: 입력 흐름은 Opus 입력 토큰, 읽기 흐름은 Opus 입력 토큰(원본 읽기 생략), 쓰기 흐름은 Opus 출력 토큰
- 공통 비용: 번역·압축 단계의 Haiku 토큰과 지연
- 공통 품질 위험: 근거 누락, 불확실 표현 약화, 설명 규칙 위반 (기존 계획 risk_1~3과 동일)
- 단가 주의: Opus 출력 단가가 가장 높으므로 출력 절감이 가장 큰 효과. 입력 절감은 효과가 작음

## 5. 비용·품질 위험 정리
cost_risk: 중계 단계가 늘어날수록 Haiku 토큰이 누적됨. 단계 수 3개 이상이면 절감분 상쇄 가능성 높음
quality_risk: 단계마다 근거 누락이 누적됨. 최종 Opus 판단 근거가 원문과 다를 수 있음
limit: 본 문서는 제안에 대한 사전 판정임. 측정 데이터 없음. 수치 판정은 기존 계획 5절 지표로 실행 후 가능

## 6. 다음 단계 (제안)
next_1: 입력 흐름만 기존 계획의 조건 B·C에 포함되는지 확인 (중복 실행 방지)
next_2: 읽기 흐름은 별도 과제 1개로 분리해 기존 계획 10절 제약 개정 후 검토
next_3: 쓰기 흐름은 보류
next_4: 압축 주체(opus/haiku) 기준 확정 후 실행
