# chaude

`claude` CLI(stream-json)를 감싸는 터미널 드라이버입니다. 컨텍스트가 임계값을 넘으면 핸드오프 스냅샷을 만들어 세션을 자동으로 교체(회전)합니다. 개인용이며 배포 대상이 아닙니다. 현재 버전은 0.9.8이고, claude-lean(0.5.3)의 후속입니다. 변경 내역은 `CHANGELOG.md`를 보세요.

## 설치 (macOS Apple Silicon 전용)

릴리즈 바이너리는 darwin/arm64만 제공합니다. 비공개 저장소이므로 먼저 `gh auth login`이 필요합니다.

```sh
gh api repos/int-ttt/chaude/contents/install.sh -H "Accept: application/vnd.github.raw" | sh
# 공개 저장소라면: curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/main/install.sh | sh
chaude --version
```

- 최신 릴리즈를 받아 체크섬을 확인한 뒤 `/usr/local/bin`(쓰기 가능 시) 또는 `~/.local/bin`에 설치합니다. PATH에 없으면 `~/.zshrc`에 추가합니다.
- 환경변수: `CHAUDE_VERSION=v0.9.5`(특정 버전), `CHAUDE_INSTALL_DIR=경로`, `CHAUDE_REPO=소유자/저장소`.

소스에서 빌드하려면 Go 1.22+가 필요하고 `go build -o bin/chaude ./cmd/chaude`로 됩니다. 다른 OS는 `platform_setup/install.sh`(mac/linux) 또는 `platform_setup\install.ps1`(windows)이 Go 설치 확인, 빌드, OS별 `.claude/settings.json` 생성을 처리합니다.

## 사용

```sh
chaude                           # 인자 없이 실행하면 REPL 시작 (chaude repl과 동일)
chaude repl                      # REPL 시작 (기본 모델 opus, opus 임계값 200000 토큰, 그 외 모델 80000 토큰)
chaude repl --threshold 200000   # 임계값 변경 (opus 메인에는 적용 안 됨)
```

