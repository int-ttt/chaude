# 0.7.3 남은 작업 계획 (rev2): 활성 슬롯 재연결 (U5, U6b, U7, U8, U9) + 릴리즈
근거: 261029-plan-multiproc-impl.md(§3 U5-U8), 261029-haiku-coder-multiproc-integration.md, 261029-explore-harness-readout.md, 261029-plan-multiproc-rebind-review.md(Opus), 261029-research-budget-flag.md. 소스 수정 없음.
코드 사실(확인): 활성 자식 = `Repl.sess`(repl.go:139); 자식 묶음 필드 tracker(:151), lastCtx(:143), cat(:159); files(:142)는 대화 수준. switchSession은 classifier.go:204. `idleEvent(s,tk,ev)`(repl.go:217)는 세션별 클로저. childSet/pickChild 비테스트 호출처 없음. 레지스트리: clients.go reconcile/isWarm/idleDeadline. 예산 플래그: `--max-budget-usd`(print 전용, 2.1.312 확인), result.total_cost_usd는 프로세스 내 누적(단조 증가).

## 1. 활성 슬롯 재연결 설계 [결정]
원칙: `r.sess`는 REPL이 읽고 쓰는 유일한 슬롯(유지). 자식 상태는 `*liveChild`(children.go:33)에 있고 슬롯 바인딩은 `unbindActive`/`bindChild` 두 함수로만 바뀐다. 직렬 작업.
### 1.1 상태 분할 (G2 대체)
- 자식별(스왑): sess, tracker, Model(r.a.Model), lastCtx, cat, LastAPI, TurnFrom, pendingSnapshot, lastRotFail, ttlSeen, idle 누적기(공유 `r.idle` -> liveChild로 이동).
- 대화 수준(스왑 금지): tr, files, lastReco, lastReply, notices, draining, rotations, seenTools, pool, rate.
- 턴별(리셋만): delegs, curTask, resumeID.
### 1.2 unbindActive / bindChild
- `unbindActive() *liveChild`: 자식별 필드를 복사, `c.parked.Store(true)`를 가장 먼저, `TurnFrom=len(r.tr.turns)`. `r.tracker`는 nil로 두지 않음(tui.go:312/329/378/983이 매 틱 읽음; bindChild/start가 덮어쓸 때까지 유지). `r.sess=nil` 구간 발생 -> tui.go:890(/status)에 nil 가드(localcmd.go:36 방식). Close 안 함.
- `bindChild(c)`: 필드 역복사, `r.a.Model=c.Model`, 마지막에 `c.parked.Store(false)`. pendingSnapshot은 c의 값을 복원. **컨텍스트 공백 보강**: `len(r.tr.turns) > c.TurnFrom`이면 `r.pendingSnapshot`에 결정적 요약(`## While this session was parked`, r.tr.turns[TurnFrom:] + 그 사이 추가된 files, 모델 토큰 0) 덧붙임; 상한 haiku 3,000자 / opus,sonnet 6,000자(DECISIONS 개요 상한). 새 턴 없으면 건드리지 않음.
- 활성 아님 판정 경쟁 방지: `*liveChild`를 start()에서 할당하고 idle 클로저가 캡처. idle 고루틴이 쓰는 필드(LastAPI, idle 누적기)는 liveChild의 자체 mutex 하에만 쓴다(touchMain은 childSet 조회 불필요). `s == r.sess` 비교 금지, `c.parked` 사용.
### 1.3 파킹 자식 이벤트 (idleEvent 분기)
- 파킹 경로: idleText/usageLine/r.idle 렌더 금지. main 응답은 자식별 버퍼링, result 시 c.Model/c.LastCtx로 메트릭 기록 + notice "[parked <model> background reply] <300자>" 큐잉(notices에 mutex 추가). tk.onSubEvent, rate_limit_event, touchMain만 처리.
- 파킹 자식의 can_use_tool: decideMode 후 "ask"이면 "denied: session parked (another model is active)" 거절. r.ui.ask 호출 금지. (G1 설계로 해결, runTurn 변경 없음.)
- API 호출 한 번도 없는 자식(LastAPI 0)은 Park 대신 Close (isWarm이 어차피 거부).
### 1.4 모델 전환 경로 switchChild (route 내, 레지스트리 nil이면 기존 코드 그대로)
route 점검을 `r.parked` 우선 처리와 classifier.go:175-179 haiku 분기보다 **앞**으로 옮긴다(그래야 live 경로 도달, "Close 0" 성립). 순서:
1. 목표 모델 == r.a.Model: `rr.Worker != ""`이면 기존 switchSession(같은 모델 park-resume), 아니면 무동작.
2. 목표 != r.a.Model: 이력 있고 웜 대상 없으면 **옛 자식이 활성인 동안** `makeSnapshot(reason)`으로 snap 확보 -> `old := unbindActive()` -> Park(이력 없거나 LastAPI 0이면 Close). Park한 자식은 `r.parked`에 쓰지 않음(이중 소유 방지).
3. 목표가 haiku이고 `rr.Worker != ""`: childSet에 haiku 키가 있으면 Take 후 (agents 실행 중이면 drain, 아니면 Close), 그 다음 worker resume(switchSession 본문에서 Close 제외).
4. 그 외 `pickChild`: actUseLive -> Take+bindChild(죽은 자식이면 actStartNew+snap로 폴백); actResume -> `r.resumeID=row.SessionID; r.start()`; actStartNew -> `r.parked[model]` 있으면 resume, 없으면 `r.start()` 후 `r.pendingSnapshot=snap`.
5. start 실패 롤백: 이전 자식 Take+bindChild, 레지스트리 행을 live로 복구, r.a.Model 복원, 오류 출력.
- 규칙(T5): childSet에 키가 있는데 !isWarm이면 Take -> (agents 실행 중 ? drain : Close) -> 그 뒤 레지스트리가 결정. **childSet/draining에 있는 SessionID는 절대 resume하지 않는다.**
- Close는 (a) 3599초 만료 sweep, (b) CloseAll, (c) Park의 같은 키 교체, (d) 위 T5/무호출 자식 정리에서만. 최대 live 자식 = (cwd, 모델)당 1 = 하네스당 3 (문서화).
### 1.5 상태표 (6상태 + draining/dead, 전이 9개)
| 상태 | 프로세스 | 슬롯 | 진입 | 이탈 |
|---|---|---|---|---|
| active | 살아있음 | r.sess | start / bindChild | T1, T4, 모델 전환 |
| parked | 살아있음, idle reader만 | 없음 | unbindActive+Park | Take+bind, T2, T3, T5, 만료 |
| draining | 살아있음, r.draining, childSet 아님 | 없음 | rotate 기존 경로 | 에이전트 종료 후 Close -> expired |
| dead | 종료됨, 항목은 남음 | 없음 | 프로세스 종료 | 정리 -> expired("child-exit") |
| expired | 없음, 행에 SessionID 보존 | 없음 | sweep/reconcile/종료 | pickChild: actStartNew(핸드오프) 또는 resume |
전이: T1 active->draining|expired (임계/카테고리/수동/idle rotate; MarkEnded(reason) 또는 draining 메모). T2 parked->draining (sweepIdle 시 tracker.running()>0이면 Close 대신 drain). T3 parked->dead (idle reader가 s.lines 닫힘 감지, c.dead; sweep/pickChild가 Take+drop, 행 expired "child-exit"; pickChild는 dead를 없는 것으로 취급). T4 active->dead (턴 중 크래시; Events 오류 시 MarkEnded("child-exit"), 다음 start()의 Upsert가 덮어씀). T5 parked(!warm)->closed (위 규칙). T6 하네스 강제 종료: reconcile이 `pidAlive(ChildPID)`를 먼저 확인, 살아있으면 State=expired reason "orphan-alive"(resume 금지); signal.Notify에 SIGHUP 추가 -> 정상 종료(markInterrupted, closeDrains, CloseAll, MarkEnded "harness-exit"). T7 `/clear`와 applyImport: CloseAll(파킹 자식) + 레지스트리 expired "clear"; `/model <name>`: rotate 유지, 단 목표 키의 파킹 자식을 먼저 Take+Close/drain. T8 start 실패 롤백(§1.4-5), actUseLive 바인딩 실패 -> actStartNew+결정적 스냅샷. T9 deadline 0(`--idle-rotate 0`): `deadline<=0`은 "만료 없음"으로 isWarm/Expired/idleDue 모두 처리.

