# v0.8.2 S3..S13 recheck (explore, read-only)
date: 2026-10-30
sources: notes/runs/261030-explore-v082-step-status.md (STALE: S7-S10 now applied), notes/runs/261030-plan-v082-write-block-final.md sec 3, 7-12
method: grep/sed/git diff --stat only. no edits, no py_compile, no pyright, no go test.

## S3 discrepancies (.claude/hooks/haiku-context-guard.py)
- env name: spec "CLAUDE_HARNESS_ALLOW_SONNET_IO" (plan 3.2 #6 line 55; sec 11 row 21 line 217). hook L53: `os.environ.get("CHAUDE_HARNESS_ALLOW_SONNET_IO") == "1"`; docstring L6 same CHAUDE_. Effect: row 21 (env CLAUDE_HARNESS_ALLOW_SONNET_IO=1 -> allow) would deny.
- env prefix in repo hooks: CLAUDE_ x many (CLAUDE_LEAN, CLAUDE_PROJECT_DIR, CLAUDE_ALLOW_MAIN_TOOLS); CHAUDE_ x2 (CHAUDE_ROTATING L55, new CHAUDE_HARNESS_ALLOW_SONNET_IO L53). User decides which prefix (사용자 확인 필요).
- statusline exception: hook L55-56 `if kind == "statusline-setup" and os.environ.get("CHAUDE_ROTATING") == "1": sys.exit(0)`; docstring L6-7. Plan 3.2 has NO such exemption. Row 16 (statusline-setup Edit -> deny) passes only while CHAUDE_ROTATING unset; with CHAUDE_ROTATING=1 it allows. (사용자 확인 필요 A open.)
- other S3 items match plan: DENY_MSG L29 verbatim; ALLOW_IO L28; getattr L18; exception path L68-70 exit 2; no-agent_id L48-49 exit 0; allow_large deny L62-63.

## S7 (agent text) - applied, verify
- sonnet-analyst.md L4 `tools: Agent`; L18 "Scope first: you have no Read/Write/Edit/Grep/Glob/Bash/WebSearch/WebFetch."  done
- sonnet-reviewer.md L4 `tools: Agent`; L11 "Do not edit. Read nothing directly: haiku-scout reads and returns excerpts with `RETURN: verbatim <=4000 chars`;" done
- opus-architect.md L4 `tools: Agent`; L11 "No direct file access (v0.8.2): you have only the Agent tool - no Read/Write/Edit/Bash/Grep/Glob." done
- verify: grep -n "^tools:" .claude/agents/sonnet-*.md .claude/agents/opus-*.md -> `tools: Agent` only (ok now)
  grep -c "WebSearch\|WebFetch\|Bash" on those tools lines -> 0 (ok)
  grep -c "RETURN: verbatim <=4000 chars" each -> analyst 2, reviewer 1, architect 3 (ok)
  grep -c "v0.8.1" .claude/agents/opus-architect.md -> 0 (ok)

## S8 (CLAUDE.md L10-12) - applied, verify
- L10 "- sonnet/opus 에이전트 파일 직접 접근 전면 금지 (Read·Write·Edit·Grep·Glob 및 Bash/PowerShell 포함, 훅 강제): 파일 읽기는 haiku-scout(RETURN: verbatim <=4000 chars), 쓰기는 haiku-batch-worker/haiku-coder, 웹은 haiku-researcher가 수행. sonnet-analyst는 ..." (extra clause after 수행.)
- L11 "...hand work only to `haiku-*` agents (D11)" done (no "may call" left)
- L12 "Opus never reads or writes files directly ... haiku-batch-worker ... haiku-drafter (user-voice prose only)." done
- verify: grep -n "may call" CLAUDE.md -> 0 hits (ok); grep -c 'haiku-drafter`/`sonnet-analyst' CLAUDE.md -> 0 (ok)

## S9 (notes/DECISIONS.md) - applied
- L43 "- [agents] 2026-10-30 v0.8.2 user: ... (훅 강제; 탈출구 CHAUDE_HARNESS_ALLOW_SONNET_IO=1); update to 35: ..."; diff +1 line (ok format)
- differs from plan 9 text: date prefix added, env name CHAUDE_ (see S3)
- verify: git diff -- notes/DECISIONS.md -> exactly 1 added line starting "- [agents]"

## S10 (CHANGELOG.md) - applied, 1 FAIL
- L3 "## 0.8.2" (above "## 0.8.1" L11): ok. bullets L4-L10. L9 "- harness-only change (cmd/chaude version 0.8.1 유지)" ok.
- L8 "- 탈출구: 환경변수 CHAUDE_HARNESS_ALLOW_SONNET_IO=1 (기본 off)" -> acceptance text "CLAUDE_HARNESS_ALLOW_SONNET_IO=1" NOT present (grep -c = 0). FAIL unless env name is settled (S3).
- verify: head -3 CHANGELOG.md -> "# Changelog", "", "## 0.8.2" (plan says head -3 shows ## 0.8.2 first; actual 0.8.2 is L3); grep -c "CLAUDE_HARNESS_ALLOW_SONNET_IO=1" CHANGELOG.md -> expect 1, now 0.

## S11 (read-only grep) - run by me
- cmd: grep -rln "sonnet-analyst\|sonnet-reviewer\|notes/runs" internal cmd README.md docs .claude/hooks/main-thread-guard.py
- hits (files): internal/harness/{classifier.go(7 lines), switchchild.go(2), drain.go(1), repl.go(1), drain_test.go, conductor_test.go, panel_test.go}; docs/claude-lean/{CHANGELOG.md, README.md, FRESH_START.md, APPLY_GUIDE.md}; .claude/hooks/main-thread-guard.py
- not inspected: Go hit lines, docs/claude-lean/CHANGELOG.md.
- write-routing candidates seen: docs/claude-lean/APPLY_GUIDE.md:157 "답이 너무 짧아 이해가 어려움 | "자세히" 요청 → `sonnet-analyst`가 `notes/runs/`에 작성" (sonnet writes directly -> S12 candidate)
- docs/claude-lean/FRESH_START.md:57 "Sonnet `sonnet-analyst`가 합칩니다 ... 결과는 `notes/*.md` 5개입니다." (ambiguous; check)
- main-thread-guard.py:29 "writing->haiku-drafter" (main-thread deny text; not sonnet write; likely no change)
- README.md / docs hits at README.md:11,13,43 and APPLY_GUIDE.md:52,82,87 are naming/routing only (no change).

## S12 (fix write-routing hits) - not run
- candidate change: docs/claude-lean/APPLY_GUIDE.md:157 "`sonnet-analyst`가 `notes/runs/`에 작성" -> "`sonnet-analyst`가 분석하고 `haiku-batch-worker`가 `notes/runs/`에 verbatim 저장" (exact wording = 사용자 확인 needed; plan gives no text)
- Go hits in internal/harness not inspected: S12 needs read of each hit line before any change.
- verify: re-run S11 grep; 0 sonnet/opus direct-write hits. If none: record "no change".

## S13 (verification) - not run
- pyright .claude/hooks/ -> expect 0 errors, 0 warnings (not run)
- grep -n "_s.reconfigure(" .claude/hooks/*.py -> expect 0 (explore said 0; not re-run)
- grep -n "BASH_READ\|BASH_WRITE\|PS_FILE" .claude/hooks/haiku-context-guard.py -> expect 0 (not re-run)
- grep -l "RETURN: verbatim" .claude/agents/haiku-*.md -> expect 8 (not re-run)
- grep -n "Read|Bash" .claude/settings.json -> expect 0; grep -n "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell" .claude/settings.json -> expect 1 (not re-run)
- sample rows plan sec 11 (rows 1-23) and pin rows P1-P5 via `python3 -I .claude/hooks/<file>.py` with JSON stdin: not run.
- row 21 will fail until S3 env name settled; row 16 depends on CHAUDE_ROTATING exemption.
- python3 -I -m py_compile <hook>, then remove __pycache__: not run.

## not found / not done
- no edits, no commit/tag/push. no py_compile/pyright/go test run.
- S0 pre-state file notes/runs/261030-verify-v082-block.md: not checked here (stale explore said missing).
- G1 sonnet-analyst agent_type check: not checked here.
- 사용자 확인 필요 A (statusline exemption), B (escape hatch), F (agent_type), env prefix: open.
