# Changelog

## 0.9.7
- `/settings`: 설정을 프로젝트 폴더별 `<프로젝트>/.chaude/settings.json`에 저장(전역 `~/.chaude/settings.json` 폐지). 파일이 없으면 기본값 사용, 프로젝트마다 독립
- 메인 모델 전체 이름: `main_model`(opus/sonnet/fable)을 `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-fable-5-1`로 CLI에 전달, 예산 단가 계산도 전체 이름 인식(이전에는 0으로 계산)
- `/resume`: 검색어 없이 또는 결과가 여럿이면 기록 폴더 선택창(↑/↓·휠·Enter·esc)
- `/view`: 이 chaude 실행에서 만든 세션만 표시, 종료 시 `.exit-handoff.json`에 세션 ID 저장해 다음 시작 때 이어받음
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.7로 갱신

## 0.9.6
- `/resume` 프로젝트 폴더: 현재 폴더뿐 아니라 프로젝트 폴더별로 Claude Code 세션을 묶어 보여 주고, 검색어(query)로 걸러 이어받기
- `/version`: chaude 버전과 Claude Code 버전을 함께 표시
- 바쁠 때 안전한 슬래시 명령은 응답 대기 중에도 바로 실행(`internal/harness/busyslash_test.go`)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.6으로 갱신

## 0.9.5
- P/T 최소 세션: prompter, tui는 도구·MCP·스킬·설정 없이 짧은 자체 규칙으로만 실행(`Minimal`, `--system-prompt`)
- 코더 도구 제한: coder는 Read,Edit,Write,Bash,Grep,Glob만 사용, MCP는 엄격 모드(`--strict-mcp-config`), 슬래시 명령 끔
- main 세션 MCP 엄격 모드 적용
- prompter 요약 표기: `[prompter summary]`(caveman 영문), 원문은 필요할 때만 `[original]`로 덧붙임. 예전 `[prompter summary EN]`·`[original input]` 제거
- CLAUDE.md 분량 절반 축소(약 8.8k → 4.4k 바이트)
- opus-architect는 Haiku 메인 릴레이 대상만 담당, Opus 메인은 haiku-*에 직접 위임(`opus로` 강제 경로 제거)
- 프로젝트 설정: serena, pyright-lsp 플러그인 비활성화(.claude/settings.json)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.5로 갱신

## 0.9.4
- 상주 세션 구성 고정: main(Opus 메인), prompter, tui(haiku-tui), coder 4개 슬롯으로 고정
- 입력·응답 흐름 정리: 입력은 prompter를 거쳐 main으로, 응답은 haiku-tui를 거쳐 pane으로 전달
- 에이전트 규칙을 슬롯별로 적용
- 단계별(stage) 진행 줄 표시
- 제거: `--resident` 옵션, 코더 C1~C4 라우팅(coder-spare-1..4), `resident_coders_status` MCP 도구
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.4로 갱신

## 0.9.3
- 상주 세션 표시: 상주(resident) 자식은 명령 처리 중(생각중)일 때만 실행 목록에 표시, 유휴 상태는 숨김(internal/harness/resident.go)
- prompter 영문 요약 중계: 입력을 Opus 메인에 넘기기 전 상주 prompter가 영문 요약을 만들고 원문과 함께 전달(viaPrompter, 실패·시간 초과 시 원문 그대로)
- MCP 도구 추가: resident_coder(상주 코더에 작업 위임, 반환은 변경 요약만), resident_coders_status(코더 상태 조회)
- 코더 라우팅 C0~C4: 컨텍스트 판단으로 기본 코더(C0)와 spare 슬롯(C1~C4) 선택, 단순 조회·소규모 수정은 Task haiku 에이전트 유지(resident_coder.go)
- 상태 줄 P/T/C 컨텍스트 바: 상주 prompter(P), tui(T), coder(C0·Ci)의 사용량으로 계산
- /view 세션 보기: 세션 목록을 입력창 아래에 표시, /view <id-prefix>로 해당 세션 프롬프트 열람(sessview.go)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.3으로 갱신