- 답변이 끝났을 때 컨텍스트 토큰이 `--threshold` 이상이면 핸드오프를 만들어 새 세션으로 넘어갑니다.
- 주요 옵션(`chaude repl`):

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--model` | opus | 모델 (`haiku`로 지정하면 Haiku 메인) |
| `--cwd` | 현재 디렉터리 | 작업 디렉터리 |
| `--threshold` | 80000 (opus는 200000 고정, 이 옵션 무시) | 이 토큰 수 이상이면 회전 |
| `--compact-window` | 400000 | 자식 프로세스 전용 자동 compact 창(0이면 끔, threshold보다 커야 함) |
| `--max-auto-rotations` | 무제한 | 자동 회전 횟수 상한 |
| `--tail-turns` | 4 | 스냅샷에 넣는 최근 대화 턴 수 |
| `--snapshot-chars` | 24000 | 스냅샷 최대 글자 수 |
| `--show-rotation` | 끔 | 회전 시 한 줄 표시 |
| `--metrics-dir` | `auto` (`record/metrics`) | 턴별 메트릭 폴더(빈 값이면 끔) |
| `--no-handoff-validate` | 끔 | 핸드오프 검증 끄기 |
| `--max-drain` | 3 | 동시에 마무리 중인 옛 세션 수(0이면 안 기다림) |
| `--no-delegation-rule` | 끔 | 시스템 프롬프트에 haiku 위임 규칙을 붙이지 않음 |

전체 옵션은 `chaude repl -h`로 확인합니다.

### 상주 세션과 코더 (0.9.4)

- 상주(resident) 자식 세션은 4개입니다: `main`(Opus 메인), `prompter`, `tui`(haiku-tui), `coder`. 명령을 처리 중(생각중)일 때만 실행 목록에 보이고, 유휴 상태는 숨깁니다.
- 입력 흐름: 입력 → prompter → main. prompter는 caveman 영문 요약을 `[prompter summary]`로 만들고, 원문은 요약이 의미를 잃을 때만 `[original]`로 덧붙입니다. prompter가 응답하지 않으면 원문만 전달합니다.
- 응답 흐름: reply → haiku-tui → pane. 응답은 haiku-tui가 존댓말로 렌더링해 화면 pane에 보입니다.
- 에이전트 규칙은 슬롯별로 적용됩니다(prompter, tui, coder 각각의 규칙 파일).
- 진행 표시: 단계별(stage) 진행 줄을 표시합니다.
- MCP 도구 `resident_coder`: 상주 코더에 코드·문서 작업을 맡기고 변경 요약만 받습니다.
- 코더는 `coder` 슬롯 하나를 씁니다. 같은 맥락의 후속 작업은 같은 코더에 보냅니다. 단순 조회와 소규모 수정은 Task haiku 에이전트가 맡습니다.
- 상태 줄의 P, T, C 컨텍스트 바는 상주 prompter, tui, coder의 사용량으로 계산합니다.
- `/view`: 세션 목록을 입력창 아래에 보여 줍니다. `/view <id-prefix>`로 해당 세션의 프롬프트를 엽니다(TUI 전용, esc로 한 단계 뒤로).

### 0.9.5 변경

- P/T 최소 세션: prompter와 tui는 도구, MCP, 스킬, 설정 파일을 모두 끈 최소 세션입니다(`--tools ""`, `--strict-mcp-config`, `--setting-sources ""`). 규칙은 각자 짧은 규칙 본문을 `--system-prompt`로 받습니다.
- 코더 도구 제한: coder는 `Read,Edit,Write,Bash,Grep,Glob`만 쓰고 MCP는 지정한 설정만 씁니다(`--strict-mcp-config`). 슬래시 명령은 끕니다.
- main MCP 엄격 모드: 메인 세션도 지정한 MCP 설정만 읽습니다.
- prompter 요약: `[prompter summary]`는 caveman 영문 요약, `[original]`는 원문입니다. 원문은 prompter가 `NEED_ORIGINAL`로 답하거나 인용·코드·경로·숫자·정정 문장처럼 요약이 의미를 잃을 수 있을 때만 붙습니다. 예전 `[prompter summary EN]`·`[original input]` 표기는 없어졌습니다.
- CLAUDE.md 분량을 절반으로 줄였습니다(약 8.8k → 4.4k 바이트). 매 턴 읽히는 규칙이라 줄인 만큼 맥락이 가벼워집니다.
- `opus-architect`는 Haiku 메인(`--model haiku`)이 넘긴 일만 받습니다. Opus 메인은 `haiku-*`에 직접 위임하고, `opus로` 강제 경로는 없습니다.
- 프로젝트 설정: `serena`와 `pyright-lsp` 플러그인을 끕니다(`.claude/settings.json`의 `enabledPlugins`).

### 0.9.6 변경

- `/resume`: 프로젝트 폴더별로 Claude Code 세션을 묶어 보여 주고, 검색어(query)로 걸러서 이어받습니다.
- `/version`: chaude 버전과 Claude Code 버전을 함께 보여 줍니다.
- 응답 대기 중에도 `/version`, `/usage`, `/help`, `/mode`는 바로 실행됩니다. 대화를 바꾸는 명령(`/resume`, `/clear`, `/rotate`, `/model`, `/cost`)은 거부되고 세션에 전달되지 않습니다.

### 0.9.7 변경

- `/settings`: 보존 기간, 추가 coder 수(270초 idle TTL, 기본 4, 0-8, `extra_coders`), 메인 모델을 프로젝트 폴더별 `<프로젝트>/.chaude/settings.json`에 저장합니다. 파일이 없으면 기본값(보존 30일, 추가 coder 4개, `opus`)을 씁니다. 다른 프로젝트 설정과 서로 영향을 주지 않으며, 바꾼 값은 다음 세션 시작 시 적용됩니다.
- 메인 모델: `main_model`은 `opus`, `sonnet`, `fable` 중 하나이고, claude CLI에는 `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-fable-5-1` 전체 이름으로 넘깁니다. 예산 단가 계산도 전체 이름을 인식합니다.
- `/resume`: 검색어 없이 입력하거나 결과가 여러 개면 기록 폴더 선택창을 엽니다(↑/↓, 휠로 이동, Enter로 불러오기, esc로 취소).
- `/view`: 이 chaude 실행에서 만든 Claude Code 세션만 보여 줍니다. 이전 실행의 세션도 종료 핸드오프로 이어받습니다.
- 종료 핸드오프: 종료할 때 이 실행의 세션 ID를 프로젝트 기록 폴더의 `.exit-handoff.json`에 저장하고, 같은 폴더에서 다시 시작하면 `/view` 목록에 반영합니다.

### 0.9.8 변경

- 프롬프트 기록: 입력한 프롬프트를 `<프로젝트>/.chaude/history.jsonl`에 저장하고 최대 500개까지 유지합니다. 다시 시작해도 남으며, 위 방향키로 이전 입력을 불러옵니다. 슬래시 완성 목록이 열려 있으면 위아래는 목록을 움직이고, 목록 맨 위에서 위 키를 누르면 이전 입력으로 넘어갑니다.
- 회전 보류: 백그라운드 에이전트가 실행 중이면 컨텍스트 회전을 미룹니다. 최대 10분(`--rotate-defer-timeout`)까지 기다리고, 0이면 바로 회전합니다.
- 에이전트 결과 보존: 백그라운드 에이전트가 끝나면 결과를 `notes/runs/<YYMMDD>-agent-<id>.md`에 덧붙여 저장합니다.

## 기존 Claude Code 세션을 chaude로 옮기기

평소 쓰던 `claude` 세션의 기록을 읽어 chaude 세션으로 이어받습니다.

1. **세션 ID 찾기.** 세션 기록은 `~/.claude/projects/<프로젝트 경로>/<세션ID>.jsonl`에 있습니다. 프로젝트 경로는 작업 디렉터리의 `/`, `.`, `_`를 `-`로 바꾼 이름입니다. 파일 이름(확장자 제외)이 세션 ID입니다.
   ```sh
   ls -t ~/.claude/projects/$(pwd | tr '/._' '---')/ | head
   ```
2. **먼저 미리보기.** claude를 띄우지 않고 크기만 확인합니다.
   ```sh
   chaude --from-claude <세션ID> --import-dry-run
   ```
   출력: `turns`(대화 턴 수), `files`(수정한 파일 수), `snapshot_chars`/`snapshot_bytes`(스냅샷 크기), `reco_found`/`reco_preserved`(마지막 `추천:` 발견/보존 여부). 추천이 보존되지 않으면 종료 코드 1입니다.
3. **이어받기.**
   ```sh
   chaude --from-claude <세션ID>
   chaude --from-claude latest            # 현재 디렉터리의 가장 최근 세션
   chaude --from-claude /경로/세션.jsonl   # 파일 직접 지정
   ```
   세션 ID와 `latest`는 **현재 디렉터리** 기준으로 찾으므로, 원래 세션을 쓰던 프로젝트 폴더에서 실행하세요(`--cwd`로도 지정 가능). 다른 `chaude repl` 옵션도 함께 쓸 수 있습니다.

옮겨지는 것
- 사용자와 어시스턴트의 대화 텍스트(최근 `--tail-turns`개 턴이 스냅샷에 들어갑니다).
- 세션 중 Write/Edit/NotebookEdit로 수정한 파일 목록.
- 마지막 어시스턴트 답변의 `추천:` 원문.
- 스냅샷에는 "이 세션은 핸드오프 노트 없이 가져온 것이므로 `notes/`(FACTS.md, DECISIONS.md)를 다시 읽을 것"이라는 안내가 붙습니다.

옮겨지지 않는 것
- 서브에이전트(sidechain) 대화, 메타 메시지, `<`로 시작하는 시스템성 메시지, 도구 실행 결과.
- 원래 세션의 컨텍스트 자체. 요약 스냅샷만 넘어가므로 정확한 값은 `notes/`에서 다시 읽게 됩니다.

## 메트릭

`chaude repl`은 턴마다 비용, 소요 시간, 회전 수, 위임 수 등을 `record/metrics`에 JSONL로 남깁니다.

```sh
chaude metrics              # 요약
chaude metrics --csv        # 턴별 행을 CSV로
chaude metrics --dir 폴더   # 다른 메트릭 폴더
```

## 명령 요약

| 명령 | 설명 |
|---|---|
| `chaude [repl] [옵션]` | REPL 시작 (명령 생략 시 기본) |
| `chaude --from-claude ID\|latest\|경로 [--import-dry-run]` | 일반 Claude Code 세션 이어받기 |
| REPL 안 `/resume`, `/import [ID\|latest]` | 현재 디렉터리의 Claude Code 세션을 최신순(날짜·턴 수·첫/마지막 프롬프트)으로 나열해 번호/↑↓로 골라 이어받기 |
| `chaude metrics [--dir D] [--csv]` | 메트릭 요약/CSV |
| `chaude gen-settings mac\|linux\|windows [--out 파일]` | OS별 `.claude/settings.json` 생성 |
| `chaude search-history`, `charcount`, `export-history`, `extract-transcript`, `compress-archive`, `usage-report`, `claude-history` | 보조 스크립트(`-h`로 옵션 확인) |
| `chaude --version` | 버전 출력 |

Claude Code 훅은 `.claude/hooks/*.py`(Python 3.8+)이며 `python3 .claude/hooks/<이름>.py`로 실행됩니다. 훅 JSON은 stdin으로 받습니다.

## 개발과 릴리즈

- 테스트: `go test ./...` (실제 claude를 쓰는 테스트: `HARNESS_LIVE=1 go test ./internal/harness -v`)
- 릴리즈: `v*` 태그를 push하면 GitHub Actions(`.github/workflows/release.yml`)가 `scripts/build-release.sh`로 darwin/arm64 아카이브와 `checksums.txt`를 빌드해 릴리즈를 만듭니다.
  ```sh
  git tag v0.9.5 && git push origin v0.9.5
  ```
