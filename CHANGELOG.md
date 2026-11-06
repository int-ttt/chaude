# Changelog

## Unreleased
- Engine: on Windows, a running agent's pid is checked with `OpenProcess` and `GetExitCodeProcess`, so a live agent is no longer marked dead.
  - 엔진: Windows에서 실행 중인 에이전트의 pid를 `OpenProcess`와 `GetExitCodeProcess`로 확인하여, 살아 있는 에이전트를 죽은 것으로 표시하지 않습니다.
- Agents: a finished agent keeps its last text as its summary, and the restored agent view shows it as `(restored)` after a restart.
  - 에이전트: 끝난 에이전트는 마지막 텍스트를 요약으로 보관하며, 재시작 후 복원된 에이전트 화면에 `(restored)`로 표시합니다.

## 0.11.4 - 2026-11-06
- Hooks: new `bash-py-guard.py` (Bash matcher) denies file edits made by inline Python or node (`open(..., 'w')`, `write_text`, `os.replace`, `shutil.copy`, `fs.writeFile*`). Read-only Python and `python3 -m pytest` stay allowed. Fresh projects and existing `settings.json` get it too.
  - 훅: 새 `bash-py-guard.py`(Bash 매처)가 인라인 Python이나 node로 하는 파일 편집(`open(..., 'w')`, `write_text`, `os.replace`, `shutil.copy`, `fs.writeFile*`)을 거부합니다. 읽기 전용 Python과 `python3 -m pytest`는 그대로 허용합니다. 새 프로젝트와 기존 `settings.json` 모두 이 훅을 받습니다.
- Agents: `sonnet-drafter` is renamed to `sonnet-writer` (agent file, hooks, CLAUDE.md, docs). Released sections keep the old name.
  - 에이전트: `sonnet-drafter`의 이름을 `sonnet-writer`로 바꿨습니다(에이전트 파일, 훅, CLAUDE.md, 문서). 릴리스된 섹션은 이전 이름을 그대로 둡니다.
- TUI: dragging the mouse past the top or bottom row of the transcript keeps scrolling and extends the selection (1 row per 50 ms at the edge, 3 rows past the pane). It stops at the content top or tail, when the pointer moves back inside, and on release.

## 0.11.3 - 2026-11-06
- Release: the Go implementation under `deprecated/` is deleted, as planned (6th release after 0.10.0). Its CI jobs and release job are removed, and `platform_setup/install.sh` now builds the Rust `chaude`. The Go source stays in git history at tag `v0.11.2`.
  - 릴리스: 계획대로 `deprecated/` 아래의 Go 구현을 삭제했습니다(0.10.0 이후 6번째 릴리스). Go용 CI 작업과 릴리스 작업을 제거했고, `platform_setup/install.sh`는 이제 Rust `chaude`를 빌드합니다. Go 소스는 태그 `v0.11.2`의 git 이력에 남아 있습니다.
- TUI: a SendMessage row stays open until the resumed agent ends. A queued message opens the same row, and the row colours are fixed.
  - TUI: SendMessage 행은 재개된 에이전트가 끝날 때까지 열려 있습니다. 대기 중인 메시지도 같은 행을 열며, 행 색상을 고쳤습니다.
- TUI: `/view` scrolls correctly and shows the agent names.
  - TUI: `/view`가 올바르게 스크롤되고 에이전트 이름을 표시합니다.
- Engine: a reply from an idle agent (handback) is shown at once instead of waiting for a turn.
  - 엔진: 유휴 상태 에이전트의 응답(핸드백)을 턴을 기다리지 않고 바로 표시합니다.

## 0.11.2 - 2026-11-06
- Docs: README effect analysis is updated (long vs short project), with the ping-pong ratio, and the line-3 wording is revised (EN and KO).
  - 문서: README(EN/KO)의 사용 효과 분석을 갱신했습니다(긴 프로젝트 vs 짧은 프로젝트, 핑퐁 비율). 3번째 줄 문구도 다듬었습니다.
- Agents: `sonnet-drafter` is added for story, game and important writing. `haiku-drafter` stays the default.
  - 에이전트: 스토리·게임·중요 문서 작성용 `sonnet-drafter`를 추가했습니다. 기본값은 `haiku-drafter`입니다.
