# 261031-plan-verify-via-go
user_pushback: "왜 확인을 못하느냐, 해당 인터페이스에 go로 연결하고 go가 claude-code 내 데이터를 가져오는 게 가능하지 않느냐."
source_items: notes/runs/261031-plan-resident-sessions.md:48-60 (1-12), -review.md:15 (13)
goal: 13개 미확인 항목을 Go/로컬 데이터로 지금 확인 가능 / 부분 / 불가로 나누고, 가능한 항목은 실측

facts_used:
- go_launch: core.go:145-153 `-p --input-format stream-json --output-format stream-json --verbose --include-partial-messages --model <m>` (+--resume, --append-system-prompt). PTY 실행 없음 (creack/pty 없음)
- usage_source: usagepoll.go:54 `claude -p /usage --output-format json --no-session-persistence` -> session_pct, week_pct (1% 단위)
- stream_rate: ratestatus.go rate_limit_info.unifiedWindows.five_hour.utilization; 7d는 stream에 없음
- transcripts: ~/.claude/projects/-Users-leekwangseok-PycharmProjects-claude-harness/ 166 top-level jsonl + 1008 subagents/*.jsonl; entrypoint cli(인터랙티브)와 sdk-cli(-p) 모두 존재
- line_types: assistant,user,attachment,queue-operation,api-request,api-request-blob,api-request-shape,atis-latch,last-prompt,tools-used
- usage_obj: input/cache_creation/cache_read/output_tokens, cache_creation.ephemeral_1h/5m_input_tokens, iterations[]
- metrics: notes/metrics/*.jsonl stage=usage(session_pct) / stage=turn(result_* tokens)

options:
- A 기존 트랜스크립트·metrics 사후 분석 (Go 스캐너, stdlib): 비용 0, 과거 데이터라 조건 통제 불가
- B Go에 PTY 실행 추가(creack/pty) 후 동일 프롬프트 ABA: 완전 통제, 의존성·구현 필요
- C -p 쪽 소규모 실측(/usage 동시 호출, ps, /clear): 소량 사용량 소모
choice: A+C 지금 실행, B는 A 결과에서 모드 차이가 보이면 별도 계획. 이유: 다수 항목이 cli/sdk-cli 트랜스크립트로 답이 나오고 PTY 구현 전에 근거 확보 가능

classification:
- now_A: 2 시스템프롬프트·도구 동일성(api-request-shape 구조, 첫 턴 프롬프트 토큰), 4 인터랙티브 usage 기록, 5 요청 TTL 종류(ephemeral_1h vs 5m), 6 유휴 중 요청 흔적, 11 task-notification 기록, 13 부가 호출(api-request 분류 필드)
- now_C: 3 /usage 계정 단위(동시 2회 값 일치), 7 프로세스 RSS/CPU(ps), 10 -p stream-json /clear 처리
- partial: 1 -p 계상(metrics의 -p 턴 전후 session_pct 증가만, 1% 해상도, 사용자 동시 사용 오염), 8 429(과거 grep만, 부하시험 보류), 5 실제 만료 시간(요청 TTL만 확인, 만료 실험은 6분+ 대기)
- not_now: 9 PTY 훅·설정 병합(Go에 PTY 없음 -> 옵션 B), 12 tui 출력 격리(검증이 아닌 설계 항목)

steps:
1 haiku-coder: /tmp/ccscan/main.go + go.mod (module ccscan, stdlib only), `go run . <projdir> <metricsdir>`. 출력은 개수·키·타입·숫자만, 문자열 값 출력 금지(허용: entrypoint, type, model, 길이<=40이고 키 이름에 key/token/auth/email/secret 없는 열거형 값)
2 haiku-runner: E1 /usage 동시 2회, E2 ps, E3 /clear stream-json (모두 --model haiku, cwd=/tmp, --no-session-persistence)
3 haiku-batch-worker: 결과를 notes/runs/261031-verify-via-go-results.md 에 저장
4 B 필요 여부 판단(2·13에서 cli/sdk-cli 차이가 있으면 PTY 계획 작성)

edge_cases:
- api-request-blob은 요청 본문일 수 있음 -> 내용 출력 금지, 키·길이만
- entrypoint 없는 줄은 "none"으로 집계
- metrics의 session_pct는 사용자 인터랙티브 사용과 섞임 -> 증가 관측만 보고, 단위 환산 금지

acceptance:
- 13개 항목 각각 상태(확인/부분/불가) + 근거 숫자 1줄
- 비밀값·이메일·프롬프트 본문이 결과 파일에 없음
must_not_change: internal/harness/*.go, .claude/*, ~/.claude/* (읽기 전용)
