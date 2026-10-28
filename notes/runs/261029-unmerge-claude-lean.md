# unmerge claude-lean
overwritten_local_files: none (merge diff vs 96765d3^1 was add-only, 28 files)
removed (git rm):
- ANALYSIS.md
- APPLY_GUIDE.md
- DESIGN_HANDOFF.md
- FRESH_START.md
- MIGRATE_NOW_PROMPT.md
- PLAN_0.5.0.md
- VERSION
- fresh_start.sh
- history/20261027-0444-36fb8dbb-01.txt
- history/INDEX.txt
- mac/launcher.sh
- scripts/charcount.py
- scripts/claude_history.py
- scripts/compress_archive.py
- scripts/export_history.py
- scripts/extract_transcript.py
- scripts/search_history.py
- scripts/usage_report.py
- tests/test_hooks.py
- tools/build_release.py
- tools/configure.py
- tools/merge_settings.py
- tools/usage_by_task.py
- win/build_win.py
- win/fresh_start.ps1
- win/install.ps1
- win/installer/Installer.cs
- win/tools/common.ps1
kept: 73a9735 .gitignore + cmd/chaude/main.go; notes/LEDGER.md local edit untouched, not committed
verify: go build ok, go test ok; git diff 96765d3^1..HEAD = .gitignore, cmd/chaude/main.go only
readme: README.md already pre-merge local version (merge did not touch it); updated version examples to 0.6.1
build: scripts/build-release.sh 0.6.1 -> dist/chaude_0.6.1_darwin_arm64.tar.gz; chaude --version checked
push: origin master:main, no force, no tags
0.6.2: bumped (main.go, README, CHANGELOG), commit 4e03bd4 pushed to main, new tag v0.6.2 (v0.6.1 untouched)
release run 40719026994 success; assets chaude_0.6.2_darwin_arm64.tar.gz + checksums.txt; install.sh into temp dir -> chaude --version checked
