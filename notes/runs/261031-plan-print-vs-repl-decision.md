# 계획: claude -p(stream-json 상주) vs REPL(interactive, TTY) 판단
date: 2026-10-31
status: 검토 전 (추천 보류)
branch: interactive-child-compare

## 1. 질문
- 자식 프로세스를 claude -p(비대화형 print, stream-json 상주)로 갈지, REPL 연결(interactive, TTY/pty)로 갈지 판단·추천.

## 2. 확보된 측정 사실 (출처 명시)
- 4턴 누적 토큰 (opus, claude 2.1.314): REPL 166,481 / -p 78,290 (약 2.13배).
  - 기본 컨텍스트: REPL 약 41.5k / -p 약 19.5k, 요청당 약 22k 차이.
  - 출처: notes/runs/261031-result-repl-vs-print-cost.md
- -p total_cost_usd: 0.654877 (측정). REPL 달러 비용: 미확인.
  - 출처: notes/runs/261031-scope-repl-vs-print-cost.md
- REPL 실행 실패 이력 (측정 전 시행착오):
  - 신뢰 대화상자 기본값이 "No, exit"; "\r"이면 claude 종료, Down+Enter로 우회.
  - "Transcript saving is off": 상속된 CLAUDE_CODE_CHILD_SESSION 표식 때문에 세션 JSONL 미기록. 측정 소스 부재.
  - 1차 시도 턴1 타임아웃 180s (FAIL). 2차 시도에서 4턴 성공.
  - 출처: notes/runs/261031-scope-repl-vs-print-cost.md (fail1, fail2); /private/tmp/replcost.WorkyL/run.log:16 (턴1 타임아웃, 임시 로그라 notes 외부)
- 측정 조건 주의: REPL 측정 시 pty 자식 env에서 CLAUDE_CODE_CHILD_SESSION 등 표식 9종을 제거함 (임시 스크립트 수정, 제품 코드 변경 없음). 제거가 프롬프트 크기에 영향 줬는지: 미확인.
  - 출처: notes/runs/261031-result-repl-vs-print-cost.md (decision 줄)
- 기존 자식 프로세스 구조: 스폰은 internal/harness/core.go:143-154 (NewSession), args = -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model M; 상주 stdin/stdout; PTY 없음.
  - 출처: notes/runs/261031-haiku-livechild-spawn-audit.md
- usage 확인:
  - 5시간 사용률: 스트림 이벤트 rate_limit_event에서 읽음 (livechild.go:41-42, repl.go:308-309).
  - /usage: claude -p /usage 텍스트를 정규식으로 파싱 (usagepoll.go:21), 5분 주기.
  - Go 코드는 OAuth 토큰을 읽거나 저장하지 않음. ANTHROPIC_API_KEY만 제거 (core.go:23,27).
  - 출처: notes/runs/261031-haiku-usage-oauth-audit.md
- 기존 결정과의 관계:
  - notes/DECISIONS.md:47 [arch] 항목: 메인 기획서 검토 결과가 "interactive TUI 자식 대신 상주 stream-json 자식 유지"를 권고. 금지 규칙은 아니며, 브랜치명(interactive-child-compare)과 충돌하고 사용자 결정 대기 중으로 기록됨.
  - 메인 기획서: notes/runs/261031-plan-io-architecture-review.md (opus-architect). 권고 근거 4개: (1) 현 child가 이미 상주 세션이라 맥락 유지됨, (2) TUI 전환 시 비용·권한·알림 신호 상실 (261030-plan-interactive-child-review.md:32 인용), (3) Haiku 콜드스타트 약 $0.009로 저렴 (추정치), (4) Haiku 맥락 100k 초과 시 단가 5배 (notes/FACTS.md:10).

## 3. 비교 기준
| # | 기준 | -p 상주 (현행) | REPL (pty) | 근거 |
|---|------|----------------|------------|------|
| 1 | 토큰·비용 | 4턴 78,290 토큰, $0.654877 (측정) | 4턴 166,481 토큰 (측정), $ 미확인 | 2절 |
| 2 | 운영 안정성 (TTY/대화상자/표식 의존) | PTY 없음, 대화상자 무관 | 신뢰 대화상자·표식·턴1 타임아웃 이력 있음 | 2절 |
| 3 | 측정·파싱 신뢰도 | stream-json 이벤트, 세션 JSONL 기록됨 | JSONL은 표식 제거 후에만 기록됨; 측정 env 수정 상태 | 2절 |
| 4 | OAuth·약관 경계 | API 키만 제거, 토큰 미취급 | 측정 중 표식 제거 — 약관상 허용 여부 미확인 | 2절 |
| 5 | 기존 결정과 정합성 | 메인 기획서 권고와 일치 | 메인 기획서 권고와 불일치 (DECISIONS.md:47) | 2절 |
| 6 | 구현 복잡도 | 현행 유지 | pty 준비 대기, 훅 소켓, 전사 파싱 등 추가 필요 (plan-interactive-repl.md) | 추정 |

## 4. 미확인 / 추정 표기
- 미확인: REPL 달러 비용 (단가 미검증).
- 추정: REPL 비용 약 $1.4 (= 0.654877 × 2.1, 캐시 토큰 비례 가정). 추정 항목.
- 미확인: REPL의 약 22k 추가 컨텍스트 내용 (시스템 프롬프트·도구 등 추정).
- 미확인: 표식 제거가 print 측 프롬프트 크기에 영향 줬는지 (print 측도 설정 변수는 동일하게 유지).
- 미확인: REPL 측 5시간 사용률 수집 경로 (rate_limit_event 수신 여부).
- 미확인: 4턴 초과 및 캐시 TTL 만료 시 REPL 동작.
- 미확인: REPL 종료 코드 (캡처 안 됨).
- 미확인: 표식 제거 방식의 약관 경계 적합성.

## 5. 산출물
- 판단 (보류): 현재 측정만으로는 상주 stream-json(-p) 유지가 토큰 효율(2.13배 차이)과 안정성·측정 신뢰도에서 앞서 보이나, REPL 비용·표식 제거 영향이 미확인이라 추천은 보류한다.
- 근거 표: 3절 참조.
- 다음 단계 제안 (결정 아님): REPL 비용 측정 재현, 표식 제거의 프롬프트 영향 확인, 브랜치 목적 사용자 확인.
