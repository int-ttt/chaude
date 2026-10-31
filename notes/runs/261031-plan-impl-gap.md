# 구현 gap 계획 (plan/caveman-structure.md "구현 점검" 기준)
source: plan/caveman-structure.md:255-279 (읽기 전용, 수정 안 함)
date: 2026-10-31
steps: 3 impl (A1-A3) + 1 audit (B) + 1 acceptance (C)

## A. 구현 필요 (MISSING)

### A1. I6 — .claude/agents/haiku-prompter.md 머리·R-블록 추가
- 변경: 3.6 머리 2줄(status/reason)을 return 블록 앞에 추가
- 변경: docs/claude-lean/CAVEMAN_FORMAT.md 3.2 스타일 R-block을 relay 반환용으로 추가
- 유지: 루프 문구, relay 문구 그대로
- executor: haiku-coder

### A2. I2 — haiku-prompter B형식 6키 추가
- 키: task, paths, ctx, limits, return, lang
- 위치: brief 읽기 절
- 기준: opus-architect.md:28-33 와 동일 형태
- executor: haiku-coder
- 주의: A1과 같은 파일이므로 한 번에 수정 권장

### A3. I13 — internal/harness/orchws.go parseWS "none" 허용
- 변경: ws 값 "none" 을 workspace 없음으로 인식, id 빈 문자열 반환
- 현재: orchws.go:11,14-20 에 none 분기 없음, [ws:none] 은 id "none" 으로 통과
- 테스트: orchws_test.go 에 none 케이스 추가
- executor: haiku-coder

## B. 부분 완료 (PARTIAL) — 설명만, 코드 변경은 명백한 경우만

### B1. I9 — 일반 존댓말 문구 vs R-read/R-write
- 근거: haiku-scout.md:16, haiku-researcher.md:15
- 상태: 일반 존댓말 문구가 R-read 절과 충돌 잔존
- 처리: 설명. 충돌 문구 목록 확정 후 판단 (scout/researcher/coder/drafter/checker/batch-worker/runner/opus-architect 대상 점검)
- executor: haiku-scout (audit)

### B2. I14 — main-thread-guard / agent-model-pin 변경
- 근거: main-thread-guard.py:32, agent-model-pin.py; 마지막 변경 커밋 b4855b7
- 상태: plan §9 "동작 안 바꿈" 과 충돌
- 처리: 설명. 변경 내용 diff 확인 후 사용자 확인 필요 (코드 변경 없음)
- executor: haiku-scout (audit)

## C. 단계별 acceptance
- python3 -m unittest discover -s tests/hooks -p 'test_*.py'
- go build ./...
- go vet ./...
- go test ./internal/harness/... -count=1
- 실행: haiku-runner

## D. executor 배정
- A1, A2, A3: haiku-coder
- C (acceptance): haiku-runner
- B1, B2 audit (근거 확인): haiku-scout

## 미확정 (unclear)
- I2: §4 표 예외 여부 확인 필요 (A2 전 확인)
- I14: b4855b7 변경이 §9 위반인지 사용자 판단 필요
- I9: 충돌 대상 범위 (어느 에이전트 파일의 어느 문구까지 수정할지) 미정
- A3: "none" 입력 시 에러 대신 빈 id 반환이 맞는지 확인 필요 (계획 문구 기준으로는 빈 id)
