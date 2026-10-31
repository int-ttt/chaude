# digest: v0.8.2 write-block plan trio (read-only)
date: 2026-10-30
agent: explore (haiku)
scope: read 3 files fully. no repo file edited. this file is the only write.
sources:
- S1 = notes/runs/261030-plan-v082-write-block.md (original, 266 lines, "plan only")
- S2 = notes/runs/261030-plan-v082-write-block-review.md (review, 79 lines, verdict FIX)
- S3 = notes/runs/261030-plan-v082-write-block-final.md (final, 261 lines, "original + opus review applied")
note_on_section_refs:
- brief asked "final §6 확인 A-F". final §6 is "Go status items" (no 확인 list). the 확인 A-F list is final §14 "사용자 확인 필요" (lines 254-260). mapped below.
- brief asked "final §14 미정 1,2". final §14 미정 1 = /usage popup model-call nature, 미정 2 = budget USD display. these are NOT the same as review/original 미정 #1-#4 (CHANGELOG wording, docs CHANGELOG, version bump, WebSearch). both sets listed.
- original §10 holds 사용자 확인 필요 #1-#4 (+미확인 item). review 확인 #1-#4 = same numbering, with recommendations.

## user_decision (verbatim, identical in S1/S2/S3)
- "b 방식 채택. 모든 sonnet/opus 모델은 읽기/쓰기 전면 차단, bash를 통한 읽기/쓰기도 차단. 반드시 haiku를 경유하도록 강제 구현."
- S3 line 6 user_addition: "chaude status items must be computed in Go client without model call (section 6)." (S3 §6)

## status_of_plan
- S1 status: "plan only. no code edited, no commit, tag, push, or release."
- S3 status: same. S3 open_count: "미정 2 (new, no source text). original 미정 4 closed by review recommendation (section 9). 사용자 확인 필요 6 (section 10). 보류 1 (M2)."
- S2 verdict: "FIX before execution. 3 HIGH, 5 MED, 5 LOW"
- note: S3 says "section 9" and "section 10" for closures; in S3 the 확인 list is §14 and execution steps are §10. stale cross-refs in S3 (open_count line).

## A. 사용자 확인 필요 (S3 §14, lines 254-260) and review 확인 (S2 lines 75-79)
A (S3 L255): statusline-setup exemption (review H2) not included. "user words 'haiku-* 외 전부'. row 16 expects deny. if built-in /statusline must work, add ALLOW_IO_EXACT = {"statusline-setup"}."
  - S2 H2 proposed: ALLOW_IO_EXACT = {"statusline-setup"}.
  - options: (a) no exemption (S3 default, row 16 deny). (b) add ALLOW_IO_EXACT.
  - pros(a): literal user words. cons(a): built-in /statusline may break.
  - recommendation: none in S3 beyond "if ... must work, add". ask user.
B (S3 L256): escape hatch CLAUDE_HARNESS_ALLOW_SONNET_IO=1 included per 확인 #3 권장안 (off by default). "keep or remove."
  - S2 확인 #3: "Escape hatch: env `CLAUDE_HARNESS_ALLOW_SONNET_IO=1`, documented in the CHANGELOG, off by default."
  - S3 D5: "escape hatch: env CLAUDE_HARNESS_ALLOW_SONNET_IO=1 skips the block for blocked agents; off by default; documented in CHANGELOG".
  - cons: user words are unconditional ("전면 차단"); escape hatch is a bypass knob.
C (S3 L257): category/route classifier `claude -p --model` per prompt is a model call (not a status row). "confirm it stays out of the no-model-call rule."
  - S3 §6 ref: classifier.go:188. routing decision, not displayed status.
