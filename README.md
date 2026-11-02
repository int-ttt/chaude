A chat-style wrapper for the Claude Code CLI (unofficial)

# chaude

`chaude`는 `claude` CLI를 감싸는 터미널 프로그램입니다. 인자 없이 `chaude`를 실행하면 대화형 화면(REPL)이 열립니다. 컨텍스트가 임계값을 넘으면 핸드오프 노트를 만들어 새 세션으로 자동 교체합니다. 개인용 도구이며 배포 대상이 아닙니다. 현재 버전은 0.9.13입니다. 변경 내역은 `CHANGELOG.md`를 보세요.

사용법은 [사용자 가이드](docs/guide/index.md)를 보세요.

## 설치

릴리즈 바이너리는 darwin/linux(amd64·arm64), windows(amd64·arm64)용을 제공합니다. 공개 저장소이므로 토큰 없이 설치할 수 있습니다.

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
chaude --version
```

특정 버전을 설치하려면 `bash` 앞에 환경변수를 붙입니다:

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | CHAUDE_VERSION=v0.9.13 bash
```

- 릴리즈 아카이브를 받아 `checksums.txt`로 검증한 뒤 `~/.local/bin/chaude`에 설치합니다. 검증할 체크섬이 없으면 설치하지 않습니다.
- 이미 설치된 `chaude`가 있으면 `chaude.<이전버전>.bak`으로 백업한 뒤 덮어씁니다.
- PATH에 없으면 힌트를 출력하고, 로그인 셸에 따라 rc 파일에 추가합니다: zsh는 `~/.zshrc`, bash는 mac `~/.bash_profile`/linux `~/.bashrc`, 그 밖은 `~/.profile`.
- 필요 도구: `curl`, `tar`, `sha256sum` 또는 `shasum`.
- 환경변수: `CHAUDE_VERSION`(버전 지정), `CHAUDE_INSTALL_DIR`(기본 `~/.local/bin`), `CHAUDE_REPO`(`소유자/저장소`).

### Windows

```powershell
irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
chaude --version
```

- `%USERPROFILE%\.local\bin\chaude.exe`에 설치하고 체크섬을 검증합니다. 환경변수(`CHAUDE_VERSION`, `CHAUDE_INSTALL_DIR`, `CHAUDE_REPO`)와 `.bak` 백업, PATH 힌트는 macOS / Linux와 같습니다.

### 토큰 (선택)

설치에는 토큰이 필요 없습니다. GitHub API 요청 한도(403)에 걸리면 `GH_TOKEN`을 설정하고 다시 실행하세요. 설치 스크립트는 토큰을 `GH_TOKEN`, `GITHUB_TOKEN`, `gh auth token`(gh가 있을 때) 순서로 찾습니다. 토큰이 거부되면 토큰 없이 다시 시도합니다.

## 처음 실행

프로젝트 폴더로 이동해서 실행합니다.

```sh
cd ~/my-project
chaude
```

화면 위쪽에 환영 상자가 보입니다. 모델, 작업 폴더(cwd), 회전 임계값이 여기 표시됩니다. 아래쪽 입력창에 바로 질문이나 작업을 적고 Enter를 누르면 됩니다.

- 기본 모델은 Opus입니다. Opus 메인이 판단하고, 조회나 반복 작업은 `haiku-*` 에이전트에 맡깁니다.
- 기본 권한 모드는 `auto`입니다. 모드는 `shift+tab`이나 `/mode`로 바꿀 수 있습니다.
- 터미널이 아니거나 `HARNESS_PLAIN` 환경변수가 있으면 전체 화면 대신 일반 텍스트 REPL로 동작합니다.

## REPL 사용법

### 입력하고 답 받기

