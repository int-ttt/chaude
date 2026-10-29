# chaude

Claude Code 사용량을 줄이는 하네스입니다. 메인 세션(Opus 또는 Fable)은 듣고·판단하고·설명만 하고, 파일·웹·명령 작업은 이름에 모델이 붙은 하위 에이전트(`haiku-*`, `sonnet-*`, `opus-architect`, `fable-architect`)가 합니다. 훅이 메인 세션의 작업 도구를 막기 때문에 규칙이 강제됩니다. 거기에 세션 회전·컨덕터 라우팅이 있는 터미널 REPL이 붙어 있습니다.

버전 `0.7.0` · 변경 내역 `CHANGELOG.md` · 상세 가이드 `docs/claude-lean/` (APPLY_GUIDE, FRESH_START)

## 필요한 것
- Claude Code 2.1.296 이상 (`claude --version`)
- Python 3.8 이상 (훅이 Python). **Windows는 `py` 런처가 함께 설치된 python.org 배포판**을 쓰세요. Microsoft Store 의 `python3` 별칭은 Python이 아니라 Store를 여는 껍데기라 훅이 돌지 않습니다.
- Windows: Git for Windows (Claude Code가 훅과 Bash 도구를 Git Bash로 실행)

## 설치
| 환경 | 명령 |
|---|---|
| Windows (PowerShell) | `irm https://raw.githubusercontent.com/int-ttt/chaude/main/install.ps1 \| iex` |
| macOS / Linux | `curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/main/install.sh \| sh` |
| 소스에서 | `go build -o bin/chaude.exe ./cmd/chaude` (mac/linux: `-o bin/chaude`) · Go 1.22 이상 |

`chaude` 바이너리 하나에 REPL, 프로젝트 설치기, 에이전트·훅·스킬·설정 파일이 모두 들어 있습니다.

## 프로젝트에 적용
```bash
chaude install <프로젝트 폴더>                 # 메인 Opus (기본)
chaude install <프로젝트 폴더> --main fable    # 메인 Fable
chaude install <프로젝트 폴더> --lean-off      # 일반 모드로 시작 (훅은 깔되 위임 규칙은 끔)
```
하는 일: 기존 `CLAUDE.md`·`.claude/settings.json` 백업 → `.claude/agents` 12개, `.claude/hooks` 10개, `.claude/skills` 7개 복사 → `settings.json` **병합**(사용자 훅·설정값 유지, Windows는 셸 없이 `py -3`로 실행하는 형태) → `CLAUDE.md` 맨 위에 정책 블록 추가(BOM·CRLF 유지, 재실행 시 블록만 교체) → `notes/`, `logs/` → `bin/chaude[.exe]` 복사 → `.gitignore`에 런타임 파일 추가.
같은 명령을 다시 실행하면 업그레이드입니다. 적용은 **새로 시작한 `claude` 세션**부터입니다.

## 사용
| 하고 싶은 것 | 입력 |
|---|---|
| 평소처럼 | `claude` 를 그냥 실행. 응답 끝 `추천:` 의 첫 항목은 `ㅇㅇ` 으로 실행 |
| REPL(세션 회전·컨덕터) | 프로젝트에서 `bin\chaude.exe` (mac: `bin/chaude`) · `--model fable` 로 메인 모델 지정 |
| 맡길 모델 지정 | 프롬프트에 `haiku로` / `sonnet으로` / `opus로` / `fable로` (한글 `하이쿠로`·`소넷으로`·`오퍼스로`·`페이블로`도 됨). REPL 컨덕터는 이 지시가 있으면 분류기 호출 없이 바로 그 모델 세션으로 보냅니다 |
| Fable | `fable로` → `fable-architect`. 가장 비싼 모델이라 사용자가 부르거나 Opus가 같은 일에 실패했을 때만 씁니다. 컨덕터도 `fable로` 가 있을 때만 Fable 세션으로 보냅니다 |
| 메인이 직접 작업 | `일반모드` (되돌리기 `절약모드`) |
| 진행상황 | `진행상황`, `끝났어?` → 모델 호출 없이 `logs/*.status` 표시 |

