# 계획: REPL 연결 vs claude -p 비교 분석 (261031)

상태: 계획only. 구현 금지. 비교 분석만 수행.
대상 경로: /Users/leekwangseok/PycharmProjects/claude-harness

## 1. 목적

- 방식 A: `claude -p` 기반 stream-json 자식 프로세스.
- 방식 B: REPL 연결 기반 interactive 자식 프로세스.
- 두 방식을 동일 기준으로 비교한다. 결론 도출과 권고는 다음 단계 리뷰에서 다룬다.
- 코드 변경, 설정 변경, 테스트 추가는 이 단계 범위가 아니다.

## 2. 비교 기준

각 기준마다 근거(파일:라인 또는 출처)를 달아야 한다. 근거가 없으면 "미확인"으로 표기한다.

1. 토큰/컨텍스트 비용
2. 세션 상태 유지
3. 장애 복구
4. 사용량·rate 조회 가능성
5. OAuth 토큰 경계 (Go 코드가 토큰을 읽거나 저장하거나 전달하지 않는지)
6. 구현 복잡도
7. 테스트 용이성
8. 출력 파싱 안정성

## 3. 사실 수집 대상

읽기는 haiku-scout로만 수행한다. 묶음당 파일 3개 이하로 나눈다. 수신물은 요약만 받는다.

묶음 A (자식 프로세스 로직)
- internal/harness/livechild.go: 자식 프로세스 생성, 입출력, 종료 관련 함수 요약
- internal/harness/repl.go: REPL 연결 관련 함수 요약

묶음 B (오케스트레이션)
- internal/harness/orch.go: 관련 함수 요약
- internal/harness/orchws.go: 관련 함수 요약

묶음 C (사용량·rate)
- internal/harness/usagepoll.go: 관련 함수 요약
- internal/harness/ratestatus.go: 관련 함수 요약

묶음 D (외부 리서치 노트, 요약만)
- notes/research/subagent-context-model.md: 요약
- notes/research/prompt-cache-ttl-clear.md: 요약

수집 항목 형식 (scout 반환 기준)
- path:line, 함수명, 한 줄 역할, 방식 A/B 중 어디에 속하는지, 토큰 관련 여부

## 4. 제약

- 비교 분석 단계의 Opus 컨텍스트는 100k 토큰 이하로 유지한다.
- 원문 대량 로드 금지. 파일 읽기는 위임하고 요약만 받는다.
- 원문 인용이 필요하면 해당 줄 범위만 요청한다.

## 5. 산출물

- notes/runs/261031-plan-repl-vs-print-compare-review.md 는 다음 단계에서 작성한다.
- 이 파일(계획)에는 리뷰 내용, 결론, 권고를 적지 않는다.

## 6. 검증 기준

- 기준 1~8 각각에 대해 근거 파일:라인 또는 출처 URL/문서명이 있어야 한다.
- 근거가 없는 항목은 "미확인"으로 표기한다. 추정으로 채우지 않는다.
- 수집 요약과 실제 코드가 어긋나면 코드를 우선하고 차이를 기록한다.
- 기존 파일은 수정하지 않는다. 이 계획 파일만 신규 작성한다.

## 진행 체크리스트

- [x] 계획 파일 작성
- [ ] 묶음 A~D 수집 (haiku-scout)
- [ ] 기준 1~8 비교표 작성 (다음 단계)
- [ ] 리뷰 파일 작성 (notes/runs/261031-plan-repl-vs-print-compare-review.md)
