# chaude
Terminal driver for the installed `claude` CLI (stream-json). Auto-rotates sessions with a handoff snapshot. Personal use, no distribution. See notes/spike-results.md.

Claude Code hooks are Python (`.claude/hooks/*.py`, edit in place on any OS; needs Python 3.8+). Helper scripts and the REPL are one Go binary: `bin/chaude` (`bin\chaude.exe` on Windows).

Install / build (installs a project-local Go under `.tools/go` if Go 1.22+ is missing, checks Python, builds, writes `.claude/settings.json` for the OS — windows hooks run via `py -3` or `python`):
- mac / linux: `platform_setup/install.sh` (`--cross` also builds all OS/arch into `dist/`)
- windows: `powershell -ExecutionPolicy Bypass -File platform_setup\install.ps1` (`-Cross` likewise)

Run: `bin/chaude repl --threshold 200000`   Test: `go test ./...` (live: `HARNESS_LIVE=1 go test ./internal/harness -v`)
Hooks: `python3 .claude/hooks/<name>.py` (hook JSON on stdin) · scripts: `bin/chaude search-history|charcount|export-history|extract-transcript|compress-archive|usage-report|claude-history` · settings: `bin/chaude gen-settings mac|linux|windows [--python CMD]`

## 설치 (macOS Apple Silicon 전용)

`chaude`는 claude-lean(0.5.3)의 후속입니다. 이름이 바뀌었고 첫 릴리즈는 v0.6.0입니다. 변경 내역은 `CHANGELOG.md`.
비공개 저장소는 `gh auth login` 후 `gh release download`로 받습니다.

터미널에서 한 줄로 설치합니다.

```sh
gh api repos/int-ttt/chaude/contents/install.sh -H "Accept: application/vnd.github.raw" | sh
# 공개 저장소라면: curl -fsSL https://raw.githubusercontent.com/int-ttt/chaude/main/install.sh | sh
chaude --version
```

- 최신 릴리즈를 받아 `/usr/local/bin`(쓰기 가능 시) 또는 `~/.local/bin`에 설치하고, PATH에 없으면 `~/.zshrc`에 추가합니다.
- macOS 격리 속성(quarantine)을 제거하고 `chaude --version`으로 확인합니다.
- 환경변수: `CHAUDE_VERSION=v0.1.0`(특정 버전), `CHAUDE_INSTALL_DIR=경로`.

릴리즈: `git tag v0.1.0 && git push origin v0.1.0` 하면 GitHub Actions(`.github/workflows/release.yml`)가 darwin/arm64 바이너리를 빌드해 릴리즈를 만듭니다.