D (S3 L258): pyright installed locally: user-visible IDE warning text not captured. acceptance = `pyright .claude/hooks/` 0/0 (확인 #1 권장안).
  - S2 확인 #1: "not required. Recommend applying L1 to all hooks and accepting 'pyright .claude/hooks/ 0 warnings'. Ask the user only if a warning is still visible in the IDE after that."
E (S3 L259): 보류 (확인 #4): main session sonnet/opus block (M2, needs Go change CHAUDE_MAIN_MODEL + version bump). excluded from v0.8.2 per instruction.
  - S2 확인 #4: "should a sonnet/opus MAIN session also be blocked (M2)? This needs a Go change and a version bump. Recommend yes, per '모든 sonnet/opus'."
  - S2 M2 fix: "the chaude launcher exports `CHAUDE_MAIN_MODEL=<haiku|sonnet|opus>` (cmd/chaude, where --model is chosen)". deny FILE_TOOLS, Bash, PowerShell "regardless of lean_off and CLAUDE_ALLOW_MAIN_TOOLS=1".
  - S3 D7/§13: main thread fail-open; M2 보류.
F (S3 L260): G1 live agent_type check unverified until run. "if agent_type absent, plan stops."
  - S3 G1 (§10): "live check: one sonnet-analyst Read; confirm PreToolUse payload has agent_type 'sonnet-analyst'". acceptance: "agent_type value recorded in verify file. If absent: STOP, ask user (사용자 확인 필요 F)".
  - S2 ordering step 1: "Live check: sonnet-analyst Read shows agent_type in the payload (before tool lists change, F12)."

Original S1 §10 사용자 확인 필요 (for cross-ref):
- #1 Pyright warning text: only line 11 location known; no output captured.
- #2 reviewer option: B (Agent only) vs A (keep Read/Grep/Glob, write/Bash-write blocked) vs C (delete reviewer). plan uses B.
- #3 lean mode: plan blocks sonnet/opus even when lean_off() is true. confirm.
- 미확인: live PreToolUse payload carries agent_type for these agents (v0.8.1 code already relies on it).

## B. 미정 (S3 §14 lines 249-252)
- 미정 1: /usage popup (status row 22) model-call nature. "claude CLI process; no source states whether it uses API tokens. Go-only computation of row 22 not possible without it."
  - S3 §6 row 22: "usageFetch runs `claude -p /usage --output-format json --no-session-persistence` (tui.go:703)" ; model call: "YES, possibly ... whether it uses API tokens = 미정 1".
- 미정 2: budget USD status (budget.go) display location in chaude: "not verified in tui code." S3 §6 says budget uses "USD from usage x price table in FACTS.md".
- closed (S3 L252): original 미정 #1 CHANGELOG wording (section 9); #2 docs/claude-lean/CHANGELOG (untouched); #3 version bump (none, harness-only); #4 WebSearch/WebFetch (removed; haiku-researcher).
- S2 미정 recommendations (for original #1-#4):
  - #1 CHANGELOG 0.8.2 wording: draft "## 0.8.2 / - sonnet/opus(비-haiku) 서브에이전트의 파일·셸 직접 접근을 훅으로 전면 차단; 모든 읽기/쓰기는 haiku 에이전트 경유 / - haiku-context-guard 매처 확장: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell / ... / - 훅 Pyright 경고 수정(reconfigure getattr)".
  - #2 docs/claude-lean/CHANGELOG.md: "update only if it already has a 0.8.1 entry (grep '0.8.1'). Otherwise leave it untouched."
  - #3 version bump cmd/chaude/main.go -> 0.8.2: "yes if M2 is approved. Otherwise no, and record 'harness-only change'."
  - #4 sonnet-analyst WebSearch/WebFetch: "remove (tools: Agent only). Route web lookups to haiku-researcher with RETURN: verbatim."

## C. sonnet-reviewer 옵션 B (S3 §1 D2, §7.3; S2 확인 #2; S1 §5/§6)
- options (S2 확인 #2):
  - A: keep Read/Grep/Glob, write/Bash-write blocked. S2: "A contradicts '읽기 전면 차단'."
  - B: Agent only. RECOMMENDED. S2: "B is conditional on H1 + M3 + M5; without them B degrades review to 300-char summaries."
  - C: delete reviewer.
- S3 D2 (final decision): "sonnet-reviewer: tools = Agent only (option B, 확인 #2 권장안). reads via haiku-scout with `RETURN: verbatim <=4000 chars`; charcount and git diff via haiku-checker with `RETURN: verbatim <=4000 chars` (M5)."
- S3 §7.3 table:
  - tools: `Read, Grep, Glob, Bash` -> `tools: Agent`
  - add: "Read nothing directly: haiku-scout reads and returns excerpts with RETURN: verbatim <=4000 chars."
  - charcount: `bin/chaude charcount` run by reviewer -> "haiku-checker runs `bin/chaude charcount <file>` and `git diff -- <paths>`, `RETURN: verbatim <=4000 chars` (M5)"
  - issue list `notes/runs/<YYMMDD>-review-<topic>.md` -> "written via haiku-batch-worker (M4)"
- S1 D2 (original) differed: "reads via haiku-scout; issue list via haiku-drafter." S1 §6 used haiku-runner for charcount.
- S2 M5: "in sonnet-reviewer.md, replace 'run bin/chaude charcount' with 'haiku-checker runs ...'. the haiku-checker allow-list in settings is unchanged."
- S2 M3: "sonnet-reviewer with Agent tool reopens recursion (DECISIONS 35 assumed none)" -> agent-model-pin: sonnet-reviewer may target only haiku-*; deny sonnet-reviewer -> sonnet-reviewer.
- S3 §11 pin rows: P1 sonnet-analyst -> sonnet-reviewer deny; P2 sonnet-reviewer -> sonnet-reviewer deny; P3 sonnet-reviewer -> haiku-scout allow.
- S3 G2: "final review by sonnet-reviewer via haiku-scout excerpts (RETURN: verbatim <=4000 chars)".

## D. 차단 대상 Bash 전면 차단 방식
- original (S1 §3.2 / §3.3): regex. BASH_READ (cat|head|tail|sed|awk|gawk|grep|egrep|fgrep|rg|less|more|nl|od|xxd|strings|wc|sort|uniq|cut|find|ls|diff|cmp|jq|file|stat|tac|paste|column; git show|diff|cat-file|grep; python|node|perl|ruby -c/-e), BASH_WRITE (tee, >, >>, &>, cp|mv|rm|touch|mkdir|rmdir|ln|chmod|chown|dd|install|truncate|patch), PS_FILE (Get-Content, gc, type, cat, Set-Content, ..., dir, sc, ac, ls). S1 §3.5 known limits: "command substitution inside quotes, `eval`, `xargs cat`, `bash -c "cat x"`, `python3 -c` with other flags, heredoc redirect. Regex cannot catch all; tools list is the hard boundary."
- S2 H3 problem: "regex READ/WRITE leaves bypasses (bash -c, sh x.sh, python3 script.py, go run, `< file`, xargs, eval, git log -p/blame/checkout/apply, curl -o, bin/chaude subcmds). It also gives false positives (any `>` in args, ls/wc/tail in pipes)."
- S2 H3 fix: "blocked agents get an unconditional deny on Bash/PowerShell. sonnet/opus agents have tools: Agent anyway, so this is belt-and-braces with zero bypass. Keep the existing regex only for the Haiku allow_large rule (unchanged, D6)."
- S3 D1: "Bash and PowerShell denied entirely for blocked agents (review H3)."
- S3 D6: "Regex is removed (no longer needed: the rule reads tool_input.allow_large only)." <- DIFFERS from S2 H3, which said keep regex for Haiku allow_large. S3 follows its own D6 (regex removed; allow_large check reads tin.get("allow_large") only).
- S3 §3.2 hook rules: step 6 "tool in FILE_TOOLS or tool in (Bash, PowerShell) -> deny(DENY_MSG)". BASH_READ, BASH_WRITE, PS_FILE, _SEP removed.
- S3 §11 expected: all sonnet/opus Bash rows deny, including `go test ./...` (row 9), `bin/chaude charcount ...` (row 7), `echo x 2>/dev/null` (row 13), `git status` not in S3 table. S1 §12 had those as allow (rows 7, 9, 13, 14, 18, 21, 23).
- S1 §12 23-row run: "rows 1-23 run on the draft in /tmp/v082hook (2026-10-30): 23/23 match." Note: this ran the ORIGINAL regex draft, not the S3 full-deny rule. S3 has NOT been run.
- S3 §11 header says "expected after S3/S4".
- S3 DENY_MSG: "sonnet/opus/비-haiku 에이전트는 파일·셸 직접 접근 금지 (v0.8.2). haiku-scout(읽기)/haiku-batch-worker(쓰기)/haiku-coder(코드)에 위임하세요."
- S1 (original) deny messages (for contrast): "Sonnet/Opus may not read or write files via Bash (cat/head/tail/sed/grep/ls/wc/redirect/tee/cp/mv/rm...). Delegate to haiku-scout (read) or haiku-coder/haiku-drafter (write)."

## E. 기타 핵심 규칙 (S3 §0-§5, §8, §9)
### 0 goal
- every subagent whose agent_type is not haiku-* has no direct file access.
- blocked tools: Read, Write, Edit, MultiEdit, NotebookEdit, Grep, Glob, Bash, PowerShell.
### 1 decisions (S3)
- D1 scope: "block = agent_id set AND agent_type not startswith `haiku-` (allowlist, review H2)."
- D3 sonnet-analyst: tools = Agent only. WebSearch/WebFetch removed; web via haiku-researcher with RETURN verbatim <=4000 chars.
- D4 opus-architect: tools already Agent. text changes only.
- D5 lean mode: block applies even when lean_off() is true (확인 #3 권장안).
- D7 main thread (no agent_id): fail-open, exit 0. main-thread-guard.py handles it.
- D8 return channel: all eight haiku-* agent files get RETURN rule: scout, checker, drafter, batch-worker, runner, researcher, extractor, coder. S2 H1 listed only 5 (scout, checker, drafter, batch-worker, runner). S3 adds researcher, extractor, coder.
- D9 fail-closed (M1). D10 notes writes by sonnet/opus -> haiku-batch-worker ("save verbatim, return path + wc -l"); haiku-drafter only for user-voice prose (M4).
- D11 agent-model-pin (M3): subagent callers may target only haiku-*; check runs BEFORE lean_off() early exit.
- D12 Pyright getattr fix in ALL 8 hooks with `_s.reconfigure(`.
- D13 version stays 0.8.1 (M2 보류). CHANGELOG "harness-only change".
- D14 docs/claude-lean/CHANGELOG.md untouched (grep "0.8.1" = 0 hits, 미정 #2 rule). NOTE: S3 D14 says "grep '0.8.1' = 0 hits" but S3 §12 lists "grep 0.8.1 docs/claude-lean/CHANGELOG.md -> unchanged". not verified by me (no grep run).

### 3.2 hook decision order (S3 §3.2, the after-body)
1. getattr(_s, "reconfigure", None) in loop over stdin/stdout/stderr.
2. sys.dont_write_bytecode = True.
3. whole body in try/except (M1). exception path: raw stdin matches `"agent_id"` AND not `"agent_type"\s*:\s*"haiku-` -> exit 2 with DENY_MSG; else exit 0.
4. parse JSON. no agent_id -> exit 0.
5. agent_id set and agent_type empty -> deny.
6. agent_id set and agent_type not startswith `haiku-`: if env CLAUDE_HARNESS_ALLOW_SONNET_IO == "1" -> exit 0; tool in FILE_TOOLS or (Bash, PowerShell) -> deny; else exit 0 (e.g. Agent).
7. agent_type startswith haiku-: if lean_off() -> exit 0. Read with tin.allow_large truthy -> deny("allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first.").
8. else exit 0.
- FILE_TOOLS = ("Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Grep", "Glob")
- ALLOW_IO = ("haiku-",) (no ALLOW_IO_EXACT; see 확인 A)
- deny output: hookSpecificOutput permissionDecision deny, exit 0. exception path exit 2.
- S2 H2 deny message: same DENY_MSG as S3.
- S2 M1: "wrap the whole body, including `import _lean`, in try/except." S3 §3.2 try/except wraps body (note: S3 code block shows `from _lean import lean_off` at top-level outside try; S3 step 3 says "whole body inside try/except" — possible inconsistency; not verified).

### 4 agent-model-pin.py (S3 §4)
- new check placed BEFORE `if lean_off() or ...`. condition: agent_id set AND caller in {sonnet-reviewer, sonnet-analyst, opus-architect} AND subagent_type not startswith haiku- -> deny "sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출할 수 있습니다 (v0.8.2)."
- existing: haiku-/sonnet-/opus- model strip (haiku-* with model=sonnet|opus: model deleted, no change needed per S2 H2 check).

### 5 other files (S3 §5)
- .claude/hooks/{context-alert,correction-guard,main-thread-guard,ledger,session-context,style-adapter}.py: reconfigure line only. S1 §3.4: pyright warning observed at line 11 of haiku-context-guard.py. S2 L1: main-thread-guard.py line 8 also has it.
- .claude/settings.json line 70: before `"matcher": "Read|Bash",` after `"matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell",`. agent matcher `Agent|Task` unchanged.

### 7 agent text (S3 §7)
- 7.1 haiku-*.md (all 8) RETURN rule line: "If the brief contains `RETURN: verbatim <=N chars`, the final message is verbatim excerpts with path:line, up to N chars (max 6000); the 300-char cap does not apply."
- 7.2 sonnet-analyst.md: tools `Write, Bash, WebSearch, WebFetch, Agent` -> `Agent`. "Scope first: you have no Read/Write/Edit/Grep/Glob/Bash/WebSearch/WebFetch."
- 7.4 opus-architect.md: "No direct file access (v0.8.1):" -> "(v0.8.2):". tools unchanged.
- S2 H1 observed live: "opus-architect asked haiku-scout for verbatim plan text and got 300 chars back."

### 8 CLAUDE.md (S3 §8)
- line 10 after: "- sonnet/opus 에이전트 파일 직접 접근 전면 금지 (Read·Write·Edit·Grep·Glob 및 Bash/PowerShell 포함, 훅 강제): 파일 읽기는 haiku-scout(RETURN: verbatim <=4000 chars), 쓰기는 haiku-batch-worker/haiku-coder, 웹은 haiku-researcher가 수행."
- line 11: remove "may call sonnet-reviewer for review (the only non-Haiku agent a subagent may call)"; replace with "hand work only to haiku-* agents". (D11)
- line 12: remove haiku-drafter/sonnet-analyst as writers for notes/runs; state haiku-batch-worker (verbatim) and haiku-drafter (user-voice prose only).
- S1 §8.2 wording differs (line 10 after: "...sonnet-analyst는 유사 파일끼리 묶어 최대 3개 파일/묶음 단위로 haiku-scout에게..."). S1 original rule quoted: "- sonnet-analyst 파일 직접 접근 금지: 유사 파일끼리 묶어 최대 3개 파일/묶음 단위로 Haiku에게 정리 요청 후 요약만 받아 분석 (토큰 절약 목적)."

### 9 CHANGELOG / DECISIONS (S3 §9)
- CHANGELOG.md: insert above `## 0.8.1`. body (미정 #1 wording, S2): see B above.
- DECISIONS.md: append one line, "- [agents] v0.8.2 user: sonnet/opus/비-haiku 서브에이전트 파일·셸 전면 차단 (haiku-* 허용 목록); update to 35: sonnet-reviewer는 haiku-*만 호출 가능 (subagent caller); sonnet-reviewer/analyst/opus tools = Agent only."

### 10 execution (S3 §10, Haiku S0-S13, main gates G1, G2)
- S0 haiku-scout pre-state diff -> notes/runs/261030-verify-v082-block.md "## pre-state".
- G1 main session: live agent_type check (see 확인 F).
- S1 haiku-batch-worker: RETURN rule into 8 files. acceptance grep -l = 8 files.
- S2 haiku-scout acceptance call: brief "Read notes/runs/261030-plan-v082-write-block.md section 3.2. RETURN: verbatim <=4000 chars." acceptance >=1000 chars quoted.
- S3 haiku-coder: rewrite haiku-context-guard.py. acceptance grep BASH_READ|BASH_WRITE|PS_FILE = 0; `_s.reconfigure(` = 0; `getattr(_s` >= 1.
- S4 haiku-coder: agent-model-pin.py. acceptance: new check line index < lean_off line index.
- S5 haiku-batch-worker: reconfigure getattr in 6 hooks.
- S6 haiku-coder: settings.json line 70; then `go test ./internal/settings/...` and `go run ./cmd/chaude gen-settings windows`.
- S7 haiku-coder: sonnet-analyst.md, sonnet-reviewer.md, opus-architect.md. acceptance "grep -n ^tools: = tools: Agent only".
- S8 haiku-coder: CLAUDE.md lines 10-12.
- S9 haiku-batch-worker: DECISIONS.md one line.
- S10 haiku-coder: CHANGELOG.md insert.
- S11 haiku-scout: L5 grep (read only) for "sonnet-analyst", "sonnet-reviewer", "notes/runs" write routing in internal/, cmd/, README.md, docs/, .claude/hooks/main-thread-guard.py.
- S12 haiku-coder: fix S11 hits.
- S13 haiku-scout verification: sample rows, pin rows, pyright, self-check greps.
- G2 main session: final sonnet-reviewer review via haiku-scout excerpts. no open HIGH. no commit/tag/push/release.
- S3 §10 counts: "Haiku steps: 14 (S0-S13). main-session gates: 2 (G1, G2)." (note: S0-S13 is 14 steps, but S0 haiku-scout and S13 haiku-scout counted; OK.)

### 11 sample rows (S3 §11, 23 rows + pin P1-P5)
- row 16: statusline-setup Edit -> deny (literal rule, 확인 A).
- row 21: sonnet-analyst Read with CLAUDE_HARNESS_ALLOW_SONNET_IO=1 -> allow.
- row 22: sonnet-analyst Read with lean_off true (default) -> deny (D5).
- row 18: raw text has "agent_id", JSON broken -> deny, exit 2.
- row 19: no agent_id (main) Read -> allow, exit 0.
- row 20: agent_id set, agent_type empty Read -> deny.
- P4: haiku-scout subagent_type haiku-coder model opus -> allow, model removed.
- P5: main subagent_type sonnet-reviewer -> allow.
- S3 §11 "main + sonnet/opus forced model (M2): not tested here (보류)."
- S2 extra rows (review ordering step 7): Explore Read -> deny; statusline-setup Edit -> allow; haiku-scout Read -> allow; sonnet-reviewer Bash git diff -> deny; bad JSON with agent_id -> deny; main + CHAUDE_MAIN_MODEL=opus + Read -> deny; main + CHAUDE_MAIN_MODEL=haiku + Read -> pass through.
  - NOTE: S2 "statusline-setup Edit -> allow" conflicts with S3 row 16 "deny". S2 proposed ALLOW_IO_EXACT; S3 did not include it. Open question A.

### 12 self-check (S3 §12)
- pyright .claude/hooks/ -> 0 errors, 0 warnings.
- grep "_s.reconfigure(" -> 0 hits.
- grep "Read|Bash" .claude/settings.json -> 0 hits; new matcher -> 1 hit.
- grep "^tools:" sonnet-*.md opus-*.md -> `tools: Agent` only.
- grep -l "RETURN: verbatim" haiku-*.md -> 8 files.
- grep BASH_READ|BASH_WRITE|PS_FILE haiku-context-guard.py -> 0.
- grep "may call" CLAUDE.md -> no sonnet-reviewer call permission.
- grep 0.8.1 docs/claude-lean/CHANGELOG.md -> unchanged.

### 13 must NOT change (S3)
- .claude/hooks/_lean.py and lean_off() semantics.
- main-thread-guard.py behavior (only line 8 getattr).
- Haiku tool lists (haiku-researcher keeps WebSearch/WebFetch).
- allow_large deny for Haiku.
- permission allow entries; env block; agent matcher Agent|Task.
- Go sources and cmd/chaude version (0.8.1).
- no commit, tag, push, release, gh release.

## F. Key numbers / strings (quick ref)
- matcher before: "Read|Bash"; after: "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell" (9 tools).
- FILE_TOOLS: 7 names. Bash/PowerShell: 2 names. total blocked tools: 9.
- RETURN cap: <=N chars, max 6000; reviewer/analyst brief uses 4000.
- env vars: CLAUDE_HARNESS_ALLOW_SONNET_IO=1 (proposed escape, off by default, S3). CHAUDE_MAIN_MODEL (proposed M2, not in v0.8.2). CLAUDE_ALLOW_MAIN_TOOLS=1 (existing main-thread-guard bypass, S2 M2). 
- hook files: .claude/hooks/haiku-context-guard.py, agent-model-pin.py, main-thread-guard.py, _lean.py, context-alert.py, correction-guard.py, ledger.py, session-context.py, style-adapter.py.
- agent files: .claude/agents/haiku-{scout,checker,drafter,batch-worker,runner,researcher,extractor,coder}.md (8); sonnet-analyst.md, sonnet-reviewer.md, opus-architect.md.
- other files: CLAUDE.md lines 10-12, notes/DECISIONS.md, CHANGELOG.md (root, new 0.8.2 above 0.8.1), docs/claude-lean/CHANGELOG.md (untouched), cmd/chaude/main.go (version 0.8.1 stays), internal/settings/ (go test target), notes/runs/261030-verify-v082-block.md (verify file).
- tui.go:703 (usageFetch); classifier.go:188 (classifier claude -p --model); budget.go (USD); repl.go (ctxTokens, effectiveThreshold, setRate); core.go (observedTTL, ctxTokens); mode.go; drain.go; agentstore.go (internal/harness/agentstore.go).
- numbers: S1 sample 23 rows, 23/23 match (on original regex draft in /tmp/v082hook; final not run). S3 unresolved: 미정 2, 사용자 확인 필요 6 (A-F), 보류 1 (M2).
- S3 §6 status table: 22 rows; row 22 (/usage) only row needing a model call (possibly). rows 1-21 no model call.

## G. doubts / inconsistencies found while reading
1. brief referenced "final §6 확인 A-F" but §6 is Go status items; 확인 A-F is §14 (L254-260).
2. S3 open_count line says "사용자 확인 필요 6 (section 10)"; the list is in §14. stale cross-ref.
3. S3 L7 says "original 미정 4 closed by review recommendation (section 9)". closure list is in §14 L252.
4. S2 H3 says keep regex for Haiku allow_large; S3 D6 removes regex. S3 follows D6.
5. S2 H2 ALLOW_IO_EXACT {"statusline-setup"}: S2 row says allow; S3 row 16 says deny (확인 A).
6. S3 §11 vs S1 §12: Bash allows (rows 7, 9, 13, 14, 18, 21, 23 in S1) became deny in S3. S1 23/23 run result does not validate S3 rules.
7. S1 §12 row 7 `bin/chaude charcount` allowed (tools list blocks Bash) vs S2 M5 which moves charcount to haiku-checker. S3 consistent with M5.
8. S3 D2 reviewer issue list via haiku-batch-worker; S1 D2 used haiku-drafter; S2 M4 says batch-worker. S3 follows S2.
9. S1 §6 used haiku-runner for charcount; S2 M5 & S3 use haiku-checker.
10. RETURN rule file count: S2 H1 = 5 files; S3 D8 = 8 files. S3 adds researcher, extractor, coder.
11. S3 §3.2 code block: `from _lean import lean_off` appears outside try/except, while S3 step 3 says whole body in try/except. S2 M1 says include `import _lean`. Not resolved in S3 code.
12. S3 §10 S0 is haiku-scout pre-state; S1 §11 step 1 was haiku-coder. S3 is the newer order.
13. S3 D14 check "grep 0.8.1 = 0 hits" is a claim; not verified by me.
14. Unverified: live agent_type payload (G1) — both S1 and S2 flag it; S3 not yet run.
15. Pyright warning text not captured (S1 §3.4, 확인 D). S1 reproduced: 0 errors 0 warnings on current file (run 2026-10-30).
16. S3 scope: sonnet/opus Agent tool calls are allowed (step 6 else exit 0).
17. Hook interplay: S3 haiku-context-guard matcher includes PowerShell; S1 note the PowerShell tool's existence not verified in this repo.

## H. what was NOT found / not done
- no source text found for S3 미정 1 and 2 (no docs or search performed; reading only, as briefed).
- did not run pyright, grep, go test, or the hook. no verification of claims in the plans.
- did not grep repo for the 0.8.1 string in docs/claude-lean/CHANGELOG.md.
- no chat-history search performed (not requested).
