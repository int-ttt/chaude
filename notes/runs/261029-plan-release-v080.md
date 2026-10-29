# v0.8.0 릴리즈 오케스트레이션 계획 (멀티프로세스 구현 마무리)
작성 261029. 근거: plan-multiproc-impl.md, LEDGER 말미, 코드 grep(함수 존재 여부), go build/vet/test 결과. 코드 수정 없음.

## 0. 현재 상태 요약
- 사용자 지시: "멀티클로드 구현 후 v0.8.0으로 릴리즈해" (pastes/9e25910e.md L44). 구현 계획 문서는 버전을 0.7.3으로 적었음 -> v0.8.0으로 읽어 대체 [추정, 확인 필요: 아래 차단 2].
- 현재 `go build ./...`, `go vet ./...` exit 0, `go test ./...` exit 0이나 harness 결과가 (cached) [확인]. 릴리즈 전 `-count=1` 재실행 필요.
- 버전 위치: cmd/chaude/main.go:16 (`0.7.2`). CHANGELOG.md 맨 위는 0.6.6 (0.6.7~0.7.2 누락, 건드리지 않음 [계획 5절]). notes/OPEN.md, notes/DELIVERABLES.md는 존재하지 않음 [확인].
- 승인 문구: pastes/9e25910e.md에 "승인 문구는 없었습니다" (L34). CLAUDE.md/DECISIONS.md 수정과 U0a 커밋·U10a는 승인 대기.

## 1. 유닛 표
| 유닛 | 상태 | 근거/대상 파일 | 검증 명령 |
|---|---|---|---|
| U0a 미커밋 작업 별도 커밋 | 미완료, 승인 없음 | classifier.go, conductor_test.go, repl.go, tui.go, ratestatus*.go, notes | git status; 사용자 승인 필요 |
| U0b 1h 캐시 토큰 게이트 | 미완료(기록 없음) | haiku-runner 1회 호출 | `claude -p --output-format stream-json --verbose "hi"`에서 ephemeral_1h_input_tokens |
| U1a clients.go 레지스트리 | 완료 | LEDGER 16:32 | `go test ./internal/harness -run TestClientRegistry -count=1` |
| U1b isWarm | 완료(코드·TestIsWarm 존재) | clients.go | `-run TestIsWarm` |
| U1c idleDeadline | 완료 | clients.go, TestIdleDeadline | `-run TestIdleDeadline` |
| U2a withCacheTTL | 완료 | core.go:41, cachettl_test.go | `-run CacheTTL` |
| U2b observedTTL | 완료 | core.go:78 | `-run CacheTTL` |
| U3a parseCacheTTL | 완료 | repl.go:868 | `-run CacheTTL` |
| U4a childKey | 완료 | children.go | `-run TestChildKey` |
| U4b childSet | 완료 | children.go, children_test.go | `-run 'TestChildSet\|TestCloseAll'` |
| U6a pickChild | 완료 (추가: resolveMaxWorkers, checkWorkerCap) | dispatch.go, dispatch_test.go | `-run 'TestPickChild\|TestMaxWorkers'` |
| U3b Main 플래그 | 부분: 플래그 3개 정의됨(repl.go:902-904). Args/SessionOpts.CacheTTL 전달·레지스트리 생성 여부 미확인 | repl.go | `go build ./... && go test ./internal/harness -count=1` |
| U5 touchMain | 미완료(함수 없음) | repl.go | 신규 테스트 + `go test ./internal/harness -count=1` |
| U6b classifier 연결 | 미완료(classifyFn 없음) | classifier.go | 동일 |
| U7 idleDue | 미완료 | repl.go | 동일 |
| U8 sweepIdle/CloseAll 연결 | 미완료 | tui.go | 동일 |
| U9 버전·CHANGELOG | 미완료 | cmd/chaude/main.go:16, CHANGELOG.md | `go build ./... && go run ./cmd/chaude --version` |
| U10a DECISIONS 기본값 auto | 미완료, 승인 필요 | notes/DECISIONS.md | 승인 문구 확인 |
| U10b DECISIONS 직렬 디스패치 범위 | 미완료, 승인 불필요 | notes/DECISIONS.md | grep |

집계: 완료 10 (U1a,U1b,U1c,U2a,U2b,U3a,U4a,U4b,U6a + 추가 dispatch 캡), 부분 1 (U3b), 미완료 8 (U0a,U0b,U5,U6b,U7,U8,U9,U10a/b 중 해당). 정확히는 유닛 U 기준 완료 9, 부분 1, 미완료 U0a,U0b,U5,U6b,U7,U8,U9,U10a,U10b = 9.
주의: LEDGER에는 U1a만 기록됨. 나머지 "완료"는 코드 존재와 테스트 통과(캐시)로 판단한 것이므로 첫 단계에서 재검증.

