rename: done (by sonnet, sed; no commits)
module: chaude (go.mod + all imports chaude/internal/...)
dir: cmd/harness -> cmd/chaude; internal/harness package name kept
version: main.go `var version = "0.6.0-dev"`; `--version|-v|version` prints "chaude <ver>"; usage/err prefixes -> chaude
refs bin/harness -> bin/chaude: README, hooks (history-export.py, _lean.py), settings.json, agents, distill skill, platform_setup install.sh/ps1 (output names chaude/dist/chaude-*)
README title -> # chaude; .gitignore += /chaude
kept on purpose: state dir name claude-harness (compat, user data path), claude-lean product refs (hooks/CLAUDE.md/version file/repo int-ttt/claude-lean), handoff_test fixture text, notes/*
verify: go build/vet/test ok; ./chaude --version -> "chaude 0.6.0-dev"; MAIN_PKG= build-release.sh 0.6.0 ok (dist removed)
not touched: .github/workflows/release.yml (no stale refs found)
