# v0.8.2 S3..S13 fix + verify
date: 2026-10-30
source: notes/runs/261030-explore-v082-s3-s13-check.md, plan notes/runs/261030-plan-v082-write-block-final.md sec 3.2, 9, 11, 12
scope: fix listed deviations (S3 env typo, Bash always deny, statusline gate, S10 env name) + pending S11-S13. no commit, tag, push.
git: branch master; other working-tree changes untouched (no reset/stash).

## S3 hook fix
- changed: .claude/hooks/haiku-context-guard.py (docstring L6-7; body L53-59)
- changes:
  1. env CHAUDE_HARNESS_ALLOW_SONNET_IO -> CLAUDE_HARNESS_ALLOW_SONNET_IO (body + docstring)
  2. Bash/PowerShell deny moved before escape hatch and statusline exemption (no exception for shell tools)
  3. file tools (FILE_TOOLS) still deny for non-haiku- agent_type unless escape or statusline gate
  4. kept: CHAUDE_ROTATING=1 gate for statusline-setup (file tools only). no Go code sets CHAUDE_ROTATING (grep: hook only).
- git diff --stat: .claude/hooks/haiku-context-guard.py | 76 ++++++++++++++++++++++++++++--------
- check: python3 -I -c "import ast; ast.parse(...)" -> ok
- check: grep -c 'BASH_READ\|BASH_WRITE\|PS_FILE' -> 0 ; grep -c '_s.reconfigure(' -> 0 ; grep -c 'getattr(_s' -> 1
- result: PASS (syntax + greps). behavior: section S13 rows.
- open: 사용자 확인 A (statusline exemption kept, gated on CHAUDE_ROTATING=1, which nothing sets). no design change made.

## S4 agent-model-pin.py (verify only, not edited here)
- grep -c '_s.reconfigure(' .claude/hooks/agent-model-pin.py -> 0
- sonnet/opus caller check L31 < lean_off() L37 -> PASS

## S10 CHANGELOG.md
- changed: CHANGELOG.md L10 "CHAUDE_HARNESS_ALLOW_SONNET_IO=1" -> "CLAUDE_HARNESS_ALLOW_SONNET_IO=1"
- also changed (same typo, S9 line): notes/DECISIONS.md L43 "탈출구 CHAUDE_HARNESS_ALLOW_SONNET_IO=1" -> "CLAUDE_..."
- git diff --stat: CHANGELOG.md | 10 +++++ ; notes/DECISIONS.md | 1 +
- command: head -3 CHANGELOG.md -> "# Changelog" / "" / "## 0.8.2"
- command: grep -c "CLAUDE_HARNESS_ALLOW_SONNET_IO=1" CHANGELOG.md -> 1
- command: grep -c "harness-only change" CHANGELOG.md -> 1
- command: grep -rl "CHAUDE_HARNESS" .claude notes/DECISIONS.md CHANGELOG.md CLAUDE.md -> (no output)
- result: PASS

## S9 DECISIONS.md (verify)
- git diff -U0 -- notes/DECISIONS.md: 1 added line, starts "- [agents]"
- result: PASS

## S11 read-only grep (no edits)
- command: grep -rln "sonnet-analyst\|sonnet-reviewer\|notes/runs" internal cmd README.md docs .claude/hooks/main-thread-guard.py
- hits (path:line, write-routing relevant):
  - internal/harness/classifier.go:82 planHint step (2): "call opus-architect ... ask for improvements only, written to notes/runs/<YYMMDD>-plan-<topic>-review.md" (opus writes file directly: candidate)
  - internal/harness/classifier.go:76-77 sonnetHint: delegate to sonnet-analyst; no direct write
  - internal/harness/classifier.go:81 planHint step (1): main (Haiku) writes plan: not sonnet/opus
  - internal/harness/switchchild.go:237-239, drain.go:12, drain_test.go:16, panel_test.go:175-191, conductor_test.go:121-162, repl.go:1016: path parsing/tests/help text: no change
  - docs/claude-lean/APPLY_GUIDE.md:157 "`sonnet-analyst`가 작성하고 `haiku-batch-worker`가 `notes/runs/`에 저장": already verbatim-save form
  - docs/claude-lean/FRESH_START.md:57 "`haiku-batch-worker`가 결과를 `notes/*.md` 5개로 저장": already form
  - docs/claude-lean/README.md:11,13,43; APPLY_GUIDE.md:52,82,87; docs/claude-lean/CHANGELOG.md (history, untouched): naming only
  - .claude/hooks/main-thread-guard.py:29 main-thread deny text "writing->haiku-drafter": no change (sec 13)
- result: hit list recorded