## 0.9.2
- 입력 키: opt+delete(Alt+Backspace) 단어 삭제(DeleteWord, 한글 포함), cmd+delete(ctrl+u) 줄 앞까지 삭제(KillToStart)
- 테스트 추가: KillToStart, DeleteWord 구두점/한글, Alt+Backspace·Ctrl+U

## 0.9.1
- status 줄 변경: 역할별 컨텍스트 바(M, P, T, C0, 실행 중 코더 C1..C4) 표시(internal/harness/drain.go)
- TUI 에이전트 라벨 수정: 접두 이름(haiku-scout, opus-architect) 기준으로 라벨 표시
- /view 명령 추가(TUI 전용, Claude Code 세션 대화 보기: /view [id-prefix])
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.1로 갱신

## 0.9.0-r1
- record 기록 기능 추가(internal/harness/record*.go)
- 에이전트 반환 템플릿에 3.6 status·reason 머리 2줄 추가, 반환 규칙 정리(I9)
- 진행 보고 규칙 추가: status 줄 형식(step/pct), scout 상한 120s·10분, haiku-tui 존댓말 1줄 렌더
- caveman-format 훅 재작성: haiku-tui 존댓말·haiku-prompter 브리프 검사 추가, 테스트 tests/hooks로 통합
- haiku-prompter 브리프·반환 형식 보완, 훅 재작성 전 기준선 스냅샷 커밋
- deprecated/ 정리: 이전 노트·가져오기 자료를 deprecated/ 아래로 이동
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md를 0.9.0-r1로 갱신

## 0.9.0
- caveman 형식 정본 docs/claude-lean/CAVEMAN_FORMAT.md 신설 (S0)
- haiku-scout·haiku-researcher R-read 템플릿, 감시 훅 caveman-format.py(PostToolUse Agent|Task, 기본 log, 비차단) 등록 (S1)
- haiku-coder·batch-worker·runner·checker·drafter R-write 템플릿 (S2)
- opus-architect·sonnet-analyst·sonnet-reviewer B형식 브리프 + R-decide 템플릿 (S3)
- 감시 대상 12개 에이전트로 확대 (S4)
- CLAUDE.md:4 존댓말 범위를 사용자 노출 텍스트로 한정, O2 예외줄 제거 (S5)
- haiku-tui 신설(사용자 대면 출력), haiku-prompter 전달문 = 원문 + 요약 (U8, U9, U10)
- U3·U4·U7 구현 조사로 결정: matcher Agent|Task, mode f 기본 활성, lang: ko는 tool_input.prompt로 확인

## 0.8.2
- sonnet/opus(비-haiku) 서브에이전트의 파일·셸 직접 접근을 훅으로 전면 차단; 모든 읽기/쓰기는 haiku 에이전트 경유
- haiku-context-guard 매처 확장: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell
- sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출 가능 (agent-model-pin)
- sonnet-analyst/sonnet-reviewer/opus-architect tools: Agent 전용
- haiku 에이전트: `RETURN: verbatim <=N chars` 지정 시 원문 발췌 반환 허용 (max 6000)
- 훅 Pyright 경고 수정(reconfigure getattr, 모든 훅)
- 탈출구: 환경변수 CLAUDE_HARNESS_ALLOW_SONNET_IO=1 (기본 off)
- cmd/chaude version 0.8.2

## 0.8.1
- resume 수정: 작업 알림(`<task-notification>`)을 사용자 이벤트에서 처리해 에이전트 추적이 끝나도록 함(onUserEvent).
- 에이전트 패널(입력창 아래) 마우스 휠 스크롤.
- 최종 결과가 비어 있으면 result를 TUI 출력창에 표시(파일 경로만 안내하지 않음).
- sonnet-analyst 직접 파일 읽기/수정 차단(Read/Grep/Glob/Edit 도구 제거, haiku-scout/haiku-coder 위임).
- opus-architect 도구를 Agent 하나로 제한.
- 라우터(분류) 기본 모델을 sonnet에서 haiku로 변경. `CHAUDE_ROUTER_MODEL=sonnet`으로 되돌릴 수 있습니다.