- TUI: wrapped agent rows keep the `│` bar on every line, including the last one.
  - TUI: 줄이 넘어간 에이전트 행은 마지막 줄을 포함해 모든 줄에 `│` 막대를 유지합니다.

## 0.11.1 - 2026-11-06
- TUI: agents resumed with SendMessage are shown back in the agent list, and running rows are yellow.
  - TUI: SendMessage로 재개된 에이전트를 에이전트 목록에 다시 표시하고, 실행 중인 행은 노란색으로 표시합니다.
- TUI: timed action rows, and queued input is no longer echoed. A `/view` crash is fixed.
  - TUI: 시간이 표시되는 작업 행을 추가하고, 대기 중인 입력은 다시 출력하지 않습니다. `/view` 충돌을 고쳤습니다.
- Docs: README install section and feature order reordered (EN and KO), with a usage effect note.
  - 문서: README(EN/KO) 설치 섹션과 기능 순서를 재배치하고 사용 효과 설명을 추가했습니다.

## 0.11.0 - 2026-11-05
- Release: the release archives are now built from the Rust workspace (`crates/cli`, binary `chaude`) for the same four targets: darwin/arm64, linux/amd64, linux/arm64, windows/amd64. Asset names (`chaude_<version>_<os>_<arch>.tar.gz`, windows `.zip`) and `checksums.txt` are unchanged, so `install.sh`, `install.ps1` and `chaude update` keep working.
  - 릴리스: 릴리스 아카이브를 같은 네 대상(darwin/arm64, linux/amd64, linux/arm64, windows/amd64)에 대해 이제 Rust 워크스페이스(`crates/cli`, 바이너리 `chaude`)에서 빌드합니다. 에셋 이름(`chaude_<version>_<os>_<arch>.tar.gz`, windows `.zip`)과 `checksums.txt`는 그대로이므로 `install.sh`, `install.ps1`, `chaude update`는 계속 동작합니다.
- Release: the Go build under `deprecated/` is no longer published. Its release job is disabled, not deleted.
  - 릴리스: `deprecated/` 아래의 Go 빌드는 더 이상 배포하지 않습니다. 릴리스 작업은 삭제하지 않고 비활성화했습니다.
- Parity: the Rust build does not yet cover every Go feature. The open phases are listed in `plan/rust-port.md`.
  - 기능 동등성: Rust 빌드는 아직 Go의 모든 기능을 다루지 않습니다. 남은 단계는 `plan/rust-port.md`에 나와 있습니다.

## 0.10.2 - 2026-11-04
- Project setup: when chaude opens a folder, it writes the agent definitions (`haiku-*.md`, `opus-architect.md`) into `<cwd>/.claude/agents`. Existing files are never overwritten.
- Project setup: if `.claude/settings.json` already exists, the main-thread guard is merged into it instead of being skipped. Existing keys and hooks are kept, and a second run changes nothing.
- Guard: from the main thread, `Agent`/`Task` calls are allowed only for `haiku-*`, `opus-architect` and `claude-code-guide`. The default general-purpose agent is blocked.

## 0.10.1 - 2026-11-04
- Resume: a background agent resumed with SendMessage (`resumedAgentId`) goes back to the active agent list. The launch `agentId` is kept and saved with the agent store, so restored agents can be matched after a restart.
- /view: session rows and the prompt header show the agent name (for example `coder`, `prompter`) instead of the session id. Sessions from an earlier run still show their transcript title.
- /view: the prompt text scrolls with up/down, pgup/pgdn, home/end and the mouse wheel. Left/right step through prompts, esc goes back to the list.

## 0.10.0 - 2026-11-03
- Repo layout: the Go implementation moved to `deprecated/src/main/intt/`. The Rust workspace is at the repo root (`crates/core`, `crates/cli`; `crates/gui` is excluded until it is built). The Rust prototype moved to `deprecated/prototype/chaude-rs/`. E2E tests moved to `tests/e2e/`, `job_monitor.py` to `scripts/`.
- Rust workspace: in progress (core, CLI and TUI skeleton). The shipped binary is still the Go build.
- Deprecation: `deprecated/` (Go) is kept through the 5 releases after 0.10.0 and deleted in the 6th release after 0.10.0. Notes are in `deprecated/README.md`.
- Release: `release-dispatch` and `scripts/bump-version.sh` now use the `deprecated/` paths and also bump the Cargo workspace version.

