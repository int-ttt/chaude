# plan: v0.8.2 write-block (option b) - implementation plan
date: 2026-10-30
status: plan only. no code edited, no commit, tag, push, or release.
author: opus-architect scope (main thread brief)
source: notes/runs/261029-scope-v082-write-block.md, notes/runs/261029-scope-v082-impl.md (ESCALATE)
user_decision (verbatim): "b 방식 채택. 모든 sonnet/opus 모델은 읽기/쓰기 전면 차단, bash를 통한 읽기/쓰기도 차단. 반드시 haiku를 경유하도록 강제 구현."
unknown_items_count: 미정 4 (no source text) / 사용자 확인 필요 3 (see section 10)
checked_by: draft hook run on 23 sample inputs in /tmp (not repo): 23/23 match expected.

## 0. goal
- rule: every sonnet-* and opus-* subagent has no direct file access.
- read/write (Read, Write, Edit, MultiEdit, NotebookEdit, Grep, Glob) denied by hook.
- read/write via Bash and PowerShell denied by hook (regex).
- every file touch goes through haiku-* agents.
- enforcement: PreToolUse hook (code), agent tools: lists (declared), CLAUDE.md (rule text).

## 1. decisions
- D1 scope: b = all sonnet-*/opus-* blocked, Bash/PowerShell read+write blocked too. source: user decision above.
- D2 sonnet-reviewer: tools = Agent only; reads via haiku-scout; issue list via haiku-drafter. source: 261029-scope-v082-write-block.md recommendation B. not confirmed by user: see 사용자 확인 필요 #2.
- D3 sonnet-analyst: tools = Agent only (WebSearch/WebFetch 미정, see 미정 #4).
- D4 opus-architect: tools already Agent. only text changes.
- D5 lean mode: sonnet/opus block applies even when lean_off() is true (default). not confirmed: see 사용자 확인 필요 #3.
- D6 Haiku rules unchanged: allow_large deny kept; analyst/reviewer exemption kept (dead code after change, see section 3).
- D7 main thread (no agent_id): hook exits 0. main-thread-guard.py handles main thread.

## 2. file list (order = execution order, section 11)
1. .claude/hooks/haiku-context-guard.py  (rewrite body; docstring; Pyright fix)
2. .claude/settings.json                 (line 70 matcher only)
3. .claude/agents/sonnet-analyst.md
4. .claude/agents/sonnet-reviewer.md
5. .claude/agents/opus-architect.md
6. CLAUDE.md                              (line 10, line 12)
7. CHANGELOG.md                           (new 0.8.2 section, wording 미정)
not changed: .claude/hooks/_lean.py, .claude/hooks/main-thread-guard.py, haiku-*.md, cmd/chaude/main.go (version bump 미정 #3), docs/claude-lean/CHANGELOG.md (미정 #2).

## 3. file 1: .claude/hooks/haiku-context-guard.py
### 3.1 before (summary)
- docstring line 2: "PreToolUse hook (matcher: Read|Bash): ..."
- lines 9-13: `for _s in (sys.stdin, sys.stdout, sys.stderr): try: _s.reconfigure(encoding="utf-8") except Exception: pass`
- BASH_READ = cat|head|tail|sed|awk|gawk|grep|egrep|fgrep|rg|less|more|nl|od|xxd|strings only; separator `(?:^|[;&\n(]|\|\||&&)`.
- BASH_WRITE = `\btee\b|(?<![>&])>>?(?!&)\s*(?!/dev/null\b)\S`.
- sonnet-analyst Read/Grep/Glob branch (lines 25-28).
- sonnet/opus Bash branch (lines 29-34).
- line 35: exit 0 for lean_off, no agent_id, sonnet/opus, analyst/reviewer.
- lines 37-39: allow_large deny for Haiku.

### 3.2 after (full body, replaces the file)
```python
#!/usr/bin/env python3
"""PreToolUse hook (matcher: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell).
sonnet-/opus- subagents: no file tools (Read/Write/Edit/MultiEdit/NotebookEdit/Grep/Glob) and no read/write via Bash/PowerShell; always enforced, lean_off() does not apply. Delegate to haiku-* agents.
Haiku subagents: may not use Read's allow_large flag (Claude Code caps each Read; read in offset/limit slices).
Main thread (no agent_id): not handled here (main-thread-guard.py)."""
import json, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    _reconf = getattr(_s, "reconfigure", None)
    try:
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

_SEP = r'(?:^|[;&|\n(`])\s*(?:\w+=\S*\s+)*'
BASH_READ = re.compile(
    _SEP + r'(?:cat|head|tail|sed|awk|gawk|grep|egrep|fgrep|rg|less|more|nl|od|xxd|strings|wc|sort|uniq|cut|find|ls|diff|cmp|jq|file|stat|tac|paste|column)\b'
    r'|(?:^|[;&|\n(`])\s*git\s+(?:show|diff|cat-file|grep)\b'
    r'|\b(?:python3?|node|perl|ruby)\s+-[ce]\b')