## 2. 유닛 순서 — 설계 결합(U5a/U5b/U6b)은 sonnet-analyst(U6b가 리뷰 실패 시 opus), 나머지 구현·테스트는 haiku-coder 정확 스펙, 매 유닛 후 좁은 테스트
| 순서 | 유닛 | 파일 | 내용 | 테스트 |
|---|---|---|---|---|
| 1 | U5a | repl.go | start()에서 *liveChild 할당, idle 클로저 캡처, `parked atomic.Bool`, `touchMain`(c.mu 하 LastAPI), 필드 `children,reg,ttlSeen`, 파킹 분기(§1.3) | TestTouchMainOnly, TestIdleEventFromParkedChild, TestIdleEventParkedNoRender, TestIdleEventParkedPermDenied, `go test -race` |
| 2 | U5b | repl.go | unbindActive/bindChild(§1.1-1.2, 델타 요약) | TestBindUnbindRoundTrip(Close 0), TestBindAddsParkedDelta, TestBindNoDeltaKeepsEmpty, TestTrackerNeverNil |
| 3 | U5d | repl.go | rotate에서 `makeSnapshot(reason) (snap string, ok bool)` 추출(핸드오프 턴+검증+withReco, close/start 없음); rotate는 이를 호출, 동작 불변 | 기존 rotate/핸드오프 테스트 무수정 통과 |
| 4 | U6b | classifier.go | switchChild 5단계(§1.4), seam `var classifyFn = classify` | TestRouteKeepsPreviousChildLive, TestRouteToHaikuParksLive(opus->haiku Close 0), TestRouteBackUsesLiveNotParkedMap, TestRouteWorkerSwitchClosesParkedHaiku, TestNoResumeOfLiveSessionID, TestRouteDeadChildFallsBack, TestRouteFallsBackWithoutRegistry, TestRouteStartFailRollsBack |
| 5 | U7 | repl.go, clients.go | `idleDue(now)`; deadline<=0 규칙(T9) | TestIdleDue, TestIdleDueDisabled, TestExpiredZeroDeadline |
| 6 | U8 | tui.go, repl.go | `sweepIdle(now)`(!busy, 1초 제한): 파킹+에이전트 실행 중 -> drain(T2), dead 정리(T3), 활성 idleDue -> 핸드오프 후 Close; CloseAll을 **두 종료 경로** tui.go:1117 defer(`if r.sess != nil` 밖으로)와 repl.go:810 loop() defer에; SIGHUP | TestSweepIdleClosesParkedOnly, TestSweepDrainsBusyParked, TestQuitCallsCloseAll, TestPlainLoopCallsCloseAll |
| 7 | U8b | clients.go, repl.go | reconcile 고아 점검(T6), /clear·applyImport CloseAll(T7) | TestReconcileOrphanAlive, TestClearClosesParked |
| 8 | U9 | budget.go(신규), repl.go | 예산 원장(§3.2) | TestBudgetDeltaPerProcess, TestBudgetGateBeforeSend, TestBudgetInterruptMidTurn |
| - | U5c | repl.go | `recordDesignPath(ev)`(opus Write의 `*-plan-*.md`, `-review.md` 제외); U5a 직후 독립 | TestDesignPathRecorded, TestDesignPathMissingSkipped |
의존: U5a -> U5b -> U5d -> U6b -> U7 -> U8 -> U8b -> U9. 금지(불변): rotate 핸드오프/검증 의미, drain 경로, haikupool, cleanEnv, Session.Close 의미, Setpgid.
수용: `go build ./... && go vet ./internal/harness/ && go test ./... -count=1` PASS(+ U5a/U5b는 `-race`), 기존 conductor_test 회귀 0.