## Windows에서 다른 점
- 훅은 Claude Code가 Git Bash로 실행합니다. 저장소의 `settings.json`(공통 형태)은 `.claude/hooks/pyrun` 런처로 어느 OS에서나 동작하고, `chaude install`은 Windows에서 셸 없이 `py -3 <훅>`을 직접 실행하는 형태로 바꿔 씁니다 (`chaude gen-settings windows` 로 따로 뽑을 수도 있음).
- 상태 폴더는 `%LOCALAPPDATA%\chaude-harness` (mac `~/.claude-harness`, linux `~/.local/state/claude-harness`).
- 한글 입력은 Windows Terminal 에서 하세요 (옛 콘솔은 IME 입력을 삼킵니다).

## REPL 옵션
`chaude` 는 인자 없이 실행하면 REPL(`chaude repl`)입니다. 답변이 끝났을 때 컨텍스트 토큰이 `--threshold` 이상이면 핸드오프를 만들어 새 세션으로 넘어가고, 컨덕터(기본 켬)는 프롬프트마다 Sonnet 분류기가 haiku|sonnet|opus|fable 세션을 고릅니다 (`--conductor=false` 로 끔).
- 주요 옵션(`chaude repl`):

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--model` | haiku | 모델 |
| `--cwd` | 현재 디렉터리 | 작업 디렉터리 |
| `--threshold` | 270000 | 이 토큰 수 이상이면 회전 |
| `--compact-window` | 400000 | 자식 프로세스 전용 자동 compact 창(0이면 끔, threshold보다 커야 함) |
| `--max-auto-rotations` | 무제한 | 자동 회전 횟수 상한 |
| `--tail-turns` | 4 | 스냅샷에 넣는 최근 대화 턴 수 |
| `--snapshot-chars` | 24000 | 스냅샷 최대 글자 수 |
| `--show-rotation` | 끔 | 회전 시 한 줄 표시 |
| `--metrics-dir` | `notes/metrics` | 턴별 메트릭 폴더(빈 값이면 끔) |
| `--no-handoff-validate` | 끔 | 핸드오프 검증 끄기 |
| `--max-drain` | 3 | 동시에 마무리 중인 옛 세션 수(0이면 안 기다림) |
| `--no-delegation-rule` | 끔 | 시스템 프롬프트에 haiku 위임 규칙을 붙이지 않음 |

전체 옵션은 `chaude repl -h`로 확인합니다.

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

`chaude repl`은 턴마다 비용, 소요 시간, 회전 수, 위임 수 등을 `notes/metrics`에 JSONL로 남깁니다.

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
| `chaude install <프로젝트> [--main MODEL] [--lean-off]` | 프로젝트에 에이전트·훅·설정·정책 설치/업그레이드 |
| `chaude metrics [--dir D] [--csv]` | 메트릭 요약/CSV |
| `chaude gen-settings mac\|linux\|windows [--out 파일]` | OS별 `.claude/settings.json` 생성 |
| `chaude search-history`, `charcount`, `export-history`, `extract-transcript`, `compress-archive`, `usage-report`, `claude-history` | 보조 스크립트(`-h`로 옵션 확인) |
| `chaude --version` | 버전 출력 |

Claude Code 훅은 `.claude/hooks/*.py`(Python 3.8+, 표준 라이브러리만)이며 훅 JSON은 stdin으로 받습니다. 단독 점검: `py -3 .claude/hooks/<이름>.py` (mac: `python3 …`).

## 개발
```bash
go test ./...            # 단위 테스트 (claude 호출 없음)
CHAUDE_LIVE=1 go test ./internal/harness -run Conductor   # 실제 claude를 띄우는 테스트
scripts/build-release.sh v0.7.0   # dist/ 에 6개 OS·아키텍처 아카이브
```
릴리스: `cmd/chaude/main.go` 의 `version`, `.claude/claude-lean.version`, `CHANGELOG.md` 를 같은 버전으로 맞추고 `v<버전>` 태그를 올리면 GitHub Actions가 빌드·업로드합니다.