## 0.8.0
- 모델별 세션 유지: 같은 작업 디렉터리에서 모델마다 살아 있는 세션을 따로 두고, 한 번에 하나씩 작업합니다. 같은 모델의 워커 전환은 기존 경로를 그대로 씁니다.
- 유휴 3599초 후 핸드오프 후 종료: 마지막 API 호출 후 3599초가 지난 세션은 핸드오프 후 닫습니다(`--idle-rotate`, TUI에서만 동작).
- 캐시 TTL 관측: 응답 usage에서 캐시 TTL을 감지하며, 5m 캐시가 감지되면 유휴 기준을 290초로 줄입니다(`--cache-ttl`).
- 분류 size tag 기본값을 large로 변경.
- 상태줄에 5h 사용량 표시.
- 설계 파일 경로 기록(U8b): opus 계획 파일 쓰기 시 DesignPath를 기록하고 핸드오프 스냅샷에 덧붙입니다.
- `/clear` 시 파킹된 자식 세션도 종료(U5c): `/clear`와 `/model`이 파킹된 자식 세션을 함께 닫습니다.
- 예산 기능(U9, 기본 꺼짐): `--task-budget-usd`(기본 0)로 켜며, 0이면 예산 로직을 쓰지 않습니다. 켜면 `--max-budget-usd`를 자식 프로세스에 전달합니다.
- 수정(BUG-1): 파킹되는 세션마다 자식 PID를 기록합니다(`ChildPID`).
- 수정(BUG-2): 재조정(reconcile)이 파킹된 세션까지 확인하며, 자식 PID가 살아 있으면 `orphan-alive`로 만료 처리하고 알림만 표시합니다.

## 0.6.6
- 분류 기반 세션 회전: 메인 세션이 답변 끝에 `[cat: X]`(같은 분류) 또는 `[cat-switch: X]`(새 분류) 표시를 붙이고, 전환이면 그 턴 뒤 핸드오프로 새 세션으로 넘어갑니다. 짧은 응답(ㅇㅇ 등)은 전환하지 않습니다. 끄기: `--no-category-rotate`. 메트릭 `stage: category`.

## 0.6.5
- REPL 기능: 에이전트 실행 상태 표시(상태줄), 이전 프롬프트로 시점 이동(Esc Esc), ↑/↓ 프롬프트 기록, 권한 모드(`/mode`, Shift+Tab: default/acceptEdits/plan/auto).
- 수정: 턴 사이에 백그라운드 서브에이전트가 보내는 권한 요청이 응답되지 않아 멈추던 문제(유휴 stdout 리더 추가). 백그라운드 작업의 늦은 `result`를 새 턴의 결과로 오인하지 않도록 방어.

## 0.6.4
- 인자 없이 `chaude`를 실행하면 REPL이 바로 시작합니다(`chaude repl`과 동일). `-`로 시작하는 REPL 옵션(예: `chaude --threshold 200000`)도 REPL로 전달됩니다. `-h/--help`는 사용법 출력.

## 0.6.3
- REPL에서 `/resume`(세션 목록 후 번호 또는 ↑↓+Enter로 선택)와 `/import [id|latest]` 추가. 현재 디렉터리의 Claude Code 세션을 날짜·턴 수·첫/마지막 프롬프트와 함께 최신순으로 보여주고 실행 중인 세션에 이어받습니다.

## 0.6.2
- claude-lean 병합(96765d3)으로 딸려 들어온 옛 파일 28개 제거. `.gitignore`가 `cmd/chaude`를 무시하던 문제 수정.
## 0.6.1
- `chaude --from-claude <id|latest|path>`: 일반 Claude Code 세션 기록을 읽어 새 chaude 세션으로 이어받음(마지막 `추천:` 원문 보존, 최근 대화 포함). `--import-dry-run`은 claude를 띄우지 않고 크기만 출력.
- 턴 로그에 비용·소요시간·회전 수·위임 수 추가, `chaude metrics [--csv]`로 요약/CSV 출력.

## 0.6.0
- claude-lean에서 이름을 `chaude`로 변경 (0.5.3 → 0.6.0). claude-lean의 연장선이며 저장소도 int-ttt/chaude로 변경.
- macOS Apple Silicon(darwin/arm64) 바이너리와 `install.sh`(gh 로그인 시 비공개 저장소도 가능) 추가.
- 이전 내역: `docs/claude-lean/CHANGELOG.md`.
