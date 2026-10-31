# plan: -p (stream-json resident child) optimization
source: notes/runs/261031-plan-print-vs-repl-decision-review.md
decision: -p 상주 자식 유지, REPL 비채택(실험 전용)
status: plan only, no code changed

## items
o1: 요청당 고정 base context(~22k 토큰 REPL 대비 차이는 REPL 쪽; -p는 ~19.5k) 추가 증가 방지 - 시스템 프롬프트/툴 정의 불필요 확장 금지
o2: Haiku 100k 임계(FACTS.md:10, 5x 가격) 근접 추적 - turnspend/usagebar에서 입력 토큰 누적이 임계에 가까우면 회전(rotate) 또는 경고
o3: 캐시 TTL 활용 - idle rotate(3599s) 유지, 캐시 warm 동안 cache_read 비율 측정해 달러 비용 기준으로 보고 (2.13배 대신 "요청당 +약 22k" 표현)
o4: stream-json 신호 활용 강화 - total_cost_usd, usage, rate_limit_event를 metrics/ratestatus/usagebar에 일관 반영
o5: 측정 보강 - -p 10턴 이상 달러/쿼터 측정 기록(reversal r3 판단용), REPL 달러 비용은 미측정으로 표기
o6: REPL 실험 코드는 측정 스크립트 전용 유지; 제품 코드에 env-marker 제거 금지

## must not change
- internal/harness/core.go NewSession args (-p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model M)

## open question (user)
q1: REPL 실험 코드를 interactive-child-compare 브랜치에 비교용으로 남길지, 정리할지
