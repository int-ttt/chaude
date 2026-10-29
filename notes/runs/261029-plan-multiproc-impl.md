# v0.7.3 다중 프로세스 구현 계획 (코드 수준)

date: 261029
status: 문서만. 소스 미수정. 리뷰(plan-multiproc-impl-review.md) 반영판.
inputs: plan-multi-client-v3.md, plan-v0.8-impl.md(+review), research/client-assumptions.md, explore-harness-spawn.md, plan-multiproc-impl-review.md
표기: [확인] = 코드/문서 확인. [가정] = 근거 없음(검증법 병기). [해석] = 이 문서의 판단. [승인필요] = 사용자 승인 전 진행 불가/보류.
기본값: Q3 설계 클라이언트는 3599초에 닫고 설계 파일 경로로 재개 / Q4 설계 1 + 실행 1, 모델당 1개 / 캐시 TTL 기본 1h(승인 전, DECISIONS:23 유지), 5m 관측 시 290초 / 3599초 핸드오프는 기존 rotate 사용.

## 0. 조사 요약 (모두 [확인], 출처 explore 노트)
- 자식 시작: core.go:108 `NewSession(SessionOpts)`; 인자 `-p --input-format stream-json ... --model o.Model`(core.go:115-122); env `cleanEnv(compactWindow)`(core.go:125). 모델: Args.Model -> repl.go:177 `start()` -> SessionOpts.Model.
- 테스트 이음새: `newSessionFn`(repl.go:174), 10여 개 테스트가 `&Repl{...}` 리터럴 사용.
- 추적: `Repl.sess`(단일), `draining []*drain`, `pool *workerPool`, `parked map[model]sessionID`(resume용, 프로세스 아님).
- 회전: `rotate(reason)`(repl.go:571) = handoff 턴 -> `start()` 새 자식 -> 옛 자식 drain. 트리거: ctx 임계(:764), 카테고리(:757), manual(:830, tui.go:1135), `/model`(tui.go:1150), 라우트(classifier.go:196).
- 라우트: `route()`(classifier.go:145) -> `switchSession`(:204)이 현재 자식을 닫고(:213) `parked`에 sessionID 저장, 복귀는 resume(:183).
- 버전: cmd/chaude/main.go:16 `var version = "0.7.2"`. CHANGELOG.md는 `## 0.6.6`에서 멈춤.
- 작업 트리 미커밋 변경(classifier.go, conductor_test.go, repl.go, tui.go, ratestatus*.go, LEDGER, metrics). 리뷰 확인: vet 청결, test PASS, sonnet-reviewer PASS(LEDGER 10-29 13:18).

## 1. 범위
0.7.3 포함:
1. 레지스트리 `~/.chaude/clients.json`: 행 키 = (cwd 절대경로, model). 행 {Cwd, Model, Role(design|exec), SessionID, State(live|parked|expired), LastAPI, LastCtx, EndedReason, DesignPath, OwnerPID, ChildPID}. (`LastUsed` 대신 `LastAPI`로 통일)
2. 모델별 live 자식 유지: **모델이 다를 때만** 이전 자식을 닫지 않고 parked-live로 둠. 같은 모델 재배정은 프로세스 재사용(캐시 유지). 최대 동시 live = (cwd당) opus 1(설계) + sonnet 1 + haiku 1.
3. 디스패치: (cwd, model) 자식에 전달. 없으면 레지스트리 resume, 불가하면 새로 시작.
4. 유휴 3599초(`--idle-rotate`, 5m TTL 관측 시 290초): 활성 자식만 `rotate("idle 3599s")` 핸드오프 후 닫기, 비활성 live 자식은 핸드오프 없이 Close. State=expired.
5. `--cache-ttl auto|1h|5m`(승인 전 기본 1h) + 관측 TTL(`usage.cache_creation`) 기록, 5m 관측 시 290초 폴백.
6. 정상 종료 시 모든 live 자식 병렬 정리. 강제 종료는 다음 시작에서 레지스트리 정리(§2).
7. 버전 0.7.3 + CHANGELOG(미커밋 두 항목도 기재).