BASH_WRITE = re.compile(
    r'\btee\b|(?<![>&])>>?(?!&)\s*(?!/dev/null\b)\S|&>>?\s*(?!/dev/null\b)\S'
    r'|' + _SEP + r'(?:cp|mv|rm|touch|mkdir|rmdir|ln|chmod|chown|dd|install|truncate|patch)\b')
PS_FILE = re.compile(
    r'\b(?:Get-Content|gc|type|cat|Set-Content|Add-Content|Out-File|Copy-Item|Move-Item|Remove-Item|New-Item|Select-String|Get-ChildItem|dir|sc|ac|ls)\b', re.I)
FILE_TOOLS = ("Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Grep", "Glob")

def deny(reason):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": reason}}))
    sys.exit(0)

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
kind = data.get("agent_type") or ""
tool = data.get("tool_name") or ""
tin = data.get("tool_input") or {}
cmd = tin.get("command") or ""

if data.get("agent_id") and kind.startswith(("sonnet-", "opus-")):
    if tool in FILE_TOOLS:
        deny(f"{kind} may not use {tool}: no direct file access. Delegate reads to haiku-scout and writes to haiku-coder, haiku-batch-worker or haiku-drafter.")
    if tool == "Bash" and (BASH_READ.search(cmd) or BASH_WRITE.search(cmd)):
        deny("Sonnet/Opus may not read or write files via Bash (cat/head/tail/sed/grep/ls/wc/redirect/tee/cp/mv/rm...). Delegate to haiku-scout (read) or haiku-coder/haiku-drafter (write).")
    if tool == "PowerShell" and (PS_FILE.search(cmd) or BASH_READ.search(cmd) or BASH_WRITE.search(cmd)):
        deny("Sonnet/Opus may not read or write files via PowerShell. Delegate to haiku-scout (read) or haiku-coder/haiku-drafter (write).")
    sys.exit(0)

if lean_off() or not data.get("agent_id") or kind in ("analyst", "reviewer"):
    sys.exit(0)
if tool == "Read" and tin.get("allow_large"):
    deny("allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first.")
