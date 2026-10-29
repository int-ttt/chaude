# chaude

`claude` CLI(stream-json)를 감싸는 터미널 드라이버입니다. 컨텍스트가 임계값을 넘으면 핸드오프 스냅샷을 만들어 세션을 자동으로 교체(회전)합니다. 개인용이며 배포 대상이 아닙니다. 현재 버전은 0.6.6이고, claude-lean(0.5.3)의 후속입니다. 변경 내역은 `CHANGELOG.md`를 보세요.

## 설치 (macOS Apple Silicon 전용)

릴리즈 바이너리는 darwin/arm64만 제공합니다. 비공개 저장소이므로 먼저 `gh auth login`이 필요합니다.

```sh
gh api repos/int-ttt/chaude/contents/install.sh -H "Accept: application/vnd.github.raw" | sh
# 공개 저장소라면: curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/main/install.sh | sh
chaude --version
```

- 최신 릴리즈를 받아 체크섬을 확인한 뒤 `/usr/local/bin`(쓰기 가능 시) 또는 `~/.local/bin`에 설치합니다. PATH에 없으면 `~/.zshrc`에 추가합니다.
- 환경변수: `CHAUDE_VERSION=v0.6.6`(특정 버전), `CHAUDE_INSTALL_DIR=경로`, `CHAUDE_REPO=소유자/저장소`.

소스에서 빌드하려면 Go 1.22+가 필요하고 `go build -o bin/chaude ./cmd/chaude`로 됩니다. 다른 OS는 `platform_setup/install.sh`(mac/linux) 또는 `platform_setup\install.ps1`(windows)이 Go 설치 확인, 빌드, OS별 `.claude/settings.json` 생성을 처리합니다.

## 사용

```sh
chaude                           # 인자 없이 실행하면 REPL 시작 (chaude repl과 동일)
chaude repl                      # REPL 시작 (기본 모델 haiku, haiku 임계값 80000 토큰, opus·sonnet 270000)
chaude repl --threshold 200000   # 임계값 변경
```

- 답변이 끝났을 때 컨텍스트 토큰이 `--threshold` 이상이면 핸드오프를 만들어 새 세션으로 넘어갑니다.
- 주요 옵션(`chaude repl`):

| 옵션 | 기본값 | 설명 |
|---|---|---|
| `--model` | haiku | 모델 |
| `--cwd` | 현재 디렉터리 | 작업 디렉터리 |
| `--threshold` | 80000 (haiku; opus·sonnet 270000) | 이 토큰 수 이상이면 회전 |
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
| `chaude metrics [--dir D] [--csv]` | 메트릭 요약/CSV |
| `chaude gen-settings mac\|linux\|windows [--out 파일]` | OS별 `.claude/settings.json` 생성 |
| `chaude search-history`, `charcount`, `export-history`, `extract-transcript`, `compress-archive`, `usage-report`, `claude-history` | 보조 스크립트(`-h`로 옵션 확인) |
| `chaude --version` | 버전 출력 |

Claude Code 훅은 `.claude/hooks/*.py`(Python 3.8+)이며 `python3 .claude/hooks/<이름>.py`로 실행됩니다. 훅 JSON은 stdin으로 받습니다.

## 개발과 릴리즈

- 테스트: `go test ./...` (실제 claude를 쓰는 테스트: `HARNESS_LIVE=1 go test ./internal/harness -v`)
- 릴리즈: `v*` 태그를 push하면 GitHub Actions(`.github/workflows/release.yml`)가 `scripts/build-release.sh`로 darwin/arm64 아카이브와 `checksums.txt`를 빌드해 릴리즈를 만듭니다.
  ```sh
  git tag v0.6.6 && git push origin v0.6.6
  ```