## 3. 릴리즈 0.7.3
### 3.1 절차
1. 선행: U0a 별도 커밋 [승인필요]. `git diff --stat`로 U0a 성격(classifier size tag, ratestatus)과 0.7.3 파일 분리 확인.
2. 버전: `cmd/chaude/main.go:16` "0.7.2" -> "0.7.3".
3. CHANGELOG.md `## 0.7.3`: 모델별 세션 유지(직렬), 유휴 3599초 핸드오프 후 종료, 캐시 TTL 관측, size tag 기본 large, 5h 사용량 상태줄, 예산 상한 옵션. "multi-client/동시 실행" 표기 금지. 0.6.7~0.7.2 공백은 이번에 채우지 않음(별도, 0.7.3 뒤 haiku-scout 초안 -> 사용자 확인).
4. 라이브 검증(태스크당 상한 $3, 태스크 간 합산 안 함):
   L1 `chaude --version` = 0.7.3 (무료).
   L2 haiku->sonnet->haiku->opus 왕복: 이전 자식 Close 0, 돌아온 자식 cache_read>0, `ps` 자식 수, 레지스트리 SessionID당 프로세스 2개 없음, 돌아온 자식이 parked-delta 요약을 받음.
   L3 `--idle-rotate 5s`: 5초 후 핸드오프+닫기, clients.json State=expired, 다음 프롬프트에 스냅샷 전달.
   L4 종료 후 `ps` 고아 0; `kill -9`와 SIGHUP 각각 -> 약 20초 뒤 `ps` 자식 수 확인(clients.json만 보지 않음); 재시작 시 reconcile이 harness-killed/orphan-alive 처리.
   L5 TTL 관측(ttlSeen 5m/1h, 서브에이전트 직후 main cache_read).
   L6 파킹 자식 백그라운드 이벤트가 활성 화면을 오염시키지 않음, notice만 표시.
   **L-항목은 U9 구현 + 0.01 USD 스모크 PASS 전까지 시작 금지.**