## S12 fix write-routing hits
- docs: no change needed (APPLY_GUIDE:157 and FRESH_START:57 already route writes to haiku-batch-worker)
- internal/harness/classifier.go:82 planHint tells opus-architect to write the review file: needs Go source change (plan sec 13 says Go sources must not change) and a wording decision. NOT changed. question below.
- result: OPEN (design question)

## S13 verification
### pyright
- command: cd /Users/leekwangseok/PycharmProjects/claude-harness && pyright .claude/hooks/
- output: "0 errors, 0 warnings, 0 informations" (plus version-update notice: 1.1.409 -> 1.1.414)
- result: PASS

### greps
- grep -c '_s.reconfigure(' .claude/hooks/*.py -> all 0 (10 files)
- grep -c 'BASH_READ|BASH_WRITE|PS_FILE' haiku-context-guard.py -> 0
- grep -l 'RETURN: verbatim' .claude/agents/haiku-*.md -> 8 ; grep -l 'max 6000' -> 8
- grep -n 'Read|Bash' .claude/settings.json -> 0 ; grep -c 'Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell' settings.json -> 1
- grep -n '^tools:' sonnet-*.md opus-*.md -> "tools: Agent" x3
- grep -c 'may call' CLAUDE.md -> 0 ; grep -c 'v0.8.1' opus-architect.md -> 0
- git status note: __pycache__ count in .claude/hooks -> 0
- result: PASS

### sample rows plan sec 11 (haiku-context-guard.py)
- literal command `python3 -I` is NOT usable: hook imports `_lean` (sibling module); -I drops script dir from sys.path -> ModuleNotFoundError swallowed by the hook's generic except -> every row allow (row 18 exit 0). 26/27 mismatched under -I. Plan defect; ran without -I (what Claude Code does).
- command: printf JSON | env -i PATH=$PATH [env] python3 .claude/hooks/haiku-context-guard.py
- results (exp / got):
  1 deny/deny; 2 deny/deny; 3 deny/deny; 4 deny/deny; 5 deny/deny; 6 deny/deny; 7 deny/deny; 8 deny/deny; 9 deny/deny
  10 allow/allow; 11 deny/deny; 12 allow/allow; 13 deny/deny; 14 deny/deny; 15 deny/deny; 16 deny/deny; 17 allow/allow
  18 deny/deny (exit 2, stderr DENY_MSG); 19 allow/allow; 20 deny/deny; 21 allow/allow (CLAUDE_HARNESS_ALLOW_SONNET_IO=1)
  22 deny/deny; 23 allow/allow; 24 deny/deny (Bash with escape env still denied)
  extra 25: sonnet-analyst Read with CHAUDE_HARNESS_ALLOW_SONNET_IO=1 -> deny (typo no longer honored; my pre-run label "allow" was wrong, deny is intended)
  extra 26: statusline-setup Edit with CHAUDE_ROTATING=1 -> allow (kept gate)
  extra 27: statusline-setup Bash with CHAUDE_ROTATING=1 -> deny
- result: 27/27 as intended (row 25 expectation corrected) -> PASS
- note: row 22 label "lean_off true (default)" conflicts with _lean.py (default CLAUDE_LEAN=1 -> lean_off False); sonnet deny holds either way.
- note: row 11 (haiku allow_large deny) depends on lean_off() False (default holds; no .claude/lean-off file).

### pin rows (agent-model-pin.py)
- literal `python3 -I` fails: `ModuleNotFoundError: No module named '_lean'` (same cause). ran without -I.
- P1 sonnet-analyst -> subagent sonnet-reviewer: deny (exp deny) OK
- P2 sonnet-reviewer -> sonnet-reviewer: deny (exp deny) OK
- P3 sonnet-reviewer -> haiku-scout: silent allow (exp allow) OK
- P4 haiku-scout -> haiku-coder model opus: allow, "model 'opus' removed" (exp allow, model removed) OK
- P5 main -> sonnet-reviewer: silent allow (exp allow) OK
- result: PASS 5/5

## not done / open
- S0 pre-state file and G1 (live agent_type check): not in this brief, not run.
- 사용자 확인 필요 A: statusline-setup exemption kept, gated on CHAUDE_ROTATING=1; no code sets it. Plan row 16 expects deny when unset (passes).
- 사용자 확인 필요 (S12 question): internal/harness/classifier.go:82 planHint asks opus-architect to write notes/runs/...-review.md directly. Options: (a) change Go hint to "haiku-batch-worker saves the review" (Go source + tests conductor_test.go:152 check "notes/runs/" still ok) or (b) leave. Needs your decision; Go not touched.
- plan defect: sec 11/pin rows say `python3 -I`; hooks import `_lean`. Suggest rows run without -I.
- DECISIONS/CHANGELOG env typo fixed beyond S10 text (same typo).
