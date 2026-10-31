# 261029 explore: haiku-context-guard.py + sonnet-analyst.md + sonnet-reviewer.md
task: read 3 files verbatim, report line 11 Pyright risk, guard decision logic, blocked tools now (read-only)
sources: .claude/hooks/haiku-context-guard.py (41 lines), .claude/agents/sonnet-analyst.md (24 lines), .claude/agents/sonnet-reviewer.md (16 lines)
status: DONE (read-only, no source edits)

## 1. haiku-context-guard.py (full text, verbatim)
```python
#!/usr/bin/env python3
"""PreToolUse hook (matcher: Read|Bash): Haiku subagents may not use Read's allow_large flag; sonnet-analyst may not Read/Grep/Glob directly (delegate to haiku-scout); sonnet-/opus- may not read or write files via Bash (delegate to haiku-scout / haiku-coder).

Claude Code itself caps each Read at env CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS (8000; observed on a
Read: "cap 8000"), so no size estimate is needed here. allow_large is the only way around that cap, and
it is denied for Haiku so it reads in slices (offset/limit) or greps first. Agents compact on their own
(autoCompactWindow 100000). No-op in normal mode."""
import json, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

BASH_READ = re.compile(r'(?:^|[;&\n(]|\|\||&&)\s*(?:\w+=\S*\s+)*(?:cat|head|tail|sed|awk|gawk|grep|egrep|fgrep|rg|less|more|nl|od|xxd|strings)\b')
BASH_WRITE = re.compile(r'\btee\b|(?<![>&])>>?(?!&)\s*(?!/dev/null\b)\S')

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
kind = data.get("agent_type") or ""
if not lean_off() and data.get("agent_id") and kind == "sonnet-analyst" and data.get("tool_name") in ("Read", "Grep", "Glob"):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": "sonnet-analyst may not read files directly; delegate to haiku-scout."}}))
    sys.exit(0)
if not lean_off() and data.get("agent_id") and kind.startswith(("sonnet-", "opus-")) and data.get("tool_name") == "Bash":
    cmd = (data.get("tool_input") or {}).get("command") or ""
    if BASH_READ.search(cmd) or BASH_WRITE.search(cmd):
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                          "permissionDecisionReason": "Sonnet/Opus may not read or write files via Bash (cat/head/tail/sed/awk/grep/redirect/tee). Delegate reads to haiku-scout and writes/edits to haiku-coder or haiku-batch-worker."}}))
        sys.exit(0)
if lean_off() or not data.get("agent_id") or kind.startswith(("sonnet-", "opus-")) or kind in ("analyst", "reviewer"):
    sys.exit(0)
if data.get("tool_name") == "Read" and (data.get("tool_input") or {}).get("allow_large"):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": "allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first."}}))
sys.exit(0)
```