5. 리뷰: sonnet-reviewer 1회, 0.7.3 diff만(U0a 제외). 지적 반영 후 재테스트.
6. 커밋/태그는 사용자 요청 시에만, push 없음: 코드 커밋 `0.7.3: per-model live child processes (serial dispatch), idle rotate at 3599s, cache TTL observe` + Co-Authored-By 줄; 노트/메트릭 별도 커밋; 이후 `git tag v0.7.3`. L1-L6 전부 PASS + 리뷰 통과 + 사용자 승인 전 태그/배포 차단. 하나라도 FAIL이면 태그 금지, 보고.
### 3.2 $3 상한 집행 [결정] (C1 해소: 플래그 존재 확인됨)
- 1차 = 하네스 원장 `type budget struct{cap, spent float64; last map[*Session]float64}`; `charge(sess,total)`는 프로세스별 `total_cost_usd` 델타(`spent += cur - last[sess]`)를 합산, `chargeOneShot(cost)`는 classifier·/usage 일회성 호출 비용, `allow(est) bool`.
- 정지 지점: S1 사전 게이트(classify 전, 매 Send(사용자 턴, 핸드오프 턴) 전): `spent>=cap` 또는 `spent+est(next)>cap`이면 거절 "[budget] $X / $3 reached, task stopped", 자식 Close 안 함, metrics stage=budget. est = lastCtx x (웜이면 cache-read 가격, 아니면 cache-write: 1h 2x / 5m 1.25x) + 출력 4k x 출력 가격, 모델별 가격표. S2 턴 중: main assistant usage 이벤트마다 진행 추정 가산, `spent+running>=cap`이면 `sess.Interrupt()`(esc 경로); 초과는 API 호출 1회 이내. 사후 정지만으로는 불충분(270k 콜드 opus 1h 캐시 쓰기 한 번이 약 $3 가능).
- 2차(백스톱) S3: 상한 지정 시(`--task-budget-usd`, 기본 꺼짐) 자식 인자(opts.Extra)에 `--max-budget-usd <cap>`. 프로세스 단위라 단독으로는 부족(태스크가 자식 3-4개 + 일회성 호출에 걸침, --resume 시 리셋 가능성).
- 태스크 경계: 사용자 프롬프트마다 원장 리셋; 늦게 도착한 백그라운드 비용은 시작한 태스크에 프로세스 델타로 자동 귀속. 구독 로그인의 total_cost_usd는 명목 가격(대리 지표로 사용).
- **0.01 USD 스모크 테스트(필수, $3 실행 전)**: stream-json 자식 1개, `--max-budget-usd 0.01`, 2턴. 기록: result subtype(예상 error_max_budget_usd), 프로세스 종료 여부(종료면 T3로 dead 처리), --resume 시 카운터 리셋 여부(C3). 비용 < $0.05. 담당 haiku-runner.

