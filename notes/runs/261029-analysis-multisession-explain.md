# 다중 클로드 세션 동작/현재 상태 설명

근거: haiku-scout 요약 2건(261029-explore-children-dispatch-clients.md, 261029-explore-livechild-idle-budget.md). 코드 직접 재확인은 하지 않았습니다(추정 표시 있음).

## 1. 두 종류의 "다중 세션"을 구분해야 합니다
- (A) 하니스 내부의 다중 child: chaude 한 프로세스가 모델별로 claude 자식 프로세스를 여러 개 들고 있는 구조. 코드로 확인됨.
- (B) ListAgents에 보이는 peer 세션(claude-harness-6f, claude-opus-lean-29, univ-de, 모든 요소의 부가설명): 별개의 Claude Code 인터랙티브 세션들. 이 저장소 Go 코드에는 ListAgents 정의/호출이 없습니다(확인됨). 즉 이 목록은 Claude Code 자체 기능이 보여주는 것으로 보이며(추정), 하니스가 만든 것이 아닙니다. 하니스와 연결되는 지점은 아래 3번의 clients.json뿐입니다.

## 2. (A) 하니스 내부 동작
- 키: (cwd, model) 쌍마다 live child 하나(children.go childKey; cwd는 절대경로+심볼릭링크 해석).
- childSet: 메모리 맵. Park(보관)/Take(꺼냄)/Expired(만료 후보)/CloseAll.
- 전환(switchchild.go switchChild): 현재 모델과 다른 모델이 필요하면 pickChild가 셋 중 하나를 고름.
  - actUseLive: 메모리에 따뜻한(warm) child가 있으면 재시작 없이 bindChild로 다시 연결.
  - actResume: 레지스트리에 parked 행과 SessionID가 있고 소유 프로세스가 없거나 죽었으며 warm이면 resume.
  - actStartNew: 그 외 새로 시작(스냅샷 전달). 실패 시 이전 child를 복구.
- 이전 child는 parkOld로 보관(lastAPI 없으면 닫음). parked 동안은 도구 사용 요청을 "session parked"로 거부하고, 백그라운드 응답은 "[parked <model> background reply]" 알림으로 처리하며 예산에 반영.
- 동시 child 상한: DefaultMaxWorkers=3 (flag > CHAUDE_MAX_WORKERS > 3). 초과 시 에러, 기존 child는 건드리지 않음.
- 위임 에이전트가 돌고 있는 child는 retire 시 drain(MaxDrain>0일 때), idle sweep에서도 busy로 보고 유지.

## 3. 레지스트리(~/.chaude/clients.json)와 peer 가시성
- 행 키 (cwd, model), 상태 live|parked|expired, session_id, last_api, owner_pid, child_pid.
- 시작 시 reconcile: 소유 프로세스가 죽은 행을 정리(child가 살아있으면 expired "orphan-alive"로 표시만 하고 죽이지 않음, live는 warm이면 parked, 아니면 expired).
- 다른 chaude 프로세스가 같은 (cwd, model) 행을 소유 중(pid 생존)이면 resume하지 않고 새로 시작합니다.
- 코드에서 parked peer를 보여주는 목록 UI는 찾지 못했습니다(tui.go 렌더링은 미확인).

## 4. idle sweep / 예산 / 캐시 TTL
- idle: --idle-rotate 기본 3599초(0=끔, TUI에서만 동작), --cache-ttl 기본 1h. TTL이 1h면 deadline=3599초, 그 외에는 min(idleRotate, 290초).
- sweepIdle: 죽은 parked child는 닫고 "child-exit", 마지막 API 이후 deadline 경과한 child는 닫고 "expired"(바쁘지 않을 때만). 메인 세션은 TUI가 "/idle-rotate"를 보내 회전.
- 예산: --task-budget-usd 기본 0 = 꺼짐(0.8.0 커밋 메시지와 일치). >0일 때만 budget 생성, claude에 --max-budget-usd도 전달.
- 캐시 TTL 관찰: usage의 ephemeral_1h/5m 토큰으로 실제 TTL을 판정해 ttlSeen에 저장.

## 5. 현재 상태(실행 중 프로세스)
- 이번 조사에서는 실제 실행 중 프로세스/clients.json 내용을 열어보지 않았습니다. 기존 노트 261029-explore-running-processes.md가 있으니 현황이 필요하면 그것을 보세요(미확인).

## 6. 의심 지점(스카우트 보고, 미검증)
- clients.json 깨지면 다음 Upsert가 다른 행을 지움.
- 프로세스 간 파일 락 없음, tmp 파일명 공유(경쟁 가능).
- pickChild가 LastCtx를 락 없이 읽음.
- expiredZero 함수명이 의미와 반대.

## Task 1 참고
라우터 기본값을 haiku로 변경, 관련 테스트 3개 파일 갱신, go build/test 통과. gofmt는 classifier.go에서 변경 전에도 지적됨(기존 문제).