## 2. 차단 요소
1. U0a 커밋 승인 없음. 승인 전에는 0.8.0 커밋 금지(계획 5절). 승인 문구를 사용자에게 요청.
2. 버전 번호: 계획은 0.7.3, 사용자 말은 v0.8.0. v0.8.0으로 진행해도 되는지 확인(릴리즈 단계 전).
3. U10a는 승인 문구 필요(DECISIONS 수정). 미승인이면 건너뛰고 기본값 1h 유지, CHANGELOG는 "TTL 관측"으로.
4. 라이브 게이트: DECISIONS L33 "TestClassifierLive (CHAUDE_LIVE=1) 후 릴리즈" 관례 -> 실행 여부 사용자 확인(비용).

## [haiku] steps
순서 표기: S=직렬, P=병렬.
1. (S) haiku-checker: `cd repo && go build ./... && go vet ./... && go test ./... -count=1` 결과 보고, 위 "완료" 유닛의 테스트 이름별 PASS 확인. 수정 금지.
2. (P, 1 이후) haiku-checker: repl.go Main에서 CacheTTL/IdleRotate/ClientsFile이 실제 Args·SessionOpts·registry/childSet 생성에 전달되는지 확인(U3b 남은 부분 범위 산정).
2'. (P) haiku-runner: U0b 게이트 1회(소액 한도 내). 결과를 notes/runs/에 기록.
3. (S, 2 이후) haiku-coder: U3b 잔여(전달 연결) - repl.go. 검증: `go build ./... && go test ./internal/harness -count=1`.
4. (S) sonnet-analyst(설계 결합부, Haiku 부적합): U5 touchMain -> U6b classifier 연결 -> U7 idleDue -> U8 tui sweepIdle/CloseAll. 이 4개는 repl.go/classifier.go/tui.go가 얽혀 직렬. 각 유닛마다 `go build ./... && go vet ./... && go test ./internal/harness -count=1`. 각 유닛 뒤 LEDGER 기록.
   - U7은 순수 함수 부분만 haiku-coder에 위임 가능(idleDue 테스트 포함).
5. (S) haiku-coder: U10b DECISIONS 직렬 디스패치 범위·"상한에 세는 것" 정의 추가(notes/DECISIONS.md). 승인 불필요. 단계 4와 병렬 가능(파일 겹침 없음). 단 CLAUDE.md는 건드리지 않음.
6. (S, 4 이후) haiku-checker: 전체 검증 `go build ./... && go vet ./... && go test ./... -count=1`.
7. (S) sonnet-reviewer: 0.8.0 diff 1회 검토(U0a 커밋 후 범위).
8. (S, U0a 승인·검토 PASS 후) haiku-coder: U9 - cmd/chaude/main.go:16 `var version = "0.8.0"`, CHANGELOG.md 맨 위 `## 0.8.0` 추가. 검증: `go build ./... && go run ./cmd/chaude --version` 출력 0.8.0.

## 3. 검증 기준
- 매 유닛: `go build ./...` exit 0, `go vet ./...` exit 0, 해당 `go test -run <이름> -count=1` PASS.
- 릴리즈 직전: `go test ./... -count=1` 전부 ok, (cached) 결과 불인정.
- 하나라도 실패하면 중단하고 보고(Haiku 2회 실패 시 sonnet-analyst로 승격).

## 4. v0.8.0 릴리즈 게이트 (계획만, 실행 금지)
1. 전제: U0a 커밋 승인 및 완료, 단계 6·7 PASS, 라이브 게이트 결정 완료.
2. 버전 파일: cmd/chaude/main.go:16 -> "0.8.0" (README 버전 표기는 0.6.6이라 별도 판단, 이번 범위 아님).
3. 변경 로그: CHANGELOG.md `## 0.8.0` - 모델별 세션 유지(직렬 디스패치, 동시 워커 상한), 유휴 3599초 후 종료, 캐시 TTL 관측, size tag 기본 large, 5h 사용량 상태줄. "multi-client/동시 실행" 표기 금지(계획 5절).
4. 커밋 메시지(예): `0.8.0: per-model live child processes, idle rotate at 3599s, cache TTL observe`. 마지막 줄에 지시된 대로 `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`. (세션 시스템 지침은 Sonnet 5.5 줄이나, 이번 지시는 Haiku 5.5 줄을 명시 -> 지시를 따름, 불일치는 사용자에게 한 줄 확인.)
5. 태그 `v0.8.0`, push는 사용자 명시 요청 시에만. release.yml이 "Changes since <prev tag>"를 자동 생성(DECISIONS L34).