## 4. 가정
G3 r.tr 자식 간 공유 무방. 검증: L2 후 transcript 턴 순서. G4 3599초에 요청 지연 마진 없음(A5). 검증: L3, 필요 시 3570s. C3 --resume 시 예산 카운터 리셋 여부: 스모크 테스트로 확인.

## 5. 차단 요소 (미해결)
- U0a 사용자 승인(커밋 분리) 미확인.
- 0.01 USD 스모크 테스트 미실행(subtype·프로세스 종료·resume 리셋 미확인) + U9 미구현 -> L-항목 시작 불가.
- 사용자 승인 전 repl.go/classifier.go/tui.go 유닛 착수 여부 재확인 필요.

## 반영 기록
적용(18):
1. [적용] 단일 활성 슬롯 r.sess + 자식별 상태 스왑 유지 (§1, §1.2)
2. [적용] pendingSnapshot/lastRotFail/idle(+ttlSeen)를 자식별로; files/lastReco(+tr 등)는 공유 (§1.1)
3. [적용] 복귀 자식에 파킹 중 턴 요약(상한 3,000/6,000자) (§1.2)
4. [적용] 빈 세션 rotate panic -> makeSnapshot 분리, U5d (§1.4-2, §2)
5. [적용] draining·dead 상태 + 전이 T1-T9 (§1.5)
6. [적용] 강제 종료 고아(T6) reconcile pidAlive + SIGHUP 정상 종료
7. [적용] 살아있는 SessionID 중복 resume 금지(T5) (§1.4)
8. [적용] deadline 0 = 만료 없음(T9), U7
9. [적용] /clear·applyImport·/model 파킹 자식 정리(T7), U8b
10. [적용] route 점검을 r.parked 우선·haiku 분기(classifier.go:175) 앞으로 이동 (§1.4)
11. [적용] $3 원장(프로세스별 total_cost_usd 델타) + 다음 호출 전 정지 + 턴 중 interrupt + --max-budget-usd 2차 (§3.2, U9)
12. [적용] 0.01 USD 스모크 테스트를 $3 실행 전 필수로 (§3.2)
13. [적용] 경쟁 방지: *liveChild 캡처, atomic parked, tracker nil 금지, /status nil 가드 (§1.2)
14. [적용] 파킹 자식 이벤트 렌더 금지·notice 큐·권한 거절 (§1.3)
15. [적용] LastAPI 0 자식은 Close (§1.3)
16. [적용] start 실패 롤백에 레지스트리·모델 복구, 죽은 자식 폴백 (T8)
17. [적용] CloseAll을 tui.go와 repl.go loop() 두 종료 경로에 (U8)
18. [적용] C1 해소(플래그 존재), 기존 C1/C2/C3 실험 대체
보류(4):
1. [보류] (cwd당 live 자식 3개의 RAM 약 200-400MB씩) 문서화만, 0.7.3에서 수용
2. [보류] 같은 cwd에 chaude 창 2개 -> 프로세스 6개 (OwnerPID는 세션 공유만 막음), 문서화만
3. [보류] CHANGELOG 0.6.7~0.7.2 공백, 0.7.3 이후 별도 처리
4. [보류] 가격표/est(next) 정확도 보정(구독 명목 가격 대리 지표), L 실행 데이터로 후속 조정