- Enter: 입력을 보냅니다.
- `alt+enter` 또는 `ctrl+j`: 줄을 바꿉니다.
- 줄 끝에 `\`를 적고 Enter를 누르면 줄바꿈이 됩니다.
- `↑` / `↓`: 이전 입력을 불러옵니다. 입력 기록은 `<프로젝트>/.chaude/history.jsonl`에 저장되고 최대 500개까지 남습니다. 다시 시작해도 남습니다.
- `/`로 시작하면 명령 목록이 열립니다. `tab`으로 고르고, 목록이 열려 있으면 `↑` / `↓`로 목록을 움직입니다.

### 상태 줄과 사용량

화면 맨 아래 상태 줄에 컨텍스트 바, 회전 횟수(`rot N`), 권한 모드, 실행 중인 에이전트 수가 보입니다. 오른쪽에는 자주 쓰는 키 안내가 있습니다.

답변이 끝날 때마다 줄 하나가 붙습니다:

```
[ctx 12,345 | in N cr N cc N out N | $0.0123 | 5h N | sess abcdef12 rot 0]
```

- `ctx`: 현재 컨텍스트 토큰 수입니다. 회전 기준이 됩니다.
- `$`: 이 턴의 비용(USD)입니다.
- `5h`: 5시간 사용량 비율입니다. 자식 `claude`가 보내는 `rate_limit_info`의 `five_hour` 사용률을 씁니다. 값을 알 수 없으면 `?`입니다.
- `/usage`: 플랜 사용량과 한도를 전체 화면으로 보여 줍니다. 조회는 별도 `claude` 호출이라 진행 중인 답변에 영향을 주지 않습니다. `esc`로 닫고, 방향키와 `pgup` / `pgdn`으로 스크롤합니다.

### 권한 모드

- `shift+tab`: 모드를 순서대로 바꿉니다. 순서는 `default` → `acceptEdits` → `plan` → `auto` → `default`입니다.
- `/mode`: 현재 모드를 보여 줍니다. `/mode <이름>`으로 바꿉니다.
- 모드 이름: `default`, `acceptEdits`, `plan`, `auto`. 시작할 때 모드는 `auto`입니다.
- `auto`: 권한 확인 없이 모든 도구 실행을 허용합니다. `acceptEdits`는 파일 편집 도구(Edit, Write, MultiEdit, NotebookEdit)만 허용합니다. `plan`은 읽기 도구만 허용하고 나머지는 거부합니다.
- 도구 실행 허가를 묻는 창이 뜨면 `y`나 Enter로 허용하고, `n`이나 `esc`로 거부합니다.

### 답변 멈추기와 되돌리기

- 답변 중 `esc`: 답변을 멈춥니다. 답변 중에는 `ctrl+c`도 멈춤입니다.
- 입력창이 비어 있을 때 `esc`를 두 번 누르면 되돌리기 창이 열립니다. `↑` / `↓`로 이전 질문을 고르고 Enter를 누르면 그 시점으로 화면과 입력창이 돌아갑니다.
- 되돌리기는 화면과 입력창만 바꿉니다. 자식 세션의 컨텍스트는 그대로 남습니다.
- `ctrl+c`(대기 중): 종료합니다. `ctrl+d`도 종료합니다. `/exit`와 `/quit`도 같습니다.

### 스크롤과 복사

- 마우스 휠로 스크롤합니다. 기본값은 켜짐입니다.
- `pgup` / `pgdn`, `ctrl+b` / `ctrl+f`, `shift+↑` / `shift+↓`로 스크롤합니다. 입력창이 비어 있으면 `home` / `end`로 맨 위와 맨 아래로 갑니다.
- `/mouse`: 휠 스크롤을 끕니다. 꺼 두면 터미널에서 드래그로 글자를 고를 수 있습니다. 대부분의 터미널은 `shift`를 누른 채 드래그해야 고를 수 있습니다.
- 드래그로 고른 글자는 놓는 순간 자동으로 복사됩니다. 상태 줄에 `복사됨 (N자)`가 2초 보입니다.

### 슬래시 명령

답변 중에는 `/usage`, `/version`, `/help`, `/status`, `/mode`, `/mouse`, `/view`, `/settings`만 쓸 수 있습니다. 나머지는 답변이 끝난 뒤에 입력합니다.

| 명령 | 하는 일 |
|---|---|
| `/help` | 명령과 키 목록을 보여 줍니다 |
| `/status` | 모델, 컨텍스트 토큰, 세션을 보여 줍니다 |
| `/model [이름]` | 모델을 보거나 바꿉니다 |
| `/clear` | 새 세션을 시작합니다. 핸드오프 노트는 이어받습니다 |
| `/rotate` | 새 세션으로 넘깁니다(핸드오프) |
| `/settings` | 설정 화면을 엽니다. 자세한 내용은 아래 |
| `/resume [n\|슬러그]` | 기록 주제를 보거나 하나를 엽니다. `/resume claude [검색어]`는 Claude Code 세션을 찾습니다 |
| `/import [ID\|latest]` | Claude Code 세션을 가져옵니다 |
| `/view [ID 앞부분]` | Claude Code 세션 목록을 보거나 하나를 엽니다. 아래 |
| `/mode [이름]` | 권한 모드를 보거나 바꿉니다 |
| `/mouse` | 마우스 휠 스크롤을 켜거나 끕니다 |
| `/usage` | 플랜 사용량과 한도를 보여 줍니다 |
| `/cost`, `/context`, `/compact`, `/config`, `/mcp`, `/agents` | `claude`에 물어 그 결과를 보여 줍니다 |
| `/init`, `/review`, `/doctor` | 턴을 하나 실행합니다(각각 CLAUDE.md 제안, 변경 검토, 설치 점검) |
| `/version` | chaude와 Claude Code 버전을 보여 줍니다 |
| `/exit`, `/quit` | 종료합니다 |

### 에이전트와 세션 보기

- `ctrl+t`: 이 세션의 에이전트 목록을 엽니다. 에이전트가 없으면 `no agents yet`이 뜹니다. `esc`로 닫습니다.
- 백그라운드 에이전트의 결과는 `notes/runs/<YYMMDD>-agent-<id>.md`에 저장됩니다.
- `/view`: 이 chaude 실행에서 만든 Claude Code 세션을 목록으로 보여 줍니다. `↑` / `↓`로 고르고 Enter로 엽니다. `←` / `→`로 프롬프트를 넘기고, `esc`로 목록에 돌아갑니다. `/view <ID 앞부분>`으로 바로 열 수 있습니다.
- `/resume`: 검색어 없이 입력하거나 결과가 여러 개면 기록 선택창이 열립니다. `↑` / `↓`로 이동하고 `esc`로 취소합니다.
  - chaude 기록 선택창: Enter는 그 기록의 내용을 화면에 불러옵니다. 현재 세션은 바뀌지 않습니다.
  - Claude Code 세션 선택창(`/resume claude`): Enter는 고른 세션을 `/import`로 가져오며, 현재 세션이 그 세션으로 바뀝니다.

### 설정 바꾸기

`/settings`에서 다음 값을 바꿉니다.

- 기록 보존 기간(기본 30일)
- 추가 코더 수(0–8, 기본 4). 추가 코더는 270초 동안 쓰지 않으면 종료됩니다.
- 메인 모델: `opus`, `sonnet`, `fable` 중 하나

설정은 `<프로젝트>/.chaude/settings.json`에 저장됩니다. 프로젝트마다 따로 저장되고, `esc`를 누를 때 저장됩니다. 바꾼 값은 다음 세션을 시작할 때 적용됩니다.

## 팁과 FAQ

**Q. 대화가 길어지면 어떻게 되나요?**
A. 답변이 끝났을 때 컨텍스트가 임계값 이상이면 자동으로 새 세션으로 넘어갑니다. 핸드오프 노트가 이어지므로 대화는 계속됩니다. 상태 줄의 `rot N`이 넘어간 횟수입니다. 임계값은 메인이 Opus면 200,000 토큰으로 고정됩니다. 다른 모델은 기본 80,000 토큰이며, 시작 옵션 `--threshold`로 바꿉니다.

**Q. 백그라운드 에이전트가 돌고 있는데 회전하면요?**
A. 에이전트가 끝날 때까지 회전을 미룹니다. 최대 10분까지 기다리고, 그 뒤에는 회전합니다.

**Q. 답변 중에 다른 명령을 치고 싶어요.**
A. `/usage`, `/version`, `/help`, `/status`, `/mode`, `/mouse`, `/view`, `/settings`는 바로 쓸 수 있습니다. 대화를 바꾸는 명령은 거부되고 "답변이 끝난 뒤 다시 입력" 안내가 뜹니다.

**Q. 조회나 간단한 수정은 어떻게 처리되나요?**
A. Opus 메인이 `haiku-*` 에이전트에 맡깁니다. 상주 세션 네 개(`main`, `prompter`, `tui`, `coder`)가 함께 돕니다. 입력은 `prompter`가 짧게 요약해 메인에 전달하고, 답변은 `tui`가 존댓말로 다듬어 화면에 보여 줍니다. 상주 세션은 처리 중일 때만 실행 목록에 보입니다.

**Q. 비용은 얼마나 줄어드나요?**
A. 핑퐁 게임 비교에서 plain Opus 대비 비용이 -42.7%였습니다(2회 평균). 표본이 2회뿐이라 범위로만 보세요. 자세한 수치는 `docs/pingpong-bench.md`에 있습니다.

**Q. 이전 Claude Code 대화를 이어 쓸 수 있나요?**
A. 네. `/import [ID|latest]`나 `/resume claude`로 현재 폴더의 세션을 가져옵니다. 명령줄 방법은 아래 "고급: CLI"에 있습니다.

**Q. 기록과 메트릭은 어디에 있나요?**
A. 프롬프트 기록은 `<프로젝트>/.chaude/history.jsonl`입니다. 턴별 메트릭(비용, 시간, 회전 수, 위임 수)은 `record/metrics`에 JSONL로 남습니다.

## 고급: CLI

REPL 대신 명령줄에서 쓰는 방법입니다.

| 명령 | 설명 |
|---|---|
| `chaude [repl] [옵션]` | REPL 시작(명령을 생략하면 기본) |
| `chaude --from-claude ID\|latest\|경로 [--import-dry-run]` | Claude Code 세션을 이어받습니다. `--import-dry-run`은 크기만 확인하고 claude를 띄우지 않습니다 |
| `chaude metrics [--dir D] [--csv]` | 메트릭 요약 또는 CSV |
| `chaude clean [dir] [--yes]` | 프로젝트에서 claude-lean 하네스를 제거합니다. 기본은 dry-run이고 `--yes`를 주면 백업 후 제거합니다. `notes/`와 `record/`는 남깁니다 |
| `chaude gen-settings mac\|linux\|windows [--out 파일]` | OS별 `.claude/settings.json`을 만듭니다 |
| `chaude --version` | 버전을 출력합니다 |

주요 REPL 옵션(`chaude repl`): `--model`(기본 opus), `--cwd`(기본 현재 폴더), `--threshold`(기본 80000, opus는 200000 고정), `--show-rotation`(회전 시 한 줄 표시), `--metrics-dir`(기본 `record/metrics`). 전체 옵션은 `chaude repl -h`로 봅니다.

Claude Code 세션 옮기기 예시:

```sh
chaude --from-claude latest --import-dry-run   # 크기만 확인
chaude --from-claude latest                    # 현재 폴더의 가장 최근 세션을 이어받기
```

세션 ID와 `latest`는 현재 폴더 기준으로 찾습니다. 서브에이전트 대화와 도구 실행 결과는 옮겨지지 않고, 요약 스냅샷만 넘어갑니다.

개발자용: 소스 빌드는 `cd src/main/intt && go build -o ../../../bin/chaude ./cmd/chaude`입니다(Go 1.22+). 테스트는 `cd src/main/intt && go test ./...`입니다. 릴리즈는 `v*` 태그를 push하면 GitHub Actions가 만듭니다.