sys.exit(0)
```
note: the `kind in ("analyst", "reviewer")` exemption is kept as-is (summary decision). it has no effect on sonnet/opus because that branch exits earlier.

### 3.3 decision logic (order)
1. stdin not JSON -> exit 0.
2. no agent_id (main thread) -> not sonnet/opus; falls to step 5 -> exit 0.
3. agent_id and agent_type starts with `sonnet-` or `opus-`:
   a. tool in FILE_TOOLS -> deny.
   b. tool Bash and command matches BASH_READ or BASH_WRITE -> deny.
   c. tool PowerShell and command matches PS_FILE, BASH_READ or BASH_WRITE -> deny.
   d. else exit 0 (e.g. Agent tool, `go test`, `git status`).
4. lean_off() true, or Haiku/other -> exit 0.
5. Haiku: Read with allow_large truthy -> deny.
6. else exit 0.

### 3.4 Pyright line 11 (cause and fix)
- observed: Pyright warning on line 11 (`_s.reconfigure(encoding="utf-8")`) reported by user. user-supplied warning text: 사용자 확인 필요 #1.
- reproduced: `pyright .claude/hooks/haiku-context-guard.py` -> 0 errors, 0 warnings (run 2026-10-30, current file).
- inferred cause: sys.stdin/stdout/stderr typed as TextIO; typeshed TextIO has no `reconfigure` (only TextIOWrapper). Attribute access flagged.
- fix: `getattr(_s, "reconfigure", None)` then call only if present. runtime behavior same (try/except kept).
- verify: `pyright .claude/hooks/haiku-context-guard.py` before and after; after must show 0 errors for this line. record both outputs in verify file.
- python: stdlib only; target >=3.8 (local runtime 3.9.6).

### 3.5 regex coverage (confirmed gaps in current file)
- current misses: single `|` pipe, backticks, `&>`, cp/mv/rm/touch, PowerShell cmdlets, `wc/sort/find/ls/diff/jq` etc.
- new: `&>>?` separate pattern, `_SEP` includes `|` and backtick.
- known limits (not fixed; document in CHANGELOG? 미정 #1): command substitution inside quotes, `eval`, `xargs cat`, `bash -c "cat x"`, `python3 -c` with other flags (`-` forms), heredoc redirect. Regex cannot catch all; tools list is the hard boundary.

## 4. file 2: .claude/settings.json
### 4.1 before (line 70)
```
        "matcher": "Read|Bash",
```
### 4.2 after
```
        "matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell",
```
### 4.3 unchanged
- all other hook entries (Agent|Task agent-model-pin, main-thread-guard matcher line 60, ledger, session-context, history-export, correction-guard, style-adapter, context-alert).
- permissions block (`Bash(bin/chaude search-history:*)`).
- env block.

## 5. file 3: .claude/agents/sonnet-analyst.md
| line | before | after |
|---|---|---|
| 4 | `tools: Write, Bash, WebSearch, WebFetch, Agent` | `tools: Agent` (WebSearch/WebFetch: 미정 #4) |
| ~10 | `Write the answer for the user (Korean 존댓말) to \`notes/runs/<YYMMDD>-answer-<topic>.md\`` | `Have \`haiku-drafter\` write the answer for the user (Korean 존댓말) to \`notes/runs/<YYMMDD>-answer-<topic>.md\`` |
| ~11 | `Full write-up → \`notes/runs/<YYMMDD>-analyst-<topic>.md\`.` | `Full write-up → \`notes/runs/<YYMMDD>-analyst-<topic>.md\` via \`haiku-drafter\`.` |
| ~14 | `Scope first: you have no Read/Grep/Glob/Edit.` | `Scope first: you have no Read/Write/Edit/Grep/Glob/Bash.` |
| ~14 | `all file edits go to haiku-coder/haiku-batch-worker` | unchanged |
| ~22 | `Plan review ... write only concrete improvements ... to the path named in the brief` | same text + `(write via haiku-drafter)` |
note: exact before text for the `~` lines is from the scope summary; re-grep before editing (line numbers may shift by 1).

## 6. file 4: .claude/agents/sonnet-reviewer.md
| line | before | after |
|---|---|---|
| 4 | `tools: Read, Grep, Glob, Bash` | `tools: Agent` |
| ~9 | (no direct-read sentence) | add: `Read nothing directly: haiku-scout reads and returns excerpts.` |
| ~11 | `Issue list (severity, file:line, one-line fix) → \`notes/runs/<YYMMDD>-review-<topic>.md\`.` | same, with `written via haiku-drafter` |
| ~ (length check) | `bin/chaude charcount` | run via `haiku-runner`: `Length limits: haiku-runner runs \`bin/chaude charcount\`.` |
note: haiku-runner exists (.claude/agents/haiku-runner.md, tools Read, Write, Edit, Bash, Grep, Glob).

