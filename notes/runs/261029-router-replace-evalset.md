# Router replace eval set (150 prompts)
date: 2026-10-29
frozen_prompt_sha256: 4876c396587b46f20cd59bc5a6ab1ded88df43d9460f0c144457c3a77df0410a
labels fixed before any model call; do not change

| id | tier | boundary | prompt |
| --- | --- | --- | --- |
| E1 | haiku | n | What package name is declared in clients_test.go? |
| E2 | haiku | n | grep으로 "TODO(hj)" 찾아서 개수만 알려줘 |
| E3 | opus | n | 오프라인 모드를 지원할지 말지, 지원한다면 어디까지 할지 방향을 정해줘 |
| E4 | opus | n | 멀티 터미널에서 같은 세션 ID 충돌이 나는데 원인을 모르겠어. 가설을 세우고 확인 방법까지 설계해줘 |
| E5 | sonnet | y | 로그 포맷을 JSON으로 바꾸는 것 검토해서 필요한 파일만 고쳐줘 |
| E6 | haiku | n | git stash list 결과가 몇 개인지 알려줘 |
| E7 | haiku | n | usage.go의 DefaultBudget 상수 값 찾아줘 |
| E8 | opus | n | Propose a strategy for handling rate-limit errors across providers and compare at least three options. |
| E9 | sonnet | n | repl.go 입력 루프에서 Ctrl-C 처리 흐름을 따라가 설명하고 문서 주석을 달아줘 |
| E10 | opus | n | Pick a strategy for versioning the notes schema: semver, date-based, or none. |
| E11 | sonnet | n | 현재 브랜치 변경분 커밋 메시지 정리하고 push까지 해줘 |
| E12 | sonnet | y | usage 집계 로직 간단히 정리하고 필요하면 파일 나눠줘 |
| E13 | sonnet | n | clients_test.go에서 중복된 setup 코드를 파일 내부 헬퍼 함수로 뽑아줘 |
| E14 | opus | n | 자식 프로세스 종료 신호 처리 방식을 새로 설계해줘, 기존 구현에는 제약이 없어 |
| E15 | haiku | n | tui.go 상단 주석에 "recieve" 오타가 있는지 찾아줘 |
| E16 | haiku | n | 파일 목록에서 session 들어간 파일 이름만 뽑아줘 |
| E17 | opus | n | 라우터 결정을 규칙 기반으로 둘지 모델 기반으로 둘지 방향성을 판단해줘 |
| E18 | opus | n | 멀티 프로세스 세션 공유 방식을 파일락과 소켓 중 어느 쪽으로 갈지 추천해줘 |
| E19 | opus | n | 세션 시작 시간이 점점 느려지는 이유를 모르겠어, 원인 후보를 좁히고 우선순위를 매겨줘 |
| E20 | sonnet | n | CHANGELOG.md의 0.7.x 항목들을 사용자 관점으로 요약해줘 |
| E21 | opus | n | Recommend an approach for deduplicating prompts in the eval pipeline: hashing, embedding, or manual review. |
| E22 | sonnet | n | notes/runs 아래 261029 분석 결과 파일들 제목만 모아서 한 장 요약 만들어줘 |
| E23 | haiku | n | notes/metrics/2026-10-29.jsonl 줄 수 알려줘 |
| E24 | opus | n | 설정 파일 포맷을 TOML로 옮길지 YAML을 유지할지 결정 근거 정리해줘 |
| E25 | sonnet | y | 이 PR 머지해도 될지 리뷰하고 문제 있으면 고쳐줘 |
| E26 | haiku | n | CHANGELOG.md 최신 버전 헤더 한 줄만 복사해서 보여줘 |
| E27 | haiku | n | Update the copyright year in LICENSE from 2025 to 2026. |
| E28 | sonnet | n | Compare the install section of README between main and v0.7.2 and summarize the differences. |
| E29 | haiku | n | notes/research/billing.md 마지막으로 수정된 날짜 알려줘 |
| E30 | haiku | n | dispatch.go에서 func 로 시작하는 줄 개수 세줘 |
| E31 | sonnet | n | Write a short PR description for the cachettl changes, summarizing the diff against main. |
| E32 | opus | n | The conductor deadlocks under load after the last three fixes; figure out what is really going on. |
| E33 | opus | n | 장기적으로 notes 디렉토리를 DB로 옮겨야 할지 찬반을 따지고 권고해줘 |
| E34 | sonnet | y | dispatch 쪽 동시성 버그가 있는지 한번 봐줘 |
| E35 | haiku | n | docs/usage.md 에서 "budget" 단어가 나오는 줄 번호 뽑아줘 |
| E36 | sonnet | n | usage.go와 billing.md 내용을 대조해서 불일치 목록 만들어줘 |
| E37 | haiku | n | Run go vet ./... and tell me the exit code. |
| E38 | opus | n | 새 기능 작업 큐를 어떻게 구현할지 설계가 안 정해졌어, 옵션을 제시하고 추천해줘 |
| E39 | haiku | n | Run `go test ./internal/harness -run TestClients` and paste the summary line. |
| E40 | sonnet | n | Write an end-to-end install check script, run it once, and summarize the results. |
| E41 | haiku | n | notes/LEDGER.md 마지막 항목 날짜만 알려줘 |
| E42 | haiku | n | .gitignore에 bin/ 항목이 있는지 확인해줘 |
| E43 | opus | n | Which is better long-term for the usage tracker: event sourcing or snapshot tables? Recommend one. |
| E44 | haiku | n | children_test.go의 TestSpawn 함수가 몇 번째 줄에 있어? |
| E45 | haiku | n | Makefile의 test 타겟이 실행하는 명령어 그대로 보여줘 |
| E46 | opus | n | Stream output or buffered output for the TUI: which should be the default? Weigh the tradeoffs. |
| E47 | opus | n | 비용 상한 초과 시 동작을 중단과 경고 중 무엇으로 할지 정책을 정해줘 |
| E48 | sonnet | n | Summarize what changed in dispatch.go and dispatch_test.go since v0.7.2. |
| E49 | opus | n | The status line sometimes shows stale numbers and I cannot reproduce it; diagnose the likely cause. |
| E50 | haiku | n | Add a trailing newline to the end of docs/usage.md. |
| E51 | haiku | n | CHANGELOG.md 최상단에 "## 0.7.5" 헤더 한 줄만 추가해줘 |
| E52 | opus | n | 지금 구조에서 Haiku 워커와 Sonnet 오케스트레이터의 경계를 어디에 둘지 설계해줘 |
| E53 | haiku | n | repl.go 맨 위 import 블록에 fmt가 있는지 확인해줘 |
| E54 | sonnet | n | unit_test.go에서 sleep으로 기다리는 테스트 세 개를 channel 기반으로 바꾸고 이름은 그대로 둬줘 |
| E55 | sonnet | n | config.go에 max_parallel 옵션 추가해줘: 기본값 4, 로드와 검증, 테스트까지 같이 |
| E56 | sonnet | n | classifier_model_test.go의 커버리지 갭을 분석해서 상위 3개 케이스를 테스트로 추가해줘 |
| E57 | haiku | n | repl.go의 prompt 문자열을 "chaude> "로 바꿔줘 |
| E58 | haiku | n | classifier_model_test.go 맨 윗줄 주석 보여줘 |
| E59 | sonnet | n | usage.go의 parseRecord와 parseEntry 두 함수를 하나로 합치고 호출부 테스트까지 맞춰줘 |
| E60 | haiku | n | Show me the last 5 git commit subjects. |
| E61 | opus | n | 같은 빌드 실패가 세 번째 반복인데 고칠 때마다 다시 깨져. 근본 원인을 설계 차원에서 찾아줘 |
| E62 | haiku | n | Rename variable buf to buffer in clients.go line 42 only. |
| E63 | haiku | n | cmd/chaude/main.go에서 os.Exit 호출이 몇 군데 있는지 세줘 |
| E64 | sonnet | n | 이번 주 커밋들을 기능별로 묶어서 변경 요약 작성해줘 |
| E65 | opus | n | 릴리즈 주기를 2주로 할지 월 단위로 할지 전략 제안해줘 |
| E66 | sonnet | y | Clean up the error messages in clients.go so they read more consistently. |
| E67 | sonnet | y | classifier 임계값 문서 좀 정리해줘 |
| E68 | opus | n | 에이전트 간 신뢰 경계를 설계해줘 — 어떤 입력을 믿고 무엇을 검증할지 정해줘 |
| E69 | opus | n | Should we split the harness into two binaries or keep one binary with subcommands? Give a recommendation. |
| E70 | sonnet | n | Move the helper functions in children.go into a new file children_util.go in the same package, with no behavior change. |
| E71 | sonnet | y | Make the retry backoff configurable, keeping the current defaults. |
| E72 | opus | n | 이번 분기 로드맵에서 캐시 기능과 병렬 실행 중 무엇을 먼저 할지 우선순위 정해줘 |
| E73 | haiku | n | What is the value of MaxRetries in clients.go? |
| E74 | haiku | n | Run gofmt -l on the repo and tell me which files it lists. |
| E75 | opus | n | 에이전트 간 메시지 포맷을 JSON-RPC로 갈지 자체 텍스트 프로토콜로 갈지 설계 방향 잡아줘 |
| E76 | haiku | n | version.go의 Version 상수를 0.7.4로 바꿔줘 |
| E77 | sonnet | n | 0.7.4 릴리즈 준비로 CHANGELOG 항목을 추가하고 태그 전 체크리스트를 돌려줘 |
| E78 | haiku | n | DECISIONS.md 맨 아래 항목 번호가 뭔지 알려줘 |
| E79 | opus | n | 설정 우선순위(환경변수, 파일, 플래그) 규칙을 어떻게 정할지 설계 원칙부터 제시해줘 |
| E80 | opus | n | 로컬 실행과 원격 실행을 같은 인터페이스로 묶을지 근거와 함께 결정해줘 |
| E81 | sonnet | n | Update docs/usage.md and README.md to document the new --budget flag, keeping both in sync. |
| E82 | opus | n | 프롬프트 인젝션 방어를 어느 계층에 둘지 아키텍처 관점에서 제안해줘 |
| E83 | opus | n | 비용 집계를 요청 단위로 할지 세션 단위로 할지 장단점 비교하고 결론 내려줘 |
| E84 | haiku | n | 이 repo에 package.json이 있는지 확인해줘 |
| E85 | opus | n | Our multi-agent handoff loses context on long tasks; propose the architecture to fix it. |
| E86 | sonnet | y | 노트 파일들 정리 좀 해줘, 중복 제거 포함 |
| E87 | opus | n | Which fits our routing better, a per-token or a per-call cost model? Give a decision. |
| E88 | haiku | n | Remove the unused variable tmp on line 88 of core.go. |
| E89 | opus | n | Decide whether to keep the custom TUI or move to bubbletea, with a recommendation and risks. |
| E90 | opus | n | 세션 복구 기능을 새로 넣을 건데 저장 포맷부터 정해줘, 방향은 자유 |
| E91 | sonnet | y | 테스트 실행 시간이 길어진 것 같은데 느린 테스트 찾아서 정리해줘 |
| E92 | opus | n | 에러 복구 전략을 재시도 중심으로 갈지 체크포인트 중심으로 갈지 설계 판단해줘 |
| E93 | haiku | n | conductor_test.go 테스트 하나만 돌리고 PASS인지 FAIL인지만 말해줘 |
| E94 | sonnet | n | Refactor the session struct in core.go to functional options, updating callers in core.go and repl.go only. |
| E95 | opus | n | What should be the priority order for next quarter: observability, performance, or new providers? |
| E96 | haiku | n | go.mod에 적힌 Go 버전이 뭐야? |
| E97 | sonnet | y | Look into why the status line flickers and fix it if the fix is simple. |
| E98 | opus | n | Our budget guard fires twice on some runs; figure out the root cause, we have no idea yet. |
| E99 | sonnet | n | dispatch_test.go와 children_test.go의 테이블 테스트 구조를 비교해서 차이점 요약해줘 |
| E100 | sonnet | n | notes/LEDGER.md와 notes/DECISIONS.md의 2026-10 항목들을 날짜순으로 요약 정리해줘 |
| E101 | sonnet | n | Find duplicated assertions in unit_test.go and conductor_test.go and consolidate them into table tests. |
| E102 | opus | n | 장기 실행 세션의 메모리 정책을 어떻게 설계할지 방향을 제시해줘 |
| E103 | sonnet | n | Deploy the docs site after running the build, and report the URL. |
| E104 | sonnet | n | Rename the function loadConfig to readConfig across internal/config and cmd/chaude, and update all call sites. |
| E105 | opus | n | Which should be the source of truth for pricing: code constants or a fetched table? Decide. |
| E106 | haiku | n | cachettl_test.go의 import 목록만 뽑아줘 |
| E107 | haiku | n | tui.go에서 spinner라는 identifier가 몇 번 쓰였는지 알려줘 |
| E108 | sonnet | n | clients.go의 재시도 로직을 리뷰하고 빠진 에러 처리가 있으면 알려줘 |
| E109 | opus | n | 멀티 프로세스 락 경합이 원인인지 아닌지 불확실한데 진단 방법을 설계해줘 |
| E110 | opus | n | Architecture question: should the classifier run as a separate process or in-process? Justify. |
| E111 | sonnet | n | 세션 종료 시 로그 파일 flush 누락 여부를 코드 리뷰하고 수정 PR을 준비해줘 |
| E112 | haiku | n | .golangci.yml 파일이 있는지 확인해줘 |
| E113 | haiku | n | children.go 파일이 몇 줄인지 세줘 |
| E114 | haiku | n | internal/harness 안에 _test.go 파일이 몇 개인지 알려줘 |
| E115 | haiku | n | go build 한 번 돌리고 성공 여부만 알려줘 |
| E116 | haiku | n | In README.md, change the "Quick start" heading to "Getting started". |
| E117 | opus | n | Diagnose the intermittent panic in repl.go; nobody knows the trigger yet. |
| E118 | haiku | n | git 브랜치 목록 보여줘 |
| E119 | sonnet | n | tui.go와 repl.go의 status 줄 렌더링 함수를 status.go로 옮기는 리팩터링을 해줘, 동작은 그대로 |
| E120 | opus | n | Should logs be structured by default? Recommend one and explain the trade-off. |
| E121 | opus | n | 툴 호출 결과 캐싱 전략을 TTL, LRU, 캐싱 없음 중에서 고르는 기준을 세워줘 |
| E122 | opus | n | 다음 6개월 로드맵 초안을 우선순위와 근거 중심으로 짜줘 |
| E123 | sonnet | n | core.go와 repl.go에서 context 인자를 안 쓰는 함수 목록을 뽑고 제거 가능한지 판단해줘 |
| E124 | sonnet | n | git log 0.7.3..HEAD 기준으로 0.7.4 릴리즈 노트 초안 써줘 |
| E125 | haiku | n | ci.yml에서 Go 버전 값 찾아줘 |
| E126 | haiku | n | core.go에 FIXME 주석이 몇 개인지 세줘 |
| E127 | sonnet | n | Go 모듈 의존성을 업데이트하고 go.sum을 정리한 뒤 테스트를 전부 통과시켜줘 |
| E128 | sonnet | n | dispatch.go와 clients.go의 에러 래핑을 %w 규칙에 맞춰 통일해줘, 두 파일만 |
| E129 | haiku | n | notes/DECISIONS.md에서 D-12 항목 제목만 알려줘 |
| E130 | sonnet | n | feature/usage-status 브랜치를 main에 squash merge하고 결과 확인해줘 |
| E131 | haiku | n | Change the default log level constant in config.go from "info" to "debug". |
| E132 | sonnet | y | README 설치 섹션을 다시 써줘, 구조는 알아서 잡고 |
| E133 | sonnet | n | cachettl_test.go에서 커버리지가 낮은 분기를 찾아 테스트 케이스를 추가해줘 |
| E134 | haiku | n | unit_test.go 실행해서 실패 개수만 숫자로 알려줘 |
| E135 | opus | n | 같은 테스트 실패가 이번 주에만 네 번째야. 왜 계속 깨지는지 가설부터 다시 세워줘 |
| E136 | sonnet | n | Review the diff of internal/harness/tui.go from the last commit and list risky changes. |
| E137 | opus | n | 프로젝트에서 버그 수정과 신규 기능의 비율을 어떻게 가져갈지 전략을 제안해줘 |
| E138 | haiku | n | cmd/chaude 디렉토리에 파일이 몇 개 있는지 알려줘 |
| E139 | sonnet | n | routerprompt_test.go의 실패 메시지 형식을 다른 테스트들과 맞춰 정리해줘 |
| E140 | sonnet | n | metrics 2026-10-28과 2026-10-29 jsonl을 비교해서 호출 수 차이를 요약해줘 |
| E141 | haiku | n | unit_test.go에 t.Skip 쓴 곳이 있어? 있으면 줄 번호만 알려줘 |
| E142 | opus | n | 테스트가 CI에서만 두 번에 한 번 실패하는데 이유를 모르겠어, 원인부터 좁혀줘 |
| E143 | opus | n | 반복되는 릴리즈 실패 3건을 보고 프로세스의 근본 문제가 뭔지 진단해줘 |
| E144 | haiku | n | Print the installed Go toolchain version. |
| E145 | sonnet | n | children.go의 goroutine 종료 경로를 점검하고 누수 가능성이 있는 곳을 정리해줘 |
| E146 | opus | n | Diagnose why memory grows linearly with session count; we have not identified the cause yet. |
| E147 | sonnet | y | tui.go 스피너가 가끔 멈추는데 원인 같이 봐줘 |
| E148 | sonnet | n | ci.yml에 go test -race 단계를 추가하고 워크플로 문법까지 확인해줘 |
| E149 | opus | n | Design a plugin system for custom tools; the interface is completely open, so propose one. |
| E150 | opus | n | 팀 커밋 단위 기준을 제안해줘, 장단점 비교 포함 |