0.7.3 제외(보류):
- **동시 실행은 0.7.3에 없음.** 여러 프로세스가 살아있고 한 번에 한 자식만 턴을 수행하는 직렬 디스패치. 사용자 요청(v3 §1 "동시에 실행")을 완전히 충족하지 않음 -> 사용자에게 통지(차단 아님, 구현 진행). childSet/registry/pickChild는 동시 실행의 선행 조건이라 버려지는 작업 없음.
- 0.8 노트(유지): 최소 동시 실행 = "자식당 백그라운드 턴 1개". 활성 자식 A가 턴 중일 때 프롬프트가 모델 B로 라우트되면 A를 턴 중 parked-live로 두고(StartIdle/drain 패턴으로 이벤트 펌프 유지) B에 전달, A의 최종 결과는 라벨 알림으로 표시. 자식당 대기 프롬프트 1개(FIFO 없음), 바쁜 중 입력은 다른 idle 자식으로 라우트될 때만 수락. 그 외(dispatch 블록, 결과 경로 폴링, `[clients]` 요약, `## clients` 의도 파일, 핑, 캐시 공유 검증, 레지스트리 파일 락)는 계속 보류.
- "design first" 게이팅 없음(v3 §4 보류). CHANGELOG 0.6.7~0.7.2 공백 보충 안 함.
- haikupool: `rr.Worker`(classifier.go:176) 우선순위 유지, 상한에 안 셈. Task 서브에이전트, drain 중 옛 세션도 안 셈.

## 2. 프로세스 모델
- 키: `childKey(cwd, model)`: cwd가 ""면 os.Getwd, 이후 filepath.Abs + EvalSymlinks(macOS /var vs /private/var) + "\x00" + model. 동일 키 live 자식은 1개.
- 메모리: `childSet` = map[key]*liveChild. `liveChild{Sess *Session; Tracker *agentTracker; Model, Role string; LastAPI time.Time; LastCtx int; Files []string; TurnFrom int}`. 활성 자식은 `r.sess`, 나머지는 childSet에만(parked-live).
- Park는 r.tracker, r.lastCtx, r.files, r.tr.turns 시작 인덱스를 저장, useLive가 복원(없으면 상태줄 ctx와 rotate 임계(repl.go:764)가 다른 모델 ctx를 씀). **Park 시 StopIdle 호출 금지**(start() repl.go:212의 idle 펌프가 stdout을 계속 비워야 함, 아니면 parked 자식이 파이프 가득 차 멈춤).
- 디스패치 `(r *Repl) dispatch(model)`:
  1. 현재 활성 키와 같으면 아무것도 안 함.
  2. 활성 자식을 Park, 레지스트리 State=live 갱신(r.parked에는 쓰지 않음, 레지스트리가 id 보관).
  3. `pickChild`: useLive(childSet에 있고 isWarm) / resume(행 parked, isWarm, OwnerPID가 살아있고 != os.Getpid()인 행은 건너뜀: 다른 chaude가 점유) / startNew.
  4. useLive면 `r.sess = child.Sess`, tracker/transcript 재바인딩(start()의 repl.go:180-181 `tk.parent=s.ID`) 및 `delete(r.parked, model)`(낡은 id 재개 방지). 가장 위험한 지점(A3).
  5. 작업 프롬프트는 기존 경로로 활성 자식에 Send.
- **같은 모델(haiku 워커 전환)은 기존 park-resume 경로 유지.** `rr.Worker != ""`(classifier.go:176-179)인 haiku->haiku 전환은 `switchSession("haiku", rr.Worker)`를 지금 그대로 둠(키 (cwd, model)로는 live haiku 1개만 표현 가능). 모델이 다를 때(haiku<->비haiku 포함, classifier.go:181-198)만 `dispatch`/`pickChild`. :183의 `r.a.Model == "haiku"` 제한은 live 경로에서 제거. 레지스트리/childSet이 nil이면 route() 전체가 기존 그대로.
- 유휴 처리: 입력 대기 중 tick(spinner.TickMsg, 1초 제한)에서 `sweepIdle(now)`:
  - **활성 자식만** 마지막 회전 이후 턴이 있을 때 `rotate("idle 3599s")`(!busy && perm==nil, 사용자 프롬프트와 같은 경로, Update 루프 안 직접 호출 금지) -> 핸드오프를 pendingSnapshot에 보관 -> 닫기 -> State=expired. 같은 LastAPI에 1회만.
  - 비활성 live 자식이 deadline 도달: 핸드오프 없이 Close, State=expired, 다음 사용은 startNew. (rotate()가 혼합 모델 r.tr.turns/r.sess를 읽으므로 비활성 자식을 활성으로 올리지 않음 -> UI 모델이 바뀌는 것 방지.) opus 행은 DesignPath 한 줄 첨부(Q3 유지).