## 7. file 5: .claude/agents/opus-architect.md
| line | before | after |
|---|---|---|
| 4 | `tools: Agent` | unchanged |
| 11 | `No direct file access (v0.8.1):` | `No direct file access (v0.8.2):` |
| 11 | `reading/search -> \`haiku-scout\` (or \`sonnet-analyst\` for synthesis)` | `reading/search -> \`haiku-scout\`` |
| 11 | `plans/answers written to \`notes/runs/\` -> \`haiku-drafter\`/\`sonnet-analyst\`` | `plans/answers written to \`notes/runs/\` -> \`haiku-drafter\`` |
| 17 (plan step) | `write \`notes/runs/<YYMMDD>-plan-<topic>.md\`` | `have \`haiku-drafter\` write \`notes/runs/<YYMMDD>-plan-<topic>.md\`` |
| 23 (plan review) | `read it and write only concrete improvements` | `have \`haiku-scout\` read it and \`haiku-drafter\` write only concrete improvements` |
note: opus-architect tool list does not change (Agent only).

## 8. file 6: CLAUDE.md
### 8.1 line 10 before (only copy of the sonnet-analyst rule)
`- sonnet-analyst 파일 직접 접근 금지: 유사 파일끼리 묶어 최대 3개 파일/묶음 단위로 Haiku에게 정리 요청 후 요약만 받아 분석 (토큰 절약 목적).`
### 8.2 line 10 after
`- sonnet/opus 에이전트 파일 직접 접근 전면 금지 (Read·Write·Edit·Grep·Glob 및 Bash/PowerShell을 통한 읽기·쓰기 포함, 훅 강제): sonnet-analyst는 유사 파일끼리 묶어 최대 3개 파일/묶음 단위로 haiku-scout에게 정리 요청 후 요약만 받아 분석 (토큰 절약 목적). 파일 읽기·쓰기는 Haiku 에이전트만 수행.`
### 8.3 line 12 before
`... notes/runs documents → \`haiku-drafter\`/\`sonnet-analyst\`).`
### 8.4 line 12 after
`... notes/runs documents → \`haiku-drafter\`).`
(exact before text of line 12 is the substring shown; re-grep before editing.)

## 9. file 7: CHANGELOG.md
- top of file is `## 0.8.1`. insert new section above it.
- heading: `## 0.8.2`
- body wording: 미정 #1 (no source text).
- must mention (content, not wording): hook matcher widened; sonnet/opus file access blocked incl. Bash/PowerShell; sonnet-analyst/sonnet-reviewer/opus-architect tools changed; Pyright reconfigure fix.
- docs/claude-lean/CHANGELOG.md: not touched unless 미정 #2 says so.
- cmd/chaude/main.go `var version = "0.8.1"`: not touched unless 미정 #3 says so.

## 10. unknowns
- 미정 #1: CHANGELOG 0.8.2 wording (no source text).
- 미정 #2: docs/claude-lean/CHANGELOG.md update yes/no (no source text).
- 미정 #3: version bump in cmd/chaude/main.go to 0.8.2 yes/no (no source text).
- 미정 #4: sonnet-analyst WebSearch/WebFetch: keep or remove (no source text).
- 사용자 확인 필요 #1: exact Pyright warning text the user saw (only the line 11 location is known; no output captured).
- 사용자 확인 필요 #2: reviewer option B (Agent only) vs A (keep Read/Grep/Glob, write/Bash-write blocked) vs C (delete reviewer). plan uses B.
- 사용자 확인 필요 #3: lean mode. plan blocks sonnet/opus even when lean_off() is true. confirm.
- 미확인: live PreToolUse payload carries agent_type for these agents (v0.8.1 code already relies on it). verify in section 11 step 3.

## 11. execution order
1. haiku-coder: apply section 3 (hook) and section 4 (settings.json matcher). do not touch other files.
2. verify hook:
   a. `python3 -m py_compile .claude/hooks/haiku-context-guard.py` (then delete any __pycache__ it created).
   b. `pyright .claude/hooks/haiku-context-guard.py` -> 0 errors for line 11 (record before/after).
   c. run the 23 sample inputs (section 12) through the hook with stdin JSON; expect every row to match.
   d. live check: start one sonnet-analyst child with a Read call; confirm deny text appears and agent_type is set. (미확인 item.)
   record results to `notes/runs/261030-verify-v082-block.md`.
