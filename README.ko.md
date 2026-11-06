# chaude

Claude Code를 채팅 형식의 프롬프트로 쓸 수 있고, 반복 작업은 Haiku 에이전트에게 맡기는 도구입니다.

[![Latest release](https://img.shields.io/github/v/release/int-ttt/chaude)](https://github.com/int-ttt/chaude/releases/latest)
[![Rust stable](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)](rust-toolchain.toml)
[![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](#설치)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[English](README.md) | **한국어**

## 개요

`chaude`는 `claude` CLI를 채팅 스타일 터미널 화면으로 감쌉니다. Opus가 메인 세션을 맡고, Haiku 에이전트가 조회, 작은 수정, 검증을 합니다. 문맥이 커지면 `chaude`가 인수인계 노트를 남기고 새 세션에서 대화를 이어 갑니다. 인자 없이 `chaude`를 실행하면 REPL이 열립니다.

## 기능

- 상태 줄이 있는 채팅형 REPL: 문맥 막대, 세션 교체 횟수, 권한 모드, 실행 중인 에이전트를 보여 줍니다.
- 인수인계 노트를 남기고 세션을 자동으로 바꿔서 긴 대화도 이어집니다.
- Opus 메인이 판단하고, Haiku 에이전트가 조회, 작은 수정, 검증을 맡습니다.
- 권한 모드 네 가지(`default`, `acceptEdits`, `plan`, `auto`)를 `shift+tab`으로 바꿉니다.
- 상태 줄에 비용, 5시간 사용량, 세션 교체 횟수를 보여 줍니다. `/usage`는 요금제 사용량과 한도를 보여 줍니다.
- 프로젝트별 입력 기록을 `.chaude/history.jsonl`에 저장합니다. `chaude search-history`로 찾을 수 있습니다.
- `/import`, `/resume claude`, `chaude --from-claude`로 Claude Code 세션을 가져옵니다.
- 세션을 시작하면 `.claude/settings.json`이 없을 때 가드 훅이 들어간 파일을 만듭니다 (0.9.14).

## 작동 방식

Opus가 메인 세션입니다. 요청을 읽고, 계획을 정하고, 최종 답변을 씁니다. 파일을 직접 읽거나 고치지는 않습니다. 이 제한은 훅이 강제합니다. 실제 작업(조회, 수정, 검증 등)은 Haiku 에이전트가 합니다.

| 에이전트 | 모델 | 용도 |
|---|---|---|
| `opus-architect` | Opus | 설계와 판단. 작업을 `haiku-*` 에이전트에게 나눠 줍니다 |
| `haiku-scout` | Haiku | 파일과 예전 대화 찾기 |
| `haiku-researcher` | Haiku | 출처를 붙인 웹 조사 |
| `haiku-extractor` | Haiku | 긴 대화에서 사실, 결정, 미결 사항 뽑기 |
| `haiku-drafter` | Haiku | 글 쓰기 또는 고치기 |
| `haiku-batch-worker` | Haiku | 형식 변환, 일괄 치환, 나누기와 합치기 |
| `haiku-coder` | Haiku | 범위가 좁은 코드 수정과 버그 수정 |
| `haiku-checker` | Haiku | 바뀐 내용을 읽기 전용으로 검증 |
| `haiku-runner` | Haiku | 오래 걸리는 작업을 백그라운드로 실행 |
| `haiku-prompter` | Haiku | 반복, 루프 작업 (메인 스레드에서만 호출) |
| `haiku-tui` | Haiku | 에이전트 결과를 한국어 존댓말 답변으로 바꿈 |

전체 목록, 도구, 배분 규칙은 [모델과 에이전트](docs/guide/models-and-agents.md)에 있습니다.

**문맥과 세션 교체.** 문맥이 기준값에 도달하면 `chaude`가 인수인계 노트와 함께 새 세션으로 넘어갑니다. 상태 줄의 `rot N`은 이 일이 몇 번 일어났는지 보여 줍니다. 기준값은 Opus에서 200,000 토큰입니다. 다른 모델은 기본 80,000이며 `--threshold`로 바꿀 수 있습니다. 세션을 넘길 때 백그라운드 에이전트가 아직 돌고 있으면 최대 10분(`--rotate-defer-timeout`) 기다린 뒤 넘어갑니다.

**Claude Code 세션.** `/import [ID|latest]` 또는 `/resume claude`를 쓰거나, `chaude --from-claude latest`를 실행하면 Claude Code 대화를 이어서 쓸 수 있습니다. 가져오기 화면에는 생각, 도구 호출과 결과, 하위 에이전트 메시지를 포함한 전체 대화가 나옵니다.

## 설치

미리 빌드된 바이너리를 제공합니다: macOS Apple Silicon(arm64), Linux(amd64, arm64), Windows(amd64; arm64 PC는 x64 에뮬레이션으로 실행합니다). Intel Mac은 지원하지 않습니다. 설치 스크립트는 설치 전에 `checksums.txt`를 확인합니다.

Claude Code CLI(`claude`)가 `PATH`에 있어야 합니다. 없으면 설치 스크립트가 경고합니다.

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | bash
chaude --version
```

지원 대상은 Apple Silicon(arm64) macOS와 Linux(amd64, arm64)입니다. Intel Mac(darwin/amd64)은 지원하지 않으며 설치 스크립트가 오류로 끝납니다.

`/usr/local/bin/chaude`에 설치합니다. 폴더가 없으면 만듭니다. `sudo`는 폴더에 쓸 권한이 없을 때만, 복사할 때만 씁니다. 기존 `chaude`는 `chaude.<이전 버전>.bak`로 남깁니다.

버전을 고정하려면 `CHAUDE_VERSION`을 지정합니다:

```sh
curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/master/install.sh | CHAUDE_VERSION=v0.11.3 bash
```

`curl`, `tar`, `awk`, `sed`, `install`, `mktemp`, 그리고 `sha256sum` 또는 `shasum`이 필요합니다.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/int-ttt/chaude/master/install.ps1 | iex
chaude --version
```

Windows on ARM64에서는 설치 스크립트가 amd64 빌드(x64 에뮬레이션)를 받고 안내를 출력합니다.

`%LOCALAPPDATA%\Programs\chaude\chaude.exe`에 설치합니다. 설치 스크립트가 이 폴더를 사용자 `PATH`에 한 번 추가합니다. 설치 후에는 새 터미널을 여세요.

토큰은 필요 없습니다. GitHub 요청 제한(HTTP 403)에 걸리면 `GH_TOKEN`을 지정하고 다시 설치하세요. 설치 스크립트는 `GITHUB_TOKEN`과 `gh auth token`도 읽습니다.

### 소스에서 빌드

Rust stable이 필요합니다. 버전은 `rust-toolchain.toml`로 고정되어 있습니다. 저장소 루트에서 실행하세요:

```sh
cargo build --release -p chaude-cli
./target/release/chaude --version
```

바이너리는 `target/release/chaude`에 생깁니다. `PATH`에 있는 폴더로 복사해서 쓰세요.

## 빠른 시작

```sh
cd ~/my-project
chaude
```

아래쪽 입력창에 질문이나 작업을 쓰고 `Enter`를 누르세요. 기본 모델은 Opus입니다. 권한 모드는 `shift+tab` 또는 `/mode`로 바꿉니다.

## 사용법

### REPL 키

| 키 | 동작 |
|---|---|
| `Enter` | 입력 보내기 |
| `alt+enter` 또는 `ctrl+j` | 줄 바꿈 (`\` 뒤에 `Enter`를 눌러도 줄이 바뀝니다) |
| `↑` / `↓` | 이전 또는 다음 입력. 최대 500개를 기억합니다 |
| `tab` | 슬래시 명령 완성. `/`를 입력하면 목록이 열립니다 |
| `shift+tab` | 다음 권한 모드 |
| `esc` 또는 `ctrl+c` (답변 중) | 답변 중단 |
| `esc` `esc` (입력이 비었을 때) | 되돌리기 선택창 열기 |
| `ctrl+c` (대기 중), `ctrl+d` | 종료 |
| `ctrl+t` | 에이전트 목록 보기 (`esc`로 닫습니다) |
| `pgup` / `pgdown`, `ctrl+b` / `ctrl+f`, `shift+↑` / `shift+↓` | 스크롤 |
| `home` / `end` | 맨 위 또는 맨 아래 (입력이 비었을 때) |
| `y` 또는 `Enter` / `n` 또는 `esc` | 도구 권한 요청 허용 또는 거부 |

마우스 휠 스크롤은 기본으로 켜져 있습니다. 터미널에서 글자를 드래그해 선택하려면 `/mouse`로 끄세요.

### 권한 모드

`shift+tab`은 모드를 `default` → `acceptEdits` → `plan` → `auto` → `default` 순서로 바꿉니다.

| 모드 | 허용 범위 |
|---|---|
| `default` | 도구를 실행하기 전에 매번 묻습니다 |
| `acceptEdits` | 파일 수정 도구(Edit, Write, MultiEdit, NotebookEdit)를 묻지 않고 실행합니다 |
| `plan` | 읽기 전용 도구만 실행합니다. 나머지는 거부합니다 |
| `auto` | 모든 도구를 묻지 않고 실행합니다 |

### 슬래시 명령

| 명령 | 설명 |
|---|---|
| `/help` | 명령과 키 보기 |
| `/status` | 모델, 문맥 토큰, 세션 보기 |
| `/model [이름]` | 모델 보기 또는 바꾸기 |
| `/mode [이름]` | 권한 모드 보기 또는 바꾸기 |
| `/clear` | 새 세션 시작 |
| `/rotate` | 지금 새 세션으로 넘기기 |
| `/settings` | 설정 화면 열기. `esc`로 나가면 저장되고, 다음 세션 시작 때 적용됩니다 |
| `/resume [번호 또는 슬러그]` | 기록 주제 목록을 보거나 하나를 엽니다. `/resume claude [검색어]`는 Claude Code 세션을 보여 줍니다 |
| `/import [ID 또는 latest]` | Claude Code 세션 가져오기 |
| `/view [ID 앞부분]` | Claude Code 세션 보기 (`esc`로 돌아갑니다) |
| `/usage` | 요금제 사용량과 한도 보기 |
| `/cost`, `/context`, `/compact`, `/config`, `/mcp`, `/agents` | `claude`에 물어보고 결과를 보여 줍니다 |
| `/init`, `/review`, `/doctor` | 한 턴을 실행합니다 (CLAUDE.md 제안, 변경 검토, 설치 점검) |
| `/mouse` | 마우스 휠 스크롤 켜기 또는 끄기 |
| `/version` | chaude와 Claude Code 버전 보기 |
| `/update` | 새 릴리스를 확인하고 바이너리를 교체합니다. 사용하려면 chaude를 다시 시작하세요 |
| `/exit`, `/quit` | 종료 |

답변이 진행 중일 때는 `/usage`, `/version`, `/help`, `/status`, `/mode`, `/mouse`, `/view`, `/settings`만 쓸 수 있습니다. 다른 명령은 답변이 끝난 뒤에 쓸 수 있습니다.

### 명령줄

| 명령 | 설명 |
|---|---|
| `chaude [repl] [플래그]` | REPL 시작 (명령이 없을 때 기본값) |
| `chaude --from-claude ID\|latest\|PATH [--import-dry-run]` | Claude Code 세션을 이어 씁니다. `--import-dry-run`은 크기만 확인합니다 |
| `chaude metrics [--dir D] [--csv]` | 턴별 지표를 요약하거나 CSV로 출력합니다 |
| `chaude search-history "<검색어>"` | 입력 기록 검색 |
| `chaude gen-settings mac\|linux\|windows [--out FILE]` | OS에 맞는 `.claude/settings.json`을 만듭니다 |
| `chaude clean [dir] [--yes]` | 프로젝트에서 하네스를 제거합니다. 기본은 dry-run입니다. `--yes`는 백업한 뒤 제거합니다 |
| `chaude update` | 새 릴리스를 확인하고 이 바이너리를 교체합니다 |
| `chaude --version` | 버전 출력 |

모든 REPL 플래그는 `chaude repl -h`로 확인하세요.

### 업데이트

- REPL에서 `/update`를 실행하세요. 최신 릴리스를 확인하고, 더 새 버전이 있으면 바이너리를 교체합니다. 사용하려면 chaude를 다시 시작하세요.
- 셸에서 `chaude update`를 실행해도 같은 확인을 합니다.

## 설정

REPL에서 바꿀 수 있는 설정은 `/settings` 화면에 있습니다. 값은 `esc`로 나갈 때 저장되고, 다음 세션을 시작할 때 적용됩니다.

프로젝트 설정은 `.claude/settings.json`에 둡니다. 세션을 시작할 때 파일이 없으면 가드 훅이 들어간 파일을 만듭니다. `chaude gen-settings`는 OS별 파일을 만듭니다.

REPL 주요 플래그(`chaude repl`):

| 플래그 | 기본값 | 뜻 |
|---|---|---|
| `--model` | `opus` | 메인 모델 |
| `--cwd` | `chaude repl -h` 참고 | 프로젝트 폴더 |
| `--threshold` | `80000` | Opus 이외 모델의 세션 교체 기준값. Opus는 `200000`으로 고정됩니다 |
| `--rotate-defer-timeout` | `10m` | 세션 교체가 필요할 때 백그라운드 에이전트를 기다리는 최대 시간. 그 뒤에는 기다리지 않고 넘어갑니다 |
| `--show-rotation` | `chaude repl -h` 참고 | 세션 교체 표시를 한 줄로 출력합니다 |
| `--metrics-dir` | `chaude repl -h` 참고 | 턴별 지표(JSONL)를 저장할 폴더 |

입력 기록은 프로젝트별로 `.chaude/history.jsonl`에 저장됩니다.

설치 스크립트가 읽는 환경 변수는 다음과 같습니다:

| 변수 | 뜻 |
|---|---|
| `CHAUDE_VERSION` | 설치할 버전(기본값: 최신), 예: `v0.11.2` |
| `CHAUDE_REPO` | 설치할 GitHub 저장소(기본값: `int-ttt/chaude`) |
| `GH_TOKEN`, `GITHUB_TOKEN` | 요청 제한이나 비공개 포크에 쓰는 선택 토큰. 둘 다 없으면 `gh auth token`을 씁니다 |

## 효과 분석

### 장기 프로젝트: 비용 절감 (Rust 포트 사용량)

Rust 포트 기간 동안 장기 프로젝트를 실제로 쓴 양입니다. 메인테이너 기기에서 2026-10-28부터 2026-11-05 14:00 UTC까지 측정했습니다. 메인 세션 229개, 하위 에이전트 실행 2,039회, API 요청 22,754건입니다. [scripts/rust_port_usage.py](scripts/rust_port_usage.py)가 Claude Code 기록을 읽고, 중복 요청을 뺀 뒤 요청마다 모델의 정가로 계산합니다.

| | 실제 (Opus, Sonnet, Haiku) | 추정 Opus만 | 절감 |
|---|---|---|---|
| 총 토큰 (입력, 캐시 쓰기, 캐시 읽기, 출력) | 771.8M | 446.0M | - |
| 대략적 비용 (USD) | 177.60 | 590.51 | 69.9% |

- 방법: Opus만 쓴 토큰 = 실제 토큰 / R. R = 1.73은 핑퐁 게임 비율입니다(Opus + Haiku 1,362,928 / Opus만 787,628 토큰, 2026-11-01). 비용 = 같은 토큰을 Opus 정가로 계산한 값(1,021.83) / R.
- 범위: 2026-11-02 effort 재측정에서 R은 1.37~1.62였습니다. 이때 절감률은 71.9%~76.2%입니다.
- Opus만 쓴 경우도 입력, 캐시, 출력 비율이 실제와 같다고 가정합니다. 핑퐁 표본이 작습니다(방식당 2회).

### 단기 프로젝트: 비용 절감 (핑퐁 벤치마크)

작은 빌드 한 번(핑퐁 게임, 프롬프트 3개)에서 chaude와 일반 Opus를 비교했습니다:

- **비용: -42.7%** (일반 Opus 대비, USD, 2회 평균, 2026-11-01).
- **비용 범위: -37.5% ~ -50.0%** (effort 설정별 unset, low, medium; 2026-11-02 재측정).
- **총 토큰: +73.0%** (Haiku 포함, 2026-11-01). 비용은 그래도 줄었습니다. 캐시 읽기가 비용을 크게 좌우합니다.
- **표본: 방식당 2회, 프롬프트 3개씩.** 표본이 작으니 대략적인 범위로만 보세요.

출처: [docs/pingpong-bench.md](docs/pingpong-bench.md).

## 문서

사용 설명서는 한국어로 되어 있습니다.

- [가이드 목차](docs/guide/index.md)
- [REPL](docs/guide/repl.md): 키, 상태 줄, 권한 모드, 슬래시 명령, 설정
- [CLI](docs/guide/cli.md): 설치, 하위 명령, 플래그, 환경 변수, 보조 스크립트
- [모델과 에이전트](docs/guide/models-and-agents.md)
- [훅과 스킬](docs/guide/hooks-and-skills.md)
- [핑퐁 벤치마크](docs/pingpong-bench.md)
- [변경 기록](CHANGELOG.md)
- [Rust 전환 계획](plan/rust-port.md)

## Go 버전에서 업그레이드

> **Go 빌드 삭제:** `deprecated/` 아래의 Go 구현은 0.10.0 이후 6번째 릴리스인 0.11.3에서 삭제했습니다. 0.11.0부터 릴리스 아카이브는 저장소 루트의 Rust 워크스페이스(`crates/cli`, 바이너리 `chaude`)에서 빌드하며, 에셋 이름은 같습니다. Go 소스는 태그 `v0.11.2`의 git 이력에 있습니다. Rust 버전은 아직 Go의 모든 기능을 다루지 않습니다. `plan/rust-port.md`를 보세요.


업그레이드하려면 설치 스크립트를 다시 실행하거나 `chaude update`를 실행하세요. 설치 스크립트는 예전 `~/.local/bin` 설치본과 그 `PATH` 줄을 지웁니다.

## 기여

저장소 루트에서 빌드하고 테스트합니다(Rust stable, `rust-toolchain.toml`로 버전 고정):

```sh
cargo build --release -p chaude-cli   # binary: target/release/chaude
cargo test
```

Go 구현은 0.11.3에서 삭제했습니다. 소스는 태그 `v0.11.2`의 git 이력에 있습니다.

이슈: 버그 제보와 기능 제안은 [GitHub Issues](https://github.com/int-ttt/chaude/issues)에 남겨 주세요.

기여 가이드라인: 아직 공개되지 않았습니다.

### 릴리스

**release-dispatch** 워크플로를 `master`에서 실행하세요 (Actions → release-dispatch → Run workflow, `v` 없이 `X.Y.Z`를 입력합니다). 또는 셸에서 실행하세요:

```sh
gh workflow run release-dispatch.yml -f version=X.Y.Z
```

워크플로가 코드와 문서의 버전을 올리고, `CHANGELOG.md`의 `## Unreleased`를 `## X.Y.Z - DATE`로 옮긴 뒤, `release: X.Y.Z` 커밋을 만듭니다. 이어서 `vX.Y.Z` 태그를 푸시하고, `release.yml`로 `checksums.txt`와 릴리스 아카이브 4개를 배포합니다. 실행하기 전에 `CHANGELOG.md`의 `## Unreleased`에 항목을 추가하세요.

## 라이선스

MIT 라이선스입니다. 자세한 내용은 [LICENSE](LICENSE)를 보세요.
