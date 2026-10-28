# go-port result (integration)
parts: harness -> 261028-go-harness.md | hooks-a (style-adapter, session-context) -> 261028-go-hooks-a.md 193/193 | hooks-b (7 hooks) -> 261028-go-hooks-b.md 86/88 (2 intended: bad stdin/bad LEAN_RESET_AT exit 0 not 1) | scripts (7) -> 261028-go-scripts.md 44/44
go: system go1.23.5 (install.sh also verified download path go1.27.3 + sha256 into .tools/go in scratch run)
checks:
- gofmt -l .: clean; go vet ./...: clean; go test -count=1 ./...: ok internal/harness
- live: HARNESS_LIVE=1 go test ./internal/harness: TestLiveRotationCarriesContext PASS, TestLiveStreamingMultipleDeltas PASS (from harness part)
- cross: CGO_ENABLED=0 linux/amd64, linux/arm64, windows/amd64, windows/arm64, darwin/amd64: all ok
- bin/harness built (go build -trimpath); bash -n platform_setup/install.sh: ok
- hooks via bin/harness: main-thread-guard Read -> deny JSON rc0; session-context startup -> additionalContext rc0; style-adapter "ㅇㅇ" -> [ack] text rc0; context-alert bad json -> silent rc0; unknown hook -> stderr msg rc0
- gen-settings mac == .claude/settings.json; gen-settings windows -> 11 harness.exe refs (10 hooks + permission)
- history-export: hook does not read stdin; export-history --hook reads scripts.Stdin (=os.Stdin) directly, so hook JSON reaches it
cleanup: deleted .claude/hooks/*.py, scripts/ (incl gen_settings.py, __pycache__), harness/ (incl osenv.py), spike/, tests/, difftest drivers; no non-comment python refs left in internal/ cmd/ .claude/
unverified: install.ps1 (no pwsh), Windows runtime of exec-form hooks
risks: claude-lean reinstall may restore .py hooks/settings and agent .md commands; docs/claude-lean/* still describe Python; style_adapter.go holds shared helpers (pyStrip, truthy, ...) used by session_context.go

## hooks back to python (user: "아니 go보다 각 환경에서 계속 수정하려면 python으로 가는게 나을것같아")
done: .claude/hooks/*.py restored from git HEAD (88d8158); internal/hooks/ deleted; `hook` subcommand removed from cmd/harness/main.go; internal/lean trimmed to ParseInterspersed (rest was hook-only)
hook edits (deps on deleted scripts/*.py): history-export.py -> subprocess `bin/harness[.exe] export-history --hook --project ROOT`, stdin bytes passed, out/err discarded, timeout 50s, skip if binary missing; session-context.py -> `from _lean import history_dir, is_ping` (history_dir copied into _lean.py from old scripts/claude_history.py)
settings: .claude/settings.json = HEAD form (python3 "$CLAUDE_PROJECT_DIR/.claude/hooks/X.py") + permission Bash(bin/harness search-history:*)
gen-settings windows: --python CMD (split on spaces unless whole string is an existing file); default `py -3` if on PATH (only checked on windows) else python; already-converted input re-targeted (idempotent: python -> py -3 -> python == first output); PowerShell permission deduped
install.sh: python3 >= 3.8 check, exit 1 with install hint; install.ps1: Test-Python over $HARNESS_PYTHON | py -3 | python | python3 (>=3.8 run check, rejects Store alias), throws if none, passes --python to gen-settings
checks: gofmt/vet clean; go test -count=1 ./... ok; cross linux/{amd64,arm64} windows/{amd64,arm64} darwin/amd64 ok; bash -n install.sh ok; gen-settings mac == settings.json; windows: 10 hooks command=python args=[${CLAUDE_PROJECT_DIR}/.claude/hooks/X.py]
python hooks run on mac (python3 3.9): main-thread-guard Read -> deny JSON; agent-model-pin model on haiku-coder -> allow+model removed; session-context startup -> additionalContext incl. "Previous session" carry-over (history_dir == bin/harness claude-history); style-adapter "ㅇㅇ" -> [ack]; correction-guard plain -> no output; history-export (temp project + copied binary + real transcript) -> logs/history/<..>.txt + INDEX.txt, rc 0; context-alert bad JSON -> traceback (original behavior)
unverified: install.ps1 (no pwsh), Windows runtime of exec-form hooks