3. agent tools: haiku-coder applies sections 5, 6, 7 (sonnet-analyst, sonnet-reviewer, opus-architect). WebSearch/WebFetch per 미정 #4.
4. CLAUDE.md: haiku-coder applies section 8 (line 10 and line 12).
5. CHANGELOG: haiku-coder adds section 9 using 미정 #1 wording.
6. checks: haiku-checker greps (section 12 grep list). `go build ./...` / `go test ./...` only if the Go version (미정 #3) changes; otherwise not needed.
7. sonnet-reviewer review of the diff (per D2, reviewer reads via haiku-scout).
8. stop. no commit, tag, push, or release (user decides).

## 12. sample inputs and expected result
method: JSON on stdin, `agent_id` set unless noted. expected = hook output (deny or allow).
| # | agent_type | tool_name | tool_input | expected |
|---|---|---|---|---|
| 1 | sonnet-analyst | Read | file_path notes/FACTS.md | deny |
| 2 | sonnet-reviewer | Grep | pattern x | deny |
| 3 | opus-architect | Write | file_path notes/a.md | deny |
| 4 | sonnet-analyst | Edit | file_path a | deny |
| 5 | opus-architect | Glob | pattern * | deny |
| 6 | sonnet-reviewer | Bash | `cat notes/FACTS.md` | deny |
| 7 | sonnet-reviewer | Bash | `bin/chaude charcount notes/x.md --limit 10` | allow (hook; tools list already blocks Bash) |
| 8 | opus-architect | Bash | `echo hi > notes/a.md` | deny |
| 9 | sonnet-analyst | Bash | `go test ./...` | allow |
| 10 | sonnet-analyst | Agent | prompt x | allow |
| 11 | haiku-scout | Read | file_path a, allow_large true | deny |
| 12 | haiku-scout | Read | file_path a | allow |
| 13 | sonnet-analyst | Bash | `echo x 2>/dev/null` | allow |
| 14 | sonnet-analyst | Bash | `go build 2>&1` | allow |
| 15 | sonnet-analyst | Bash | `make &> out.log` | deny |
| 16 | sonnet-analyst | Bash | `echo a \| tee x` | deny |
| 17 | sonnet-analyst | Bash | `python3 -c "open('x').read()"` | deny |
| 18 | sonnet-analyst | Bash | `python3 scripts/x.py` | allow |
| 19 | sonnet-analyst | Bash | `echo $(cat x)` | deny |
| 20 | sonnet-analyst | Bash | `git show HEAD:x` | deny |
| 21 | sonnet-analyst | Bash | `git status` | allow |
| 22 | sonnet-analyst | PowerShell | `Get-Content x` | deny |
| 23 | sonnet-analyst | PowerShell | `go test ./...` | allow |
| 24 | (no agent_id, main thread) | Read | file_path notes/FACTS.md | allow (hook; main-thread-guard decides). not in the 23-run; expected from code path, not run |
result: rows 1-23 run on the draft in /tmp/v082hook (2026-10-30): 23/23 match.

grep self-check after apply (haiku-checker):
- `grep -n "matcher" .claude/settings.json` shows the new matcher on the haiku-context-guard entry and no `"Read|Bash"`.
- `grep -n "Read|Bash" .claude/settings.json` -> no match.
- `grep -n "tools:" .claude/agents/sonnet-*.md .claude/agents/opus-*.md` -> only `Agent` (plus 미정 #4 items).
- `grep -n "sonnet-analyst" CLAUDE.md` -> no delegate-to-sonnet-analyst for notes writes.
- `grep -n "v0.8.1" .claude/agents/opus-architect.md` -> no match.
- `grep -n "reconfigure" .claude/hooks/haiku-context-guard.py` -> getattr form only.

## 13. must NOT change
- lean mode files, main-thread-guard.py, agent-model-pin, ledger, session-context.
- Haiku agent tool lists.
- allow_large deny for Haiku.
- permission allow entries.
- no commit, tag, push, release, or gh release.
