done: harness/osenv.py (new), core.py+repl.py use it, scripts/gen_settings.py, platform_setup/install.{sh,ps1}, DECISIONS.md
verified: bash -n install.sh ok; gen_settings mac == settings.json (diff empty); windows variant 11 hooks python3->python; unittest tests.test_acceptance 2 OK on mac; hook correction-guard rc=0
not verified: install.ps1 (no pwsh), windows/linux runtime, Claude Code hook shell on Windows (assumes Git Bash so $CLAUDE_PROJECT_DIR works)
note: install.* overwrite .claude/settings.json (git-tracked); run on mac would be no-op
update2: windows hooks exec form (10 hooks), PowerShell permission rule, linux XDG / windows LOCALAPPDATA state dir, mac unchanged
verified2: mac settings diff empty; windows JSON parses; unittest 2 OK; bash -n ok; state_dir simulated for 3 OS; install.ps1 + windows runtime unverified (no pwsh)
uncertain: exec-form needs `python` on PATH (py launcher not used); docs page read only first 100k chars
