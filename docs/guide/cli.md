# 명령줄 사용법

이 문서는 `chaude`의 모든 하위 명령, 옵션, 환경변수와 OS별 설치 방법을 설명합니다. 화면에서 쓰는 기능은 [repl.md](repl.md)를 보세요.

## 설치

릴리즈 바이너리는 darwin, linux, windows용이며 amd64와 arm64를 지원합니다. 저장소가 공개이므로 토큰 없이 설치할 수 있습니다. 토큰은 선택 사항이며, API 요청 한도(403)에 걸릴 때만 필요합니다.

설치 스크립트는 토큰을 이 순서로 찾습니다.

1. `GH_TOKEN` 환경변수
2. `GITHUB_TOKEN` 환경변수
3. `gh auth token` 명령의 결과(gh가 설치되어 있을 때)

토큰이 거부되면 토큰 없이 다시 시도합니다.

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
chaude --version
```

특정 버전을 설치하려면 `CHAUDE_VERSION`을 붙입니다.

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | CHAUDE_VERSION=v0.9.13 bash
```

설치 스크립트가 하는 일은 다음과 같습니다.

- OS와 CPU에 맞는 `chaude_<버전>_<os>_<arch>.tar.gz`를 받습니다. `x86_64`는 amd64로, `aarch64`는 arm64로 봅니다.
- `checksums.txt`로 아카이브를 검증합니다. 체크섬 파일을 받지 못하면 설치하지 않습니다.
- `CHAUDE_INSTALL_DIR`(기본 `~/.local/bin`)에 `chaude`를 둡니다. 이미 있으면 `chaude.<이전버전>.bak`으로 백업합니다.
- macOS에서는 다운로드 격리 속성(quarantine)을 지웁니다.
- 설치 폴더가 PATH에 없으면 `export PATH="<폴더>:$PATH"` 줄을 rc 파일에 추가합니다. 파일은 `SHELL` 값으로 정합니다. zsh는 `~/.zshrc`, bash는 macOS `~/.bash_profile`, linux `~/.bashrc`입니다. 그 밖의 셸은 `~/.profile`입니다. 추가된 줄 앞에는 `# chaude` 주석이 붙습니다.
- 필요한 도구는 `curl`, `tar`, `sha256sum` 또는 `shasum`입니다. 없으면 어떤 도구가 없는지 알려 주고 멈춥니다. `git`과 `go`는 필요 없습니다.
- `claude`가 PATH에 없으면 경고를 출력합니다. `chaude` 실행에는 Claude Code가 필요합니다.

### Windows

PowerShell에서 실행합니다.

```powershell
irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
chaude --version
```

- 기본 설치 폴더는 `%USERPROFILE%\.local\bin`이고, 실행 파일은 `chaude.exe`입니다.
- 아카이브는 `chaude_<버전>_windows_<arch>.zip`입니다. `checksums.txt`로 검증합니다.
- 기존 `chaude.exe`는 `chaude.<이전버전>.bak`으로 백업합니다.
- 설치 폴더가 PATH에 없으면 안내 메시지를 출력합니다. 스크립트가 직접 PATH를 바꾸지는 않습니다.
- 오류가 나도 PowerShell 창을 닫지 않도록 오류를 잡아서 메시지로 보여 줍니다.

### 설치 환경변수

| 이름 | 기본값 | 설명 |
|---|---|---|
| `CHAUDE_VERSION` | 최신 릴리즈 | 설치할 버전(예: `v0.9.13`) |
| `CHAUDE_INSTALL_DIR` | macOS/Linux `~/.local/bin`, Windows `%USERPROFILE%\.local\bin` | 설치 폴더 |
| `CHAUDE_REPO` | `int-ttt/chaude` | `소유자/저장소` 형식의 저장소 |
| `GH_TOKEN` | 없음 | 선택. API 요청 한도(403)를 피할 때 쓰는 토큰 |
| `GITHUB_TOKEN` | 없음 | `GH_TOKEN`이 없을 때 쓰는 토큰 |

### 업데이트와 제거