## 2. sonnet-analyst.md (full text, verbatim)
```markdown
---
name: sonnet-analyst
description: Sonnet (분석 담당). Complexity 1-2 work and implementing opus-architect plans. Reasoning the main thread hands off - long explanations, plans, comparisons, option analysis from given facts; synthesizing many sources; interpreting data or analysis results; multi-file/design-level code; tasks Haiku failed twice.
tools: Write, Bash, WebSearch, WebFetch, Agent
model: sonnet
effort: medium
omitClaudeMd: true
autoCompactWindow: 100000
---

Questions handed off by the main thread: the brief quotes the user's words and the needed context. Write the answer for the user (Korean 존댓말) to `notes/runs/<YYMMDD>-answer-<topic>.md`; return only the path + a 1-line gist.
Read failure reports in the brief first; never repeat tried approaches. Documents: read FACTS.md / DECISIONS.md; never invent facts.
Separate evidence from inference; mark uncertainty. Code: smallest complete change, narrow verification, exact results.
Full write-up → `notes/runs/<YYMMDD>-analyst-<topic>.md`.
Otherwise return ≤5 lines: conclusion · confidence · path · open question.
Final message = only a brief result summary (a few lines, Korean 존댓말: outcome · changed paths · PASS/FAIL) for the main session to post; full details go to `notes/runs/`, give only the path.
Your final message stays in the main thread's context: never paste file contents, lists or long explanations — put them in files.
Scope first: you have no Read/Grep/Glob/Edit. Bundle at most 3 files and have `haiku-scout` read them (bring back a summary); all file edits go to `haiku-coder`/`haiku-batch-worker`. Stop without implementing and return `ESCALATE: notes/runs/<YYMMDD>-scope-<topic>.md` (what you found, the open decision, the options) when: two or more plausible designs and no evidence for one; requirements conflict; 5+ interlocked files must change; or your own check fails once. Escalating early is cheaper than a weak result. Over ~20 responses: stop at a milestone and report instead of continuing; spawn nothing for the same files you already read.
Mechanical sub-steps (bulk edits, conversions, wide searches): with the Agent tool, hand them to `haiku-batch-worker`/`haiku-scout`/`haiku-coder` (≤5-line briefs) and check results; else list them under `## [haiku] steps` in your result file (agent, files, exact change) and say so.
- never edit files yourself; hand every edit to haiku-coder with an exact spec, then verify its diff via haiku-checker or Bash git diff.
Agent-read files (notes/*, runs/): terse English `key: value`, Korean values verbatim; user-read text: Korean. Reply plainly: done, key numbers (what each counts), doubts, path.

Plan review (from the Haiku main): when the brief gives a plan path to review, have haiku-scout read it, then write only concrete improvements (missing steps, wrong agent choice, risks, ordering) to the path named in the brief (default `<plan>-review.md`); do not execute the plan. Return ≤3 lines: verdict, path.
```

## 3. sonnet-reviewer.md (full text, verbatim)
```markdown
---
name: sonnet-reviewer
description: Sonnet (검토 담당). Read-only review before anything leaves the project - submitted, published, printed, sent, merged or released (documents, reports, slides, code diffs). Replaces the main thread reading full text.
tools: Read, Grep, Glob, Bash
model: sonnet
effort: medium
omitClaudeMd: true
autoCompactWindow: 100000
---

Do not edit. Documents: facts vs FACTS.md, tone/versions/format vs DECISIONS.md, the requirements named in the brief, length limits (`bin/chaude charcount`), clarity for the intended reader, consistency across versions. Code: bugs, edge cases, unrelated changes.
Issue list (severity, file:line, one-line fix) → `notes/runs/<YYMMDD>-review-<topic>.md`.
Return ≤5 lines: PASS / FIX · top 3 issues · path.
Final message = only a brief result summary (a few lines, Korean 존댓말: outcome · changed paths · PASS/FAIL) for the main session to post; full details go to `notes/runs/`, give only the path.
Agent-read files (notes/*, runs/): terse English `key: value`, Korean values verbatim; user-read text: Korean. Reply plainly: done, key numbers (what each counts), doubts, path.
```

## 4. Line 11 of guard
code: `        _s.reconfigure(encoding="utf-8")` (inside `for _s in (sys.stdin, sys.stdout, sys.stderr):` try block; indent 8 spaces in file)
likely Pyright: reportAttributeAccessIssue, "Cannot access attribute \"reconfigure\" for class \"TextIO\"" (typeshed types sys.stdin/stdout/stderr as TextIO; TextIO has no reconfigure). Runtime-safe on CPython (TextIOWrapper has it; try/except covers the rest). NOT run through Pyright; inference from typeshed, unverified.
other possible static noise: line 15 `from _lean import lean_off` (reportMissingImports if Pyright root not .claude/hooks; _lean.py exists there, 2145 bytes).

## 5. Decision logic (guard)
- settings.json: matcher "Read|Bash" (hook command `python3 "$CLAUDE_PROJECT_DIR/.claude/hooks/haiku-context-guard.py"`, timeout 5). So Grep/Glob events never reach the hook.
- stdin JSON parse failure -> exit 0.
- kind = agent_type or "".
- no agent_id (main thread) or lean_off() -> the new Bash and analyst checks are skipped; final branch exits 0 (lean_off or no agent_id).
- A. kind == "sonnet-analyst" and tool in Read/Grep/Glob -> DENY (Grep/Glob branch unreachable given matcher).
- B. kind startswith sonnet-/opus- and tool == Bash and command matches BASH_READ (cat|head|tail|sed|awk|gawk|grep|egrep|fgrep|rg|less|more|nl|od|xxd|strings at command start or after ; & \n ( || &&) or BASH_WRITE (tee, or > / >> redirect not to /dev/null, not >&) -> DENY.
- C. kind startswith sonnet-/opus- or kind in (analyst, reviewer) -> exit 0 (allowed).
- D. other agents with agent_id (haiku-*, etc.): Read with allow_large truthy -> DENY.
- otherwise allow.

## 6. Blocked now (summary)
- sonnet-analyst: Read/Grep/Glob (Read only effective, matcher). Bash read/write patterns.
- sonnet-* / opus-*: Bash read/write patterns only.
- haiku-* subagents: Read with allow_large only.
- not blocked: main thread; sonnet-reviewer Read/Grep/Glob (guard does not restrict it); Write/Edit tools (not in matcher); Bash not matching the regexes; everything when lean_off().

## 7. Doubts / not found
- sonnet-analyst frontmatter tools (Write, Bash, WebSearch, WebFetch, Agent) and its body ("no Read/Grep/Glob/Edit") agree; line-25 Grep/Glob branch is dead under matcher Read|Bash.
- sonnet-reviewer frontmatter has Bash; its body says "Do not edit" but guard only blocks read/write-looking Bash, not Write/Edit tools (not in matcher). Its Bash may be blocked by BASH_READ for reads like grep/cat.
- git diff shows guard is uncommitted: docstring matcher changed Read -> Read|Bash, added BASH_READ/BASH_WRITE and sonnet-/opus- Bash rule (committed HEAD version had only analyst Read/Grep/Glob and Haiku allow_large rules).
- _lean.py not read (lean_off definition unknown).
- Pyright not run.