- 타이머 기준: 주 이벤트 `stream_event message_start` 시각(`isMainEvent`, 서브에이전트 제외). isWarm은 idleDeadline과 같은 기준.
- 비활성 세션 이벤트(`idleEvent`의 s != r.sess): 활성 상태를 건드리지 않음. usage -> 해당 liveChild.LastCtx(r.idle.usage/r.lastCtx 아님); 텍스트는 r.idleText로 스트리밍 안 하고 버퍼링 후 결과 때 dim 블록 `[<model> 대기 세션] <text>` 한 번 표시; 권한 프롬프트 라벨 앞에 `[<model>]`.
- ctx 한도: `a.Threshold`(기본 80000), `effectiveThreshold()`(repl.go:695) 그대로.
- **DesignPath**: 작성자는 Opus(설계) 자식(opus-architect가 이미 `notes/runs/<YYMMDD>-plan-<topic>.md`를 씀). chaude는 요청/생성하지 않고 **Write 이벤트 경로만 기록**: runTurn의 Write/Edit tool_use 분기(repl.go:452-463, `r.pool.noteEdit` 옆)에서 활성 모델이 opus이고 `file_path`가 `*/notes/runs/*-plan-*.md`이며 `*-review*.md`가 아니면 opus 행 DesignPath = 절대경로(마지막 기록 우선). `idleEvent` tool_use(백그라운드 턴)에서도 동일. 미기록이면 첨부 없이 일반 핸드오프만, 경고 없음. DesignPath가 있으나 첨부 시점에 파일 없음/크기 0이면 첨부 생략 + metrics에만 `{"stage":"design_path","missing":true,"path":...}`, 사용자 오류 표시 없음, 디스패치 차단 안 함. 첨부문: `설계 파일: <abs path> (먼저 읽고 이어서 진행)`을 pendingSnapshot 끝에 한 줄.
- **종료/강제 종료**: PID 직접 kill 없음(Setpgid 추가 금지).
  - 정상 종료(TUI quit tui.go:1117-1122, 루프 종료 repl.go:~930): `children.CloseAll()`이 모든 live 자식을 **병렬** 종료(Close마다 goroutine + WaitGroup; Close는 stdin 닫기 -> 15초 대기 -> Kill, 직렬이면 최대 45초). 레지스트리 행 State=parked, EndedReason="exit", SessionID 유지.
  - SIGINT/SIGTERM: 기존대로 interrupt 전달(repl.go:912-920). SIGHUP/SIGKILL/크래시: 정리 코드 없음. 자식은 chaude 프로세스 그룹이라 SIGHUP은 직접 도달, SIGKILL 시 stdin EOF로 `claude -p --input-format stream-json`이 종료됨 [가정, 아래 수동 검증].
  - 레지스트리 정리: 다음 시작 때 `r.agents.interruptRunning(false)`(repl.go:903) 옆에서 `r.reg.reconcile(now, deadline)`: State=live이고 `!pidAlive(OwnerPID)`(agentstore.go:95 재사용)인 행 -> SessionID!="" && now-LastAPI<deadline이면 parked, 아니면 expired; EndedReason="harness-killed". 살아있는 다른 chaude 소유 행은 건드리지 않음.
  - 수동 검증: chaude 시작, 2모델 라우트, `kill -9 <chaude pid>`, 20초 후 `pgrep -f 'claude -p --input-format stream-json'` = 0; 재시작 시 clients.json 행 parked + harness-killed. 고아가 남으면 후속으로 kill 폴백(ChildPID 생존 && `ps -o ppid=`==1 && comm에 claude)을 추가.

## 3. 변경 파일과 유닛
규칙: 유닛 = 파일 1개 + 함수 1개(또는 한 타입의 CRUD), `go test` 가능. [순서] = 선행 필요. H = haiku-coder, S = sonnet-analyst. S에게는 리뷰 문서와 §2를 함께 전달.