## 0.9.15
- 설정: `gen-settings`의 출력 폴더(`--out`)가 없으면 자동으로 만듭니다. `.claude` 폴더면 `hooks/`도 함께 만듭니다.
- 턴 대기: 응답을 기다리는 고정 300초 제한을 없앴습니다. 300초가 지나면 턴이 중단되어 뒤늦게 온 Agent 결과가 버려지던 문제를 고쳤습니다. 이제 열린 tool_use가 있으면 조용히 기다리고, 열린 tool_use가 없이 `--idle-notice`(기본 30분) 동안 조용하면 안내를 한 번 출력한 뒤 계속 기다립니다. 중단은 Esc, Ctrl+C, SIGINT로만 합니다.

## 0.9.14
- 설정: 세션 시작 시 `.claude/settings.json`이 없으면 가드 훅이 들어간 파일을 자동으로 만듭니다.
- 빌드 대상 축소: 릴리즈 빌드를 darwin/arm64, linux/amd64, linux/arm64, windows/amd64 4개로 줄였습니다. Intel Mac(darwin/amd64)은 더 이상 빌드하지 않아 설치와 `chaude update`가 오류로 안내합니다. windows/arm64 설치와 업데이트는 windows/amd64 빌드를 받습니다(x64 에뮬레이션).
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.14로 갱신

## 0.9.13
- 메인 Opus 기본 effort를 medium으로 바꿨습니다 (`설정: 메인 Opus 기본 effort를 medium으로 변경`).
- 효율: 핑퐁 벤치 effort 재측정(unset/low/medium, 2회 평균)을 추가했습니다. 비용은 unset 0.861 / low 0.689 / medium 0.786 USD(plain 1.379 대비 -37.5% / -50.0% / -43.0%)입니다. 이번 측정에서 low가 가장 쌌고 medium이 그 다음입니다. 표본이 2회라 순서를 확정할 수 없습니다. 상세는 `docs/pingpong-bench.md`입니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.13으로 갱신

## 0.9.12
- 상주 세션: 꺼졌거나 없는 prompter·tui·coder 슬롯을 요청할 때 한 번 다시 띄운 뒤 재시도합니다. 다시 띄우지 못하면 입력을 원문 그대로 메인에 보냅니다.
- 문서: README와 APPLY_GUIDE에서 `sonnet-reviewer` 참조를 지웠습니다. 이 항목의 CHANGELOG 기록은 그대로 둡니다.
- 설치: 비공개 저장소용 토큰 헤더 안내를 README에 추가했습니다 (`curl -fsSL -H "Authorization: Bearer $GH_TOKEN" .../install.sh | bash`). 설치 스크립트 토큰 순서는 `GH_TOKEN`, `GITHUB_TOKEN`, `gh auth token`입니다.
- 설치: `install.sh`와 `install.ps1`은 릴리즈 바이너리만 받습니다. 소스 빌드 절차는 README "개발자용" 절로 옮겼습니다.
- 소스: Go 코드 위치를 `src/main/intt`로 고쳐 적었습니다 (루트 `go.work`가 이 모듈을 가리킵니다).
- 설정: `.gitattributes`로 `*.sh`, `*.py` 줄바꿈을 LF로 고정합니다.
- 문서: 효율 수치는 비용(-42.7%)만 적고 토큰 절감으로 표현하지 않습니다. `docs/pingpong-bench.md` 요약에서 토큰 차이 줄을 뺐습니다. 원시 수치 표는 그대로입니다.
- 문서: 스킬 `correction`에서 없는 `sonnet-analyst` 참조를 지웠습니다.
- 에이전트: `sonnet-analyst` 참조를 모두 없앴습니다. 판단·병합은 메인 Opus가 정하고 파일 작업은 `haiku-batch-worker`가 맡습니다. 추출은 `haiku-extractor`가 합니다. `/distill` 병합 단계와 설명 단계, `FRESH_START`, `APPLY_GUIDE`, `README`를 고쳤습니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.12로 갱신

