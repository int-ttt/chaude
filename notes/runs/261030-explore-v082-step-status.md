# v0.8.2 write-block: step status (explore, read-only)
date: 2026-10-30
sources: notes/runs/261030-explore-v082-steps.md, 261030-plan-v082-write-block-final.md (261 lines, sec 2-14), 261030-plan-v082-write-block-review.md
method: repo state checked by grep/sed/git diff --stat only. no edits, no py_compile/pyright/go test run.
verdict_label: status = file-state evidence, not acceptance-verified.

## order
S0 -> G1 (main live check) -> S1 -> S2 -> S3..S12 -> S13 -> G2. stop after G2. no commit/tag/push/release.

## step status
S0 pending: notes/runs/261030-verify-v082-block.md does not exist (needs "## pre-state" diff).
G1 unknown: no record of sonnet-analyst agent_type check. if agent_type absent -> STOP, ask user (사용자 확인 필요 F).
S1 done-looking: 8 haiku-*.md (coder, drafter, checker, batch-worker, runner, scout, extractor, researcher) each have 1 "RETURN: verbatim" and 1 "max 6000".
S2 unknown: scout acceptance call (>=1000 chars quoted) not recorded.
S3 done-looking: .claude/hooks/haiku-context-guard.py rewritten (no BASH_READ/BASH_WRITE/PS_FILE; getattr reconfigure; ALLOW_IO=("haiku-",); DENY_MSG verbatim; try/except fail-closed).
   DEVIATION a: escape hatch env spelled CHAUDE_HARNESS_ALLOW_SONNET_IO (line 53, also docstring line 6). spec = CLAUDE_HARNESS_ALLOW_SONNET_IO.
   DEVIATION b: extra exemption line 55: kind=="statusline-setup" and CHAUDE_ROTATING=="1" -> exit 0. spec: no exemption (사용자 확인 필요 A open).
   py_compile not run by me.