U0a (사용자 승인 필요 [승인필요]) 미커밋 작업 트리를 **0.7.3 시작 전에 별도 커밋**. 순서: (1) 코드: `classifier: size tag defaults to large; status line: 5h rate usage`(classifier.go, conductor_test.go, repl.go, tui.go, ratestatus.go, ratestatus_test.go; 버전 올리지 않음) (2) 메트릭: `notes/LEDGER.md` + `notes/metrics/2026-10-29.jsonl`을 `metrics: 2026-10-29`(b6e057f 패턴) (3) notes (notes/runs/*.md, notes/research/*)는 문서 커밋 또는 untracked 유지 (4) 그 후 0.7.3. **전부 사용자 승인 후**; 승인 전에는 repl.go/classifier.go/tui.go를 건드리는 유닛(U3b, U5, U6b, U7, U8)과 릴리즈 단계가 대기. 순수 유닛(U1a-U1c, U2a, U2b, U3a, U4a, U4b: clients.go/children.go/core.go만)은 먼저 시작 가능. 비차단 점검: ratestatus.go `if v < 1 { v *= 100 }`는 분수 1.0(100%)을 "1%", 퍼센트 0.5를 "50%"로 표시 -> `notes/metrics/*.jsonl`의 rate_limit_info 실제 단위 확인 후 분수 전용이면 `v <= 1`.
U0b (haiku-runner, 호출 1회) 게이트: `claude -p --output-format stream-json --verbose "hi"`에서 `ephemeral_1h_input_tokens` 존재 여부(소액 호출, §4 비용 상한 적용). 없으면 U2b 폴백(5m 가정, 300초 초과 후 cache_read>0이면 1h).

독립 그룹 (병렬 가능):
- U1a H `internal/harness/clients.go`: `clientRec` + `clientRegistry` CRUD(`newClientRegistry(path)`, `Get`, `Upsert`, `MarkEnded(cwd,model,reason,now)`, `defaultClientsFile()`, `reconcile(now, deadline)`). agentstore.go 패턴(tmp+rename). 저장 시 파일을 다시 읽어 자기 (cwd, model) 행만 교체하고 다른 행 보존. OwnerPID/ChildPID 기록(`Session.PID() int` = cmd.Process.Pid, 가짜는 0 필요).
- U1b H `clients.go`: 순수 `isWarm(rec, limit, now, deadline)` (State live|parked, now-LastAPI < deadline, LastCtx < limit).
- U1c H `clients.go`: 순수 `idleDeadline(idleRotate time.Duration, ttl string)`; 호출자가 `r.ttlSeen`이 비어있지 않으면 그것, 아니면 플래그값 전달. "1h" -> idleRotate; "5m" -> min(idleRotate, 290s); "auto"이고 미관측 -> 290s; idleRotate 0 -> 0.
- U2a H `core.go`: 순수 `withCacheTTL(env, ttl)`; `SessionOpts.CacheTTL`; core.go:125를 `cmd.Env = withCacheTTL(cleanEnv(o.CompactWindow), o.CacheTTL)`. cleanEnv 시그니처 변경 금지(unit_test.go:82,88). "1h"/"5m"만 env 설정.
- U2b H `core.go`: 순수 `observedTTL(usage, prev)` (1h>0 -> "1h"; 5m>0 -> "5m"; 없으면 prev).
- U3a H `repl.go`: 순수 `parseCacheTTL(s)` (auto|1h|5m 외 오류).
- U4a H 신규 `children.go`: `childKey(cwd, model)`.
- U4b H `children.go`: `childSet`, `liveChild`, `Park`, `Take`, `Expired(now, deadline)`, `CloseAll()`(병렬). Session은 `closer` 인터페이스. [순서] U4a 후.

직렬 그룹 (S 담당; U0a 승인 후):
- U3b S `repl.go` Main: 플래그 `--cache-ttl`(승인 전 기본 1h), **`--idle-rotate`**(3599s, 0=끔; DECISIONS:21/23 확정 이름, 신규 `--idle-end` 쓰지 않음), `--clients-file`(기본 defaultClientsFile(), ""=끔), `parseCacheTTL`, Args와 `SessionOpts.CacheTTL` 전달. [순서] U2a, U3a.
- U5 S `repl.go`: `touchMain(ev, now)` (isMainEvent && message_start -> r.lastAPI, 레지스트리/childSet LastAPI 갱신; assistant usage -> `r.ttlSeen = observedTTL(...)`). 호출은 runTurn 이벤트 경로와 `idleEvent`(repl.go:217). 이 위치에 DesignPath 기록(§2)과 비활성 세션 이벤트 처리 추가. Repl 필드 `children`, `reg`, `lastAPI`, `ttlSeen`(nil 허용). [순서] U1a, U1c, U2b, U4b.
- U6a S 신규 `dispatch.go`: 순수 `pickChild(set, reg, key, cwd, model, limit, now, deadline) childAction`. [순서] U1b, U4b.
- U6b S `classifier.go`: 모델이 다를 때 `switchSession` 대신 `childSet.Park` + `pickChild`; 같은 모델(haiku 워커) 본문은 그대로. 레지스트리 nil이면 기존 경로. 이음새 `var classifyFn = classify`. [순서] U5, U6a.
- U7 S `repl.go`: `idleDue(now)` (deadline>0, 회전 이후 턴 있음, now-lastAPI >= deadline, 같은 lastAPI 재발화 없음). [순서] U5.
- U8 S `tui.go`: spinner.TickMsg 1초 제한에서 `sweepIdle`(§2), 종료 경로 `CloseAll()`. 비TTY REPL은 타이머 없음(문서화). !busy일 때 spinner tick 확인(A6). [순서] U7, U6b.
- U9 H `cmd/chaude/main.go:16` version "0.7.3" + `CHANGELOG.md`. [순서] 모든 테스트 PASS 및 U0a 승인 커밋 완료 후.
- U10a [승인필요] `notes/DECISIONS.md`: 기본값 auto로 변경 + DECISIONS:23 "pin" 줄 대체(승인 후 기본값 한 줄 변경과 같이).
- U10b (언제든, 사용자 결정 변경 없음) DECISIONS: 직렬 디스패치 범위, "상한에 세는 것" 정의(에이전트 노트).

실행 순서: U0a(승인) -> U0b -> 순수 유닛 병렬 -> U4b -> U3b -> U5 -> U6a -> U6b -> U7 -> U8 -> 전체 검증 -> U9 -> U10. 끝에 sonnet-reviewer 1회(0.7.3 diff만; U0a 커밋 후라 classifier/ratestatus 작업 제외).
건드리지 말 것: `rotate()` 핸드오프/검증 로직, drain 경로(DECISIONS:18), haikupool 선택, cleanEnv의 ANTHROPIC_API_KEY 제거, `switchSession` 같은 모델 본문, `Session.Close` 의미, Setpgid 추가, `r.agents.interruptRunning` 동작.

## 4. 테스트
좁은 실행: `go test ./internal/harness -run 'Client|TTL|Idle|Warm|Touch|Child|Pick|Route|Reconcile|Design'` 후 `go test ./... && go vet ./...`.
- TestClientRegistryRoundTrip / BrokenJSON(빈 레지스트리, panic 없음) / AtomicWrite(.tmp 잔존 없음) / KeyedByCwdModel(행 안 섞임, 다른 cwd 행 보존).
- TestIsWarm: (deadline-1s) true, deadline false, LastCtx>=limit false, expired false.
- TestIdleDeadline: (flag 1h, seen 5m)->290s; (flag auto, seen 1h)->3599s; 1h->3599s; 5m->290s; ""->290s; idleRotate 0->0.
- TestWithCacheTTL / TestObservedTTL / TestParseCacheTTL(10m 오류).
- TestChildKey: ""와 "."가 같은 cwd와 같은 키. TestChildSetParkTake: 같은 키 1개 유지, Take 후 제거, Expired 정확, CloseAll이 모든 closer 호출.
- TestCloseAllParallel: 가짜 closer 3개(각 100ms) 합계 < 250ms.
- TestTouchMainOnly: 서브에이전트 이벤트는 lastAPI 불변, 주 message_start는 갱신.
- TestIdleEventFromParkedChild: r.lastCtx 불변, idleText 호출 없음.
- TestPickChild: live+warm->useLive; parked warm->resume; 만료/ctx 초과->startNew; 다른 살아있는 OwnerPID 행은 resume 제외.
- TestIdleDue: 3598초 false, 3599초 true, 같은 lastAPI 두 번째 false, 회전 후 턴 없으면 false.
- TestReconcileKilledOwner: OwnerPID 죽은 live 행 -> parked(SessionID 있고 deadline 이내) 또는 expired, EndedReason=harness-killed; 산 OwnerPID 행 불변.
- TestRouteKeepsPreviousChildLive: haiku -> sonnet -> haiku에서 newSessionFn 2회, 세 번째 재사용, 이전 자식 Close 0.
- TestRouteSameModelWorkerUsesOldPath: haiku 워커 전환은 Close 1회, childSet 불변.
- TestRouteFallsBackWithoutRegistry: 레지스트리 nil이면 기존 동작(기존 conductor_test 회귀 0).
- TestDesignPathRecorded: opus가 notes/runs/x-plan-y.md Write -> 행 설정, `-review.md` 무시. TestDesignPathMissingSkipped: 파일 없음/0바이트 -> 첨부 생략 + metrics만, 오류 없음.
acceptance: 위 전부 PASS, 기존 회귀 0, vet 통과, `go build -o /tmp/chaude ./cmd/chaude && /tmp/chaude --version` = 0.7.3. 수동: `chaude --idle-rotate 5s`로 5초 후 핸드오프+닫기, clients.json State=expired, 다음 프롬프트에 스냅샷 전달; 종료 후 `ps` 고아 0; kill -9 시나리오(§2).

**라이브 테스트 비용 규칙 [사용자 지시]**: 실제 모델을 호출하는 모든 라이브 테스트 태스크(U0b 게이트, 수동 3모델 왕복, TTL 관측, resume 캐시 적중 등)는 태스크당 비용 상한 **$3**.
- 집행 방식 [가정, 검증 전]: (1) CLI가 예산 플래그(예: `--max-budget-usd`)를 지원하면 모든 라이브 호출에 `--max-budget-usd 3`을 붙임. 지원 여부는 `claude --help`로 U0b 전에 확인. (2) 미지원이면 테스트 하네스에서 하드 스톱: 각 이벤트의 `usage`(input/output/cache_read/`cache_creation`)와 결과 이벤트의 `total_cost_usd`가 있으면 그것을 누적해 $3 초과 시 즉시 자식 Close + 태스크 중단 + 보고. 단가표로 환산할 때 cache_creation은 1h(2x)/5m(1.25x) 구분 적용. (3) 둘 다 검증 전이므로 첫 라이브 태스크 전에 한 번 소액으로 집행이 작동함을 확인(미확인이면 라이브 테스트 시작 금지). 하네스는 태스크 단위 누적기를 두고 태스크 간 합산하지 않음.

## 5. 릴리즈 절차
1. U0a 커밋(사용자 승인)이 선행. 테스트/vet/빌드 PASS 확인. 하나라도 실패 시 중단하고 보고.
2. 버전: `cmd/chaude/main.go:16` `var version = "0.7.3"` (ldflags `-X main.version` 빌드면 그쪽도 확인).
3. `CHANGELOG.md` 맨 위 `## 0.7.3`: `모델별 세션 유지(한 번에 하나씩 작업), 유휴 3599초 핸드오프 후 종료, 캐시 TTL 관측(5m 감지 시 290초로 단축)` + U0a의 두 항목(size tag 기본 large, 5h 사용량 상태줄). auto 승인 시 TTL 문구를 `캐시 TTL auto 감지`로. "multi-client"/"동시 실행" 표기 금지. 0.6.7~0.7.2 누락은 건드리지 않음.
4. 커밋 메시지 `0.7.3: per-model live child processes (serial dispatch), idle rotate at 3599s, cache TTL observe`; 커밋/태그(`git tag v0.7.3`)는 사용자 요청 시에만, push 안 함. **릴리즈 단계는 U0a 승인과 사용자 승인을 기다림.**

## 6. 가정 (검증법)
A1 [가정] stream-json이 `usage.cache_creation`(ephemeral_1h/5m)을 싣는다. 검증: U0b. 트랜스크립트(CLI 2.1.312)에는 있음[확인].
A2 [가정] 구독 경로에서 main은 auto일 때 1h, overage에서는 5m. 검증: ttlSeen 로그.
A3 [가정] live 자식을 `r.sess`로 되돌릴 때 tracker/transcript/agents/ctx/files 재바인딩만으로 정상. 검증: sonnet-analyst가 start()와 runTurn 읽고 TestRouteKeepsPreviousChildLive + 수동 3모델 왕복.
A4 [가정] 서브에이전트 호출은 main 캐시를 갱신하지 않음. 검증: 서브에이전트 직후 main cache_read.
A5 [가정] 3599초는 요청 지연 마진 없음. 검증: 핸드오프 시작~첫 토큰 지연; 필요 시 `--idle-rotate 3570s`.
A6 [가정] spinner.TickMsg가 !busy에서도 tick(tui.go:374). 검증: U8.
A7 [가정] 동시 live 3개의 메모리 비용 허용. 검증: `ps -o rss`.
A8 [가정] TTL 안 resume은 캐시 적중. 검증: resume 후 첫 턴 cache_read.
A9 [가정] 클라이언트 간 캐시 공유 없음. 0.7.3은 의존하지 않음.
A10 [가정] 레지스트리는 cwd당 chaude 한 프로세스만 씀. 완화: tmp+rename, 자기 행만 교체, OwnerPID 점유 검사. 락은 보류.
A11 [가정] chaude SIGKILL 시 stdin EOF로 자식이 종료. 검증: §2 kill -9 수동 시나리오.
A12 [가정] 라이브 테스트 $3 상한 집행(예산 플래그 지원 또는 usage/total_cost_usd 누적). 검증: §4.

## 7. 열린 질문 / 사용자 승인 필요
1. [통지] 0.7.3은 직렬 디스패치이며 "동시 실행"은 0.8. 구현은 진행.
2. [승인필요] U0a 별도 커밋(3단계)과 릴리즈 단계 진행.
3. [승인필요] 캐시 TTL 1h 고정(DECISIONS:23 "pin") -> auto(초과 사용 시 5m, 쓰기 2배 회피) 변경. 그 전까지 기본 1h, 5m 관측 시 290초 폴백.
4. 확정(해결됨): DesignPath는 Opus 자식이 작성, chaude는 Write 경로만 기록. 강제 종료는 PID kill 없이 레지스트리 정리.

## 8. 반영 기록
리뷰(plan-multiproc-impl-review.md) 결정 6 + 유닛 수정 12 + 사용자 테스트 규칙 1 = 19건 반영.
- D1 직렬 디스패치: §1 "동시 실행은 0.7.3에 없음" 표기, 0.8 노트 유지, CHANGELOG 문구 변경(§5).
- D2 DesignPath: §2 Opus 작성/chaude는 Write 경로만 기록/없으면 첨부 생략 + metrics만.
- D3 강제 종료: §2 PID kill 없음, reconcile(OwnerPID 죽음 -> parked/expired, harness-killed), 정상 종료 CloseAll 병렬.
- D4 미커밋 변경: U0a 별도 커밋(코드 -> 메트릭 -> notes), 사용자 승인 표시, 릴리즈 대기.
- D5 DECISIONS:23: 승인 전 기본 1h, 5m 관측 시 290초, U10a 승인필요.
- D6 같은 모델 haiku 워커 전환은 park-resume 유지, 다른 모델만 live 경로(§2).
- 유닛 7 liveChild 상태 확장/StopIdle 금지(§2). 8 비활성 세션 idleEvent(§2, U5). 9 비활성 자식은 핸드오프 없이 Close(§2). 10 `--idle-rotate`(U3b, 수동 검증). 11 idleDeadline 관측 TTL 우선(U1c). 12 LastAPI 통일(§1, U1b). 13 childKey 정규화(§2, 테스트). 14 레지스트리 저장/PID(U1a). 15 순서 변경 및 U10a/b 분리(§3). 16 추가 테스트 5개(§4). 17 에이전트 배정(§3 머리, 순수 유닛 H / 결합 유닛 S). 18 건드리지 말 것 추가(§3).
- 사용자 지시: 라이브 테스트 태스크당 $3 상한(§4, A12, 집행 방식은 가정).
- 열린 질문 1-5는 결정으로 정리(§7). 소스/DECISIONS.md 미수정.