## 0.9.11
- `chaude clean [dir]`: 프로젝트에서 claude-lean 하네스를 제거합니다. 기본은 dry-run으로 제거 대상만 보여 주고 아무것도 바꾸지 않습니다.
- `--yes`: 바뀔 파일을 `.chaude-backup-<YYYYMMDD-HHMMSS>/`에 먼저 복사한 뒤 제거합니다.
- 제거 범위: `.claude/{agents,hooks,skills}` 설치 파일, CLAUDE.md의 claude-lean 정책 블록, settings.json의 패키지 훅 항목. `notes/`, `record/`, 매니페스트 밖 파일, settings.json의 사용자 키·사용자 훅은 건드리지 않습니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.11로 갱신

## 0.9.10
- auto 권한 모드 기본: 권한 모드를 지정하지 않으면 auto로 시작합니다.
- 드래그 선택: 출력 영역을 드래그해 선택하면 놓을 때 자동으로 클립보드에 복사합니다.
- 링크·경로 색상: 출력의 링크와 파일 경로를 구분되는 색으로 표시합니다.
- /usage 전체 화면: 사용량 보기가 전체 화면으로 열리고 스크롤할 수 있습니다.
- Windows 회전 기호: 회전 표시 기호의 색을 Windows 터미널에서 맞게 고쳤습니다.
- gen-settings: 기본 경로를 바로잡았습니다.
- charcount: BOM과 CRLF 줄바꿈을 같은 방식으로 처리합니다.
- claude-history·compress-archive 등: `-h` 도움말과 빈 출력 동작을 고쳤습니다.
- extract-transcript: 경로 구분자를 OS에 맞게 처리합니다.
- CI: macOS·Linux·Windows 3개 OS에서 vet·test·빌드·스모크를 돌립니다.
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.10으로 갱신

## 0.9.9
- Windows·Linux 지원: 릴리즈 빌드가 darwin/linux/windows × amd64·arm64 6개 플랫폼 아카이브와 checksums.txt를 만듭니다. `install.sh`가 Linux를 지원합니다.
- Windows 대응: resume 경로 인코딩, claude 실행 경로 해석, 상주 소켓(Windows 전용 구현 분리)
- 효율 테스트: 핑퐁 게임 plain 대비 harness 비용 -42.7%(2회 평균, 전체 토큰 +73%, Haiku 포함). 결과와 주의점은 README의 "효율 테스트" 섹션, 장기 테스트 계획은 `docs/long-session-bench-plan.md`(미실행)
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.9로 갱신

## 0.9.8
- 프롬프트 기록: 입력 기록을 프로젝트별 `<프로젝트>/.chaude/history.jsonl`에 저장(최대 500개), 다시 시작해도 유지. 위 방향키로 이전 입력을 불러옵니다(슬래시 완성 목록이 열려 있으면 위아래는 목록 이동, 목록 맨 위에서 위 키를 누르면 이전 입력으로 넘어감)
- 회전 보류: 백그라운드 에이전트가 실행 중이면 컨텍스트 회전을 보류(최대 10분, `--rotate-defer-timeout`, 0이면 바로 회전). 시간이 지나면 회전 후 `rotate_defer` 기록 남김
- 에이전트 결과 보존: 백그라운드 에이전트가 끝나면 결과를 `notes/runs/<YYMMDD>-agent-<id>.md`에 덧붙여 저장
- 상주 세션은 main, prompter, tui, coder 4개로 고정(0.9.7 이전과 동일, 설정으로 바꾸지 않음)
- `/settings`: `coder_slots`를 `extra_coders`(추가 coder 수, 0-8, 기본 4)로 변경. 추가 coder(coder2...)는 호출 시 생성되고 270초 유휴 후 종료. 예전 `coder_slots` 키도 읽음
- TUI: 입력창 타이핑은 출력 화면 스크롤 위치를 바꾸지 않음(프롬프트 전송이나 맨 아래에서의 출력 갱신만 따라감)
- haiku-coder: 최종 결과 요약을 `notes/runs/`에 먼저 기록한 뒤 반환
- 버전 표기: cmd/chaude/main.go, docs/claude-lean/README.md, README.md를 0.9.8로 갱신

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