S4 done-looking: agent-model-pin.py v0.8.2 deny at line 35, lean_off gate at line 37 (new check before it); docstring line 8 updated; getattr present.
S5 done-looking: `grep -n "_s.reconfigure(" .claude/hooks/*.py` = 0 hits; getattr(_s present in 8 hooks (agent-model-pin, context-alert, correction-guard, haiku-context-guard, ledger, main-thread-guard, session-context, style-adapter).
S6 done-looking: settings.json line 70 = "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell". go test ./internal/settings not run.
   note: settings.json line 60 has another matcher "Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch" not mentioned in plan. owner unverified.
S7 pending: sonnet-analyst.md:4 "tools: Write, Bash, WebSearch, WebFetch, Agent" (spec: tools: Agent). sonnet-reviewer.md:4 "tools: Read, Grep, Glob, Bash" (spec: tools: Agent). opus-architect.md:4 "tools: Agent" (already ok; body "v0.8.1" text not checked).
S8 pending: CLAUDE.md line 10 still old "sonnet-analyst 파일 직접 접근 금지..." (spec: sonnet/opus 전면 금지 text); line 11 still "may call sonnet-reviewer"; line 12 still haiku-drafter/sonnet-analyst writer text.
S9 pending: notes/DECISIONS.md has 0 "v0.8.2 user" lines (42 lines total; last [agents] line is 261029).
S10 pending: CHANGELOG.md (repo root) head = "# Changelog" / "## 0.8.1". no 0.8.2 section.
S11 unknown/pending: read-only L5 grep not recorded.
S12 unknown/pending: depends on S11.
S13 pending: verification not recorded (pyright, greps, sample rows 1-23, pin rows P1-P5).

## spec for pending steps (short)
S7: sonnet-analyst.md tools -> "tools: Agent"; "Scope first: you have no Read/Grep/Glob/Edit." -> "...no Read/Write/Edit/Grep/Glob/Bash/WebSearch/WebFetch."; file reads via haiku-scout "RETURN: verbatim <=4000 chars"; web via haiku-researcher; answer draft via haiku-drafter; verbatim save via haiku-batch-worker.
S7: sonnet-reviewer.md tools -> "tools: Agent"; add "Read nothing directly: haiku-scout reads and returns excerpts with RETURN: verbatim <=4000 chars."; charcount + git diff via haiku-checker (RETURN: verbatim <=4000 chars); issue list via haiku-batch-worker.
S7: opus-architect.md "No direct file access (v0.8.1):" -> "(v0.8.2):"; reading -> haiku-scout (RETURN: verbatim <=4000 chars); notes/runs writes -> haiku-batch-worker.
S8: CLAUDE.md line 10 -> "- sonnet/opus 에이전트 파일 직접 접근 전면 금지 (Read·Write·Edit·Grep·Glob 및 Bash/PowerShell 포함, 훅 강제): 파일 읽기는 haiku-scout(RETURN: verbatim <=4000 chars), 쓰기는 haiku-batch-worker/haiku-coder, 웹은 haiku-researcher가 수행."; line 11 "may call sonnet-reviewer..." -> "hand work only to haiku-* agents"; line 12 writers -> haiku-batch-worker (verbatim), haiku-drafter (user-voice prose only).
S9: append one line "- [agents] v0.8.2 user: sonnet/opus/비-haiku 서브에이전트 파일·셸 전면 차단 (haiku-* 허용 목록); update to 35: sonnet-reviewer는 haiku-*만 호출 가능 (subagent caller); sonnet-reviewer/analyst/opus tools = Agent only." (plan sec 9 wording; "update to 35:" odd, copied as written).
S10: insert above "## 0.8.1" in CHANGELOG.md the 0.8.2 section (plan sec 9 text; must contain "harness-only change" and "CLAUDE_HARNESS_ALLOW_SONNET_IO=1").
S11: haiku-scout read-only grep: "sonnet-analyst", "sonnet-reviewer", "notes/runs" write routing in internal/, cmd/, README.md, docs/, .claude/hooks/main-thread-guard.py.
S12: fix only S11 hits that tell sonnet/opus to write files directly; if none, record "no change".
S13: pyright .claude/hooks/ -> 0 errors 0 warnings; greps in sec 12; sample rows sec 11.

## verification commands (from plan sec 10-12)
- grep -l "RETURN: verbatim" .claude/agents/haiku-*.md  (expect 8)
- grep -c "BASH_READ\|BASH_WRITE\|PS_FILE" .claude/hooks/haiku-context-guard.py  (expect 0)
- grep -n "_s.reconfigure(" .claude/hooks/*.py  (expect 0)
- grep -n "^tools:" .claude/agents/sonnet-*.md .claude/agents/opus-*.md  (expect "tools: Agent" only)
- grep -n "Read|Bash" .claude/settings.json  (expect 0); grep -n "...|Bash|PowerShell" .claude/settings.json (expect 1)
- grep -n "may call" CLAUDE.md  (no sonnet-reviewer call permission)
- head -3 CHANGELOG.md  (## 0.8.2 then bullets)
- pyright .claude/hooks/  (0 errors, 0 warnings)
- python3 -I -m py_compile <hook>; then remove __pycache__
- go test ./internal/settings/...; go run ./cmd/chaude gen-settings windows
- git status --short  (no commit/tag)

## doubts
- escape-hatch env name mismatch: hook CHAUDE_HARNESS_ALLOW_SONNET_IO vs spec CLAUDE_HARNESS_ALLOW_SONNET_IO. fix or align before S10 CHANGELOG.
- statusline-setup exemption present in hook (line 55) but plan says none; row 16 expects deny.
- final plan sec 14 E and review M2 use name CHAUDE_MAIN_MODEL (no step implements it; excluded from v0.8.2 per 보류 M2). explore file's "typo" note unclear; final file text is CHAUDE_MAIN_MODEL.
- DECISIONS wording "update to 35:" odd, copied as written.
- sec 14 미정 1 (/usage model-call nature) and 미정 2 (budget display) open; not needed for S1-S13.
- Go version stays 0.8.1 (D13); S6 runs go test but no Go code edited.
- S0 pre-state file missing: may matter for diff baseline (review L4).