- 업데이트는 설치 명령을 다시 실행하면 됩니다. 기존 바이너리는 `.bak`으로 남습니다.
- 제거는 설치 폴더에서 `chaude`(Windows는 `chaude.exe`)를 지우면 됩니다. 백업 파일과 PATH 줄은 직접 지웁니다.

## 명령 한눈에 보기

```
chaude                          REPL 시작(기본)
chaude repl [옵션]              REPL 시작(옵션 지정)
chaude --from-claude ID|latest|경로 [--import-dry-run]
chaude metrics [--dir D] [--csv]
chaude gen-settings [windows|mac|linux] [--out FILE] [--src FILE] [--python CMD]
chaude clean [dir] [--yes]
chaude mcp-resident <소켓>      내부용 브리지
chaude <도우미 스크립트> [인자]  아래 "도우미 스크립트" 참고
chaude --version | -v | version
chaude -h | --help | help
```

- `-`로 시작하는 인자는 REPL 옵션으로 처리합니다. 예: `chaude --model sonnet`.
- 알 수 없는 명령을 치면 `unknown command`와 사용법을 출력하고 종료 코드 2로 끝납니다.

## repl

`chaude`와 `chaude repl`은 같은 REPL을 엽니다. 옵션은 아래 표와 같습니다. 전체 목록은 `chaude repl -h`로도 볼 수 있습니다.

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--model` | `opus` | 메인 모델 이름 |
| `--cwd` | 비어 있음 | 작업 폴더. 빈 값이면 chaude를 실행한 현재 폴더(`os.Getwd`)를 씁니다. 자식 `claude`도 이 폴더에서 뜨고, 기록 폴더와 `/resume claude` 검색도 같은 값을 씁니다 |
| `--threshold` | 80000 | 회전 기준 컨텍스트 토큰. 메인 모델이 opus이면 무시되고 200000을 씁니다 |
| `--compact-window` | 400000 | 자식 프로세스만 쓰는 `CLAUDE_CODE_AUTO_COMPACT_WINDOW`. 0이면 끕니다. 임계값보다 크게 두세요 |
| `--max-auto-rotations` | -1 | 자동 회전 최대 횟수. -1이면 제한 없음 |
| `--transcript` | 상태 폴더의 `transcript.jsonl` | 대화 기록 파일 |
| `--tail-turns` | 4 | 핸드오프에 넣을 최근 턴 수 |
| `--snapshot-chars` | 24000 | 핸드오프 스냅샷의 최대 글자 수 |
| `--auto-allow` | false | 모든 권한 확인을 허용합니다. 테스트용입니다 |
| `--show-rotation` | false | 회전할 때 한 줄 표시를 출력합니다 |
| `--metrics-dir` | `auto` | 턴별 메트릭 JSONL 폴더. `auto`는 `<record>/metrics`, 빈 값은 끔 |
| `--no-handoff-validate` | false | 핸드오프 검증을 끕니다(안전 장치 해제) |
| `--max-drain` | 3 | 동시에 정리(drain) 중일 수 있는 이전 세션 수. 0이면 정리하지 않습니다 |
| `--drain-timeout` | 60m | 정리 중인 에이전트를 기다리는 최대 시간 |
| `--rotate-defer-timeout` | 10m | 백그라운드 에이전트가 도는 동안 회전을 미루는 최대 시간. 0이면 바로 회전합니다 |
| `--rotate-cooldown` | 2m | 회전이 실패한 뒤 다시 시도하기 전 대기 시간 |
| `--no-category-rotate` | false | 카테고리 기반 세션 회전을 끕니다 |
| `--no-delegation-rule` | false | 자식 시스템 프롬프트에 haiku 위임 규칙을 넣지 않습니다 |
| `--agents-file` | `~/.chaude/agents.json` | 에이전트 상태 저장 파일. 재시작과 `/resume` 때 복원합니다. 빈 값은 끔 |
| `--from-claude` | 없음 | Claude Code 기록으로 시작합니다. 아래 "--from-claude" 참고 |
| `--import-dry-run` | false | `--from-claude`와 함께 씁니다. 핸드오프 크기와 추천 상태만 출력하고 `claude`는 띄우지 않습니다 |
| `--cache-ttl` | `1h` | 프롬프트 캐시 TTL. `auto`, `1h`, `5m` 중 하나입니다 |
| `--task-budget-usd` | 0 | 작업당 USD 상한. 0이면 끕니다. `claude`에도 `--max-budget-usd`로 넘깁니다 |
| `--idle-rotate` | 3599s | 자식이 이 시간만큼 놀면 차가운 것으로 보고 회전합니다. 0이면 끕니다. TUI에서만 동작합니다 |
| `--clients-file` | `~/.chaude/clients.json` | `(작업 폴더, 모델)`별 클라이언트 기록. 빈 값은 끔 |
| `--max-workers` | 3 | 동시에 살아 있을 수 있는 워커 세션 수 상한. `CHAUDE_MAX_WORKERS`가 설정되어 있고 옵션이 없으면 그 값을 씁니다 |
| `--orch` | `m0` | 오케스트레이션 모드. `m0`, `b`, `c`, `d`, `e`, `f` 중 하나입니다. 모드 `b`, `c`, `d`, `f`는 모델을 opus로, `e`는 haiku로 정합니다 |

- `--model`에 주는 이름은 `claude`에 그대로 전달됩니다. 설정 화면의 메인 모델은 `opus`, `sonnet`, `fable` 중 하나입니다.
- `--orch`는 A/B 테스트용 옵션입니다. 평소에는 `m0`을 씁니다.

### --from-claude

Claude Code 세션을 이어받아 chaude에서 시작합니다.

```sh
chaude --from-claude latest --import-dry-run   # 크기와 추천 상태만 확인
chaude --from-claude latest                    # 가장 최근 세션 이어받기
chaude --from-claude <세션ID>
chaude --from-claude /경로/세션.jsonl
```

- `ID`는 세션 ID, `latest`는 가장 최근 세션, `PATH`는 `.jsonl` 파일 경로입니다.
- 옮기는 범위: 화면(TUI)에는 메인 세션의 모든 내용을 보여 줍니다. 사용자와 어시스턴트 글, `thinking` 블록(저장된 경우), 도구 호출(`▸ 이름 입력`), 도구 결과(`← ...`, 20줄 또는 4000자에서 잘리고 잘린 양이 표시됩니다), 서브에이전트 대화를 포함합니다. 서브에이전트 대화는 그 대화를 시작한 `Agent` 호출 바로 아래에 `┆`로 들여 씁니다.
- 서브에이전트 연결은 `<세션>/subagents/*.meta.json`의 `toolUseId`로 합니다. 백그라운드로 돈 서브에이전트처럼 연결되지 않은 것은 화면 끝에 붙습니다.
- 핸드오프 스냅샷(다음 `claude`에 넘기는 요약)은 여전히 글만 씁니다. 도구 호출, 결과, 서브에이전트 내용은 스냅샷에 들어가지 않으므로 크기가 늘지 않습니다.
- `--import-dry-run`은 `turns`(스냅샷 입력 턴), `view_turns`(화면 턴), `subagents`(연결된 서브에이전트 수)를 출력합니다.
- 일반 모드(TUI가 아닌 실행)는 이 전체 화면을 그리지 않습니다.

## metrics

턴별 메트릭을 요약하거나 CSV로 출력합니다.

```sh
chaude metrics
chaude metrics --csv > turns.csv
chaude metrics --dir ./record/metrics
```

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--dir` | `auto` | 메트릭 폴더. `auto`는 `<record>/metrics`입니다 |
| `--csv` | false | 요약 대신 턴별 행을 CSV로 출력합니다 |

## gen-settings

OS별 `.claude/settings.json`을 만듭니다. 원본은 posix용 설정입니다.

```sh
chaude gen-settings mac --out .claude/settings.json
chaude gen-settings windows --out .claude/settings.windows.json
```

| 인자/옵션 | 기본값 | 설명 |
|---|---|---|
| `windows`, `mac`, `linux` | 실행 중인 OS | 대상 OS |
| `--out FILE` | 표준 출력 | 결과를 이 파일에 씁니다 |
| `--src FILE` | `CLAUDE_PROJECT_DIR/.claude/settings.json`, 없으면 `bin/` 상위 폴더의 `.claude` | 원본 설정 |
| `--python CMD` | `py -3`(PATH에 `py`가 있을 때), 아니면 `python` | windows 훅에 쓸 인터프리터 |

## clean

프로젝트에서 claude-lean 하네스를 제거합니다.

```sh
chaude clean            # 대상만 출력(dry-run)
chaude clean ~/my-project --yes
```

| 옵션 | 설명 |
|---|---|
| `[dir]` | 대상 프로젝트 폴더 |
| `--yes` | 실제로 지웁니다. 먼저 바뀔 파일을 `.chaude-backup-<YYYYMMDD-HHMMSS>/`에 복사합니다 |

- 지우는 대상은 CLAUDE.md의 claude-lean 정책 블록(마커 사이), `.claude/agents`, `.claude/hooks`, `.claude/skills`의 설치 파일, `.claude/settings.json`의 훅 항목입니다.
- `notes/`, `record/`, 목록에 없는 파일, 사용자 설정 키와 사용자 훅은 건드리지 않습니다.
- CLAUDE.md에 남는 내용이 없으면 파일을 지웁니다.

## mcp-resident

내부용 명령입니다. chaude가 상주 세션의 MCP 설정에 넣어 자동으로 실행합니다. 직접 쓸 일은 없습니다.

## 도우미 스크립트

`chaude <이름> -h`로 각 스크립트의 사용법을 볼 수 있습니다.

### charcount

텍스트 또는 마크다운 파일의 글자 수를 셉니다. 공백 포함과 제외, 바이트 수를 출력합니다.

```sh
chaude charcount notes/plan.md --section "배경" --limit 800
cat notes/plan.md | chaude charcount -
```

| 옵션 | 설명 |
|---|---|
| `FILE` | 대상 파일. `-`를 주면 표준 입력을 읽습니다 |
| `--section HEADING` | 그 제목 아래 섹션만 셉니다 |
| `--limit N` | N자 제한과 비교해 초과 여부를 출력합니다 |

### claude-history

프로젝트의 Claude Code 대화 기록 폴더 경로를 출력합니다. 기록은 `~/.claude/projects` 아래에 있습니다.

```sh
chaude claude-history [PROJECT_DIR]
```

- `PROJECT_DIR`을 생략하면 `CLAUDE_PROJECT_DIR`, 없으면 현재 폴더를 씁니다.
- 찾지 못하면 오류를 stderr에 출력하고 종료 코드 1로 끝납니다.

### compress-archive

대화 기록(`.jsonl`)을 `.jsonl.gz`로 압축하고 원본을 지웁니다.

```sh
chaude compress-archive DIR [DIR...]
```

- 숨김 폴더는 건너뜁니다. 끝나면 기록 개수와 용량 변화(MB)를 출력합니다.
- 인자 없이 실행하면 사용법을 출력합니다.

### export-history

프로젝트 대화 기록을 읽기 좋은 텍스트 파일로 내보냅니다.

```sh
chaude export-history --out ./history-export
```

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--project DIR` | `CLAUDE_PROJECT_DIR` 또는 현재 폴더 | 대상 프로젝트 |
| `--file F` | 없음 | 세션 `.jsonl` 파일 하나만 내보냅니다 |
| `--out DIR` | 없음 | 출력 폴더 |
| `--width N` | 120 | 줄 너비 |
| `--split-kb N` | 500 | 출력 파일 분할 크기(KB) |
| `--hook` | false | Claude Code 훅 JSON을 표준 입력에서 읽습니다 |

### extract-transcript

대화를 조각 파일로 나눕니다. 사용자와 Claude 메시지만 뽑고, 긴 붙여넣기는 해시로 중복을 제거합니다.

```sh
chaude extract-transcript <SOURCE> --out notes/distill
```

- `SOURCE`는 `.jsonl` 파일, 프로젝트 경로, 또는 `~/.claude/projects` 아래 폴더입니다.
- `DIR/chunks/NNN.md`와 `DIR/INDEX.md`를 만듭니다.

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--out DIR` | `notes/distill` | 출력 폴더 |
| `--chunk N` | 7000 | 조각 크기 목표(글자 수) |
| `--reply-chars N` | 800 | Claude 답변을 자를 글자 수 |

### search-history

프로젝트 대화 기록에서 검색어가 들어간 메시지를 최신순으로 찾습니다.

```sh
chaude search-history "배포 실패" --since 2026-10-01 --who user
```

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `"<terms>"` | 필수 | 검색어 |
| `--project DIR` | `CLAUDE_PROJECT_DIR` 또는 현재 폴더 | 대상 프로젝트 |
| `--since DATE` | 없음 | 이 날짜(YYYY-MM-DD) 이후만 |
| `--who` | `all` | `user`, `claude`, `all` 중 발화자 필터 |
| `--limit N` | 12 | 표시 건수 |
| `--width N` | 300 | 스니펫 너비 |

### usage-report

모델별 요청 수, 캐시 토큰, 출력 토큰과 추정 비용을 집계합니다.

```sh
chaude usage-report [PROJECTS_DIR] [--top N]
```

- `PROJECTS_DIR`을 생략하면 `~/.claude/projects`를 읽습니다.
- `--top N`은 비용이 많은 세션을 몇 개 보여 줄지 정합니다. 기본은 15입니다.

## 환경변수

| 이름 | 읽는 곳 | 설명 |
|---|---|---|
| `HARNESS_PLAIN` | REPL | 설정되어 있으면 전체 화면 대신 일반 텍스트 REPL로 동작합니다 |
| `HARNESS_RECORD_DIR` | 기록 위치 | 기록 폴더를 직접 지정합니다. 없으면 git 저장소 루트의 `record/`를 씁니다 |
| `HARNESS_USAGE_POLL` | `/usage`와 사용량 | `0`이면 5분마다 하는 자동 사용량 조회를 끕니다 |
| `CHAUDE_MAX_WORKERS` | `--max-workers` | 옵션을 주지 않았을 때 워커 수 상한의 기본값입니다 |
| `CHAUDE_IME_SHIFT` | 한글 입력 | IME 조합 글자의 커서 열 보정값(정수)입니다. 기본 0입니다. 글자가 한 칸 어긋나면 조정하세요 |
| `TERMINAL_EMULATOR` | 한글 입력 | `JetBrains-JediTerm`이면 보정값을 -1로 봅니다. `CHAUDE_IME_SHIFT`가 있으면 그 값이 우선입니다 |
| `CLAUDE_PROJECT_DIR` | 도우미 스크립트, `gen-settings` | 기본 프로젝트 폴더입니다 |
| `LOCALAPPDATA` | 상태 폴더 | Windows에서 `%LOCALAPPDATA%\claude-harness`를 상태 폴더로 씁니다 |
| `XDG_STATE_HOME` | 상태 폴더 | Linux에서 `$XDG_STATE_HOME/claude-harness`를 상태 폴더로 씁니다. 없으면 `~/.local/state/claude-harness` |
| `CHAUDE_ORCH` | chaude 내부 | `--orch` 값을 chaude가 시작할 때 설정합니다. 사용자가 직접 설정하는 값이 아닙니다 |
| `GH_TOKEN`, `GITHUB_TOKEN`, `CHAUDE_VERSION`, `CHAUDE_INSTALL_DIR`, `CHAUDE_REPO` | 설치 스크립트 | [설치](#설치)를 보세요 |
| `HARNESS_LIVE`, `HARNESS_LIVE_USAGE` | 테스트 | `1`이면 실제 `claude`를 쓰는 테스트를 켭니다. 일반 사용과는 무관합니다 |

- macOS의 상태 폴더는 `~/.claude-harness`입니다.
- 설정 파일은 환경변수가 아니라 `<프로젝트>/.chaude/settings.json`에 있습니다.
