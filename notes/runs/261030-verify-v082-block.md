# 261030 verify v0.8.2 block (S13)

## pre-state
- status: git diff of pre-change state NOT captured in this session (no snapshot before S1-S12 edits).
- post-state diff vs HEAD (1af0f16) recorded below.
- .claude/hooks/haiku-context-guard.py: 74 lines changed (diff --stat: 58 insertions, 18 deletions) vs HEAD.
- .claude/settings.json: 1 line changed (haiku-context-guard matcher, line 70) vs HEAD.
- settings.json post matcher: `Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell`
- full diff: `git -C /Users/leekwangseok/PycharmProjects/claude-harness diff -- .claude/hooks/haiku-context-guard.py .claude/settings.json`

## sample rows (plan section 11), hooks run with `python3` (no -I)
- driver: /tmp/v082_rows/rows.py (scratch, outside repo)
- row01..row23 exp/got: all PASS (23/23); row18 (broken JSON with agent_id, sonnet-analyst) rc=2 deny.
- pin rows P1..P5 (agent-model-pin.py): all PASS (5/5).
  - P1 sonnet-analyst -> sonnet-reviewer: deny
  - P2 sonnet-reviewer -> sonnet-reviewer: deny
  - P3 sonnet-reviewer -> haiku-scout: allow
  - P4 haiku-scout -> haiku-coder with model opus: allow, model removed from updatedInput
  - P5 main thread Agent sonnet-reviewer: allow
- TOTAL rows=23 pin=5 FAILS=0

## doubt: plan form `python3 -I`
- `python3 -I` row01 (sonnet-analyst Read): rc=0 got=allow (expected deny).
- cause: `from _lean import lean_off` runs before `raw = sys.stdin.read()`; with -I the script dir is not on sys.path, import fails, raw is "", fail-closed branch (`'"agent_id"' in raw`) is skipped -> exit 0.
- Claude Code runs hooks without -I, so production path is unaffected. Plan's -I check is not valid as written.
- not fixed here (out of S13 scope). Suggest: read stdin before the _lean import.

## self-check greps (plan section 12)
- `grep -c 'Read|Bash' .claude/settings.json` = 0 (PASS)
- new matcher present in settings.json = 1 (PASS)
- `^tools:` in sonnet-*/opus-*: sonnet-analyst.md:4 `tools: Agent`, sonnet-reviewer.md:4 `tools: Agent`, opus-architect.md:4 `tools: Agent` (PASS)
- `RETURN: verbatim` in haiku-*.md = 8 files (PASS)
- `BASH_READ|BASH_WRITE|PS_FILE` in haiku-context-guard.py = 0 (PASS)
- `may call` in CLAUDE.md = 0 (PASS)
- `_s.reconfigure(` in .claude/hooks/*.py = 0 (PASS)
- docs/claude-lean/CHANGELOG.md: git status clean (unchanged), no 0.8.1 entry added (PASS)
- git tag: `v0.8.1` already points at HEAD 1af0f16 (pre-existing; not created by this run). No new commit/tag by this run.

## S11 hit list (read-only grep, no edits)
- pattern: `sonnet-analyst|sonnet-reviewer|notes/runs` in internal/ cmd/ README.md docs/ .claude/hooks/main-thread-guard.py
- total lines: 37
- write-routing hits telling sonnet/opus to write files directly: 0 after S12 (manual classification)
- non-blocking hits (for reference):
  - internal/harness/classifier.go:76 sonnetHint: delegate to sonnet-analyst (no write instruction)
  - internal/harness/classifier.go:81-82 planHint: Haiku main writes plan and calls opus-architect (writer is Haiku main, not sonnet/opus)
  - internal/harness/switchchild.go:237-239, drain.go:12, panel_test.go, drain_test.go, conductor_test.go:152: path matching only
  - docs/claude-lean/FRESH_START.md:57 and APPLY_GUIDE.md:157: sonnet-analyst writes content, haiku-batch-worker saves (S12 result)

## concurrency note
- two background agents were running during S13: a76359632a76de6d1 ("v0.8.2 차단 구현 실행") and ab9dd0119751f1790 ("역할분담 재조정 후 v0.8.2 이어서 실행"). They may edit the same files; results above reflect file state at check time.

## pyright
- `pyright .claude/hooks/` -> `0 errors, 0 warnings, 0 informations` (PASS)
