# Changelog

## 0.7.0
- Fable 지원: `fable-architect` 에이전트(`fable로`로만 호출, Opus가 실패한 일에 사용), 컨덕터 분류기가 `fable`도 라우팅(프롬프트에 `fable로`/`페이블로`가 있을 때만), `chaude --model fable`, `chaude install --main fable`. 훅(`agent-model-pin`, `haiku-context-guard`, `ledger`)이 `fable-*` 이름을 인식.
- Windows: `chaude install`이 훅을 셸 없이 `py -3 <훅>`으로 실행하는 형태로 설치(검증: Claude Code는 Windows에서 훅을 Git Bash로 실행하고, exec 형태는 `py` 런처로 동작·Store `python` 별칭은 실패). 저장소 `settings.json`은 `.claude/hooks/pyrun` 런처로 어느 OS에서나 동작. `claude` 실행 파일을 PATH에서 찾음(classifier, `/version`). 한국어 Windows 코드페이지에서 사용량 바 폭이 2배로 계산되던 문제 수정. `.gitattributes`로 `.sh`/`.py`/`pyrun` LF 고정.
- `chaude install <프로젝트> [--main MODEL] [--python CMD] [--lean-off] [--os ...]`: 에이전트·훅·스킬·설정·CLAUDE.md 정책·notes/·logs/·bin/chaude를 한 번에 설치/업그레이드. 패키지 파일은 바이너리에 내장. 기존 settings.json은 병합(사용자 훅·튜닝값 유지, 패키지 훅만 교체), CLAUDE.md는 BOM·CRLF 유지하며 정책 블록만 추가/교체, 백업은 `.claude-backup-<시각>/`.
- 릴리스: mac(arm64/amd64)·linux(amd64/arm64)·windows(amd64/arm64) 아카이브, `install.ps1`(Windows), `install.sh`는 mac/linux 공통.
- 컨덕터: 프롬프트에 `haiku로`/`sonnet으로`/`opus로`/`fable로`(한글 이름 포함)가 있으면 Sonnet 분류기를 부르지 않고 바로 그 모델로 라우팅(호출 1회·3~5초 절약). 분류기 라이브 테스트에 fable 케이스 추가.
- 루트 `README.md` 추가. 유사 프로젝트(RTK, caveman, claude-mem, context-mode, oh-my-claudecode, token-savior 등) 조사 결과는 `notes/research/similar-projects.md`.

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
