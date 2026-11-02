# PR #1 review (open PRs: 1)
pr: #1 | title: Fable 지원, Windows 훅·설치기, chaude install/open/우클릭 메뉴, /사용량, 모드 저장 | author: float421
base: 9522bc4 (0.7.2), master is 71 commits ahead; head f69baa25; 8 commits; +3633 diff lines, 60+ files
verdict: partial
applied: 71f5c41 (.gitattributes *.sh/*.py eol=lf; .gitignore .claude/lean-off)
skipped:
- pyrun launcher + settings.json hook rewrite: superseded by master gen-settings (py -3 exec form for windows)
- install.sh/install.ps1, release.yml zip, build-release.sh: master already has 6-OS install and zip release (37da38f, 1c5db93)
- fable-architect agent + classifier direct routing: master already has fable main model; routing change conflicts with master tui/classifier rewrite, product decision
- chaude install/open/context-menu (internal/install 627 lines): inverse of master chaude clean (5312153); needs design call
- REPL /사용량, IME cursor, DEC 2026 sync output, mode persistence: overlap rewritten repl.go/tui.go; unverifiable here
- .gitattributes `* text=auto`, `*.ps1 eol=crlf`: global renormalization, not minimal
- .gitignore .claude/chaude-mode, .claude-backup-*: unused in master
note: working-tree .gitignore has user's unstaged `e2e/results/` line; left unstaged (index-only edit for the commit)
checks:
- cd src/main/intt && go vet ./... -> rc 0
- cd src/main/intt && go test ./... -count=1 -> ok clean, ok harness, ok scripts, ok settings (no FAIL)
- bash -n: no .sh changed
attribution: commit used Co-Authored-By Haiku 5.5 (system reminder); brief said Opus 5.5 — flag to main agent
open: no GitHub write actions performed; PR #1 untouched
