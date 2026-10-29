# haiku direct routing (261029)

## 결론
- 분류기가 간단한 조회/검색과 단순 파일 내용 수정을 Opus를 거치지 않고 Haiku 세션으로 바로 보냅니다.
- 수정 요청은 "이미 수정 이력이 있는 Haiku 세션" 후보 중 맥락에 맞는 것을 분류기가 고르고, 없으면 새 Haiku 세션을 씁니다.
- 판단/전략/장단점/정정/설계 등은 기존대로 Opus입니다.

## 설계
- 분류기 출력: topic, size 에 더해 `kind: lookup|edit|other`, `worker: <후보 id>|new` (총 5줄). kind 없으면 other(기존 동작).
- pickTarget(cur, topic, size, kind): size=small 이고 kind=lookup|edit 이면 haiku (사용자가 sonnet 강제한 경우 제외). 현재가 haiku 인데 그 조건이 아니면 opus 로 복귀. 그 외는 기존 pickModel 그대로.
- 후보 풀(haikupool.go, workerPool): Haiku 메인 세션에서 Write/Edit/NotebookEdit tool_use 가 나올 때 (세션 id, 직전 task, 파일) 기록 = 수정 이력이 있는 세션만 후보. 최신순 최대 5개를 분류기 입력에 "id | task | edited" 로 전달. 프로세스 메모리 한정(재시작 시 비움).
- worker 검증: 후보 목록에 없는 id 는 버리고 새 세션. kind=edit 일 때만 사용 (lookup 은 항상 새 세션).
- 세션 전환(Repl.switchSession): 떠나는 세션 id 를 parked[model] 에 보관하고 핸드오프 없이 종료, haiku 는 Resume=worker id 로 재개(없으면 새 세션). haiku 에서 opus 로 돌아올 때 parked 가 있으면 그 세션을 재개(핸드오프 회전 대신).
- haiku 로 가는 task 에는 delegateHint 를 붙이지 않습니다.

## 변경 파일
- internal/harness/classifier.go: 프롬프트, pickTarget, routeResult/parseRouteFull, classify 시그니처(후보 입력, routeResult 반환), route(), switchSession
- internal/harness/haikupool.go (신규): workerPool
- internal/harness/repl.go: Repl 필드(pool, parked, resumeID, curTask), start() 의 Resume, 편집 tool_use 시 noteEdit, pool 초기화
- internal/harness/conductor_test.go: TestHaikuDirectRouting(13 케이스), TestWorkerPool, live 테스트 시그니처/기대값 수정

## 테스트
- `go vet ./...` 깨끗, `go test ./...` 통과 (chaude/internal/harness ok). gofmt 깨끗.
- 라우팅 결정 테스트: lookup->haiku, edit->haiku(새/후보 재사용/미지 id 폐기), 판단/전략->opus, haiku 세션에서 판단->opus 복귀, sonnet 강제 유지, kind 누락 시 기존 동작.
- TestClassifierLive(CHAUDE_LIVE=1) 는 이번에 실행하지 않았습니다 (기대값만 haiku 로 수정).

## 위험 / 미검증
- 실제 분류기가 kind/worker 를 지시대로 내는지 live 미검증. "version 값 알려줘" 류가 haiku 로 가는지 확인 필요.
- switchSession 의 실제 child 전환(Resume, parked 복귀)은 fake session 테스트 없이 구조만 확인. 실행 검증 필요.
- 이전 규칙 "메인은 haiku 불가" 가 풀립니다. Haiku 메인은 Opus 용 delegation 규칙(delegationRule)·훅 세트가 맞지 않을 수 있고, 서브에이전트 훅은 haiku-* 만 허용하는 구성과 충돌 여부 미확인.
- Opus 복귀 시 Haiku 가 한 일을 Opus 가 모릅니다(parked 재개는 핸드오프 없음). harness 의 r.tr.turns 에는 남지만 Opus 컨텍스트에는 없음 -> 후속 대화에서 누락 가능. 요약 노트 주입이 후속 과제.
- 후보 풀이 메모리 한정이라 재시작 후 이력 소실. Haiku 세션 id 는 첫 init 이벤트 이후에만 있어 그 전 편집은 기록 안 될 수 있음.
- 품질: Haiku 가 단순 수정에서 실수할 가능성(Opus 검토 없음).
