# explore: settings.json hooks + haiku-context-guard + main-thread-guard
date: 2026-10-29
source files:
  /Users/leekwangseok/PycharmProjects/claude-harness/.claude/settings.json
  /Users/leekwangseok/PycharmProjects/claude-harness/.claude/hooks/haiku-context-guard.py
  /Users/leekwangseok/PycharmProjects/claude-harness/.claude/hooks/main-thread-guard.py
not read: .claude/hooks/_lean.py (imported by both guards for lean_off(); semantics unverified here)

## 1. settings.json "hooks" section (verbatim, lines 18-125)
```json
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/correction-guard.py\"",
            "timeout": 5
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/style-adapter.py\"",
            "timeout": 5
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/context-alert.py\"",
            "timeout": 5
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Agent|Task",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/agent-model-pin.py\"",
            "timeout": 5
          }
        ]
      },
      {
        "matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"",
            "timeout": 5
          }
        ]
      },
      {
        "matcher": "Read",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/haiku-context-guard.py\"",
            "timeout": 5
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/ledger.py\"",
            "timeout": 5
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "matcher": "startup|resume|compact|clear",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/session-context.py\"",
            "timeout": 5
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/history-export.py\"",
            "timeout": 60
          }
        ]
      }
    ],
    "PreCompact": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/history-export.py\"",
            "timeout": 60
          }
        ]
      }
    ]
  }
```

## 2. .claude/hooks/haiku-context-guard.py (verbatim)
```python
#!/usr/bin/env python3
"""PreToolUse hook (matcher: Read): Haiku subagents may not use Read's allow_large flag; sonnet-analyst may not Read/Grep/Glob directly (delegate to haiku-scout).

Claude Code itself caps each Read at env CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS (8000; observed on a
Read: "cap 8000"), so no size estimate is needed here. allow_large is the only way around that cap, and
it is denied for Haiku so it reads in slices (offset/limit) or greps first. Agents compact on their own
(autoCompactWindow 100000). No-op in normal mode."""
import json, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
kind = data.get("agent_type") or ""
if not lean_off() and data.get("agent_id") and kind == "sonnet-analyst" and data.get("tool_name") in ("Read", "Grep", "Glob"):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": "sonnet-analyst may not read files directly; delegate to haiku-scout."}}))
    sys.exit(0)
if lean_off() or not data.get("agent_id") or kind.startswith(("sonnet-", "opus-")) or kind in ("analyst", "reviewer"):
    sys.exit(0)
if data.get("tool_name") == "Read" and (data.get("tool_input") or {}).get("allow_large"):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": "allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first."}}))
sys.exit(0)
```

## 3. .claude/hooks/main-thread-guard.py (verbatim)
```python
#!/usr/bin/env python3
"""PreToolUse hook: the main (Opus) conversation only listens, decides, and delegates.
Work tools are blocked unless the call comes from a subagent (agent_id present).
Off in normal mode (type 일반모드, or start with CLAUDE_LEAN=0); CLAUDE_ALLOW_MAIN_TOOLS=1 still works."""
import json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

WORK_TOOLS = {"Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Bash", "PowerShell", "Grep", "Glob", "WebSearch", "WebFetch"}

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
if lean_off() or os.environ.get("CLAUDE_ALLOW_MAIN_TOOLS") == "1" or data.get("agent_id"):
    sys.exit(0)
tool = data.get("tool_name", "")
if tool in WORK_TOOLS:
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": f"Main session does not use {tool}. Delegate: search->haiku-scout, web->haiku-researcher/`/research`, writing->haiku-drafter, verify->haiku-checker/`/check`, bulk/notes->haiku-batch-worker, jobs->haiku-runner/`/status`, code->haiku-coder, reasoning/synthesis->sonnet-analyst (ESCALATE -> opus-architect), final review->sonnet-reviewer. If the user wants you to work directly, tell them to type 일반모드."
    }}))
sys.exit(0)
```

## 4. How each hook identifies the agent (stdin JSON)
- haiku-context-guard.py: reads `agent_type` (kind, default ""), `agent_id`, `tool_name`, `tool_input.allow_large`. Does NOT read transcript_path.
  - main thread = no `agent_id` -> exit 0 (no-op).
  - sonnet-analyst = `agent_id` set AND `agent_type == "sonnet-analyst"` AND tool in (Read, Grep, Glob) -> deny (unless lean_off()).
  - exit 0 (no-op) if: lean_off(), no agent_id, agent_type startswith "sonnet-" or "opus-", or agent_type in ("analyst", "reviewer").
  - everything else (i.e. Haiku-type subagents) : deny if tool_name == "Read" and tool_input.allow_large is truthy.
- main-thread-guard.py: reads `agent_id`, `tool_name`. Does NOT read agent_type or transcript_path.
  - main thread = no `agent_id`. Subagents (agent_id present) -> exit 0.
  - env CLAUDE_ALLOW_MAIN_TOOLS=1 or lean_off() -> exit 0.
  - tool_name in WORK_TOOLS -> deny.
- transcript_path: not referenced by either guard.

## 5. How deny is emitted
Both guards: print one JSON object to stdout, then sys.exit(0):
{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": "<text>"}}
Allow/no-op = no stdout output, exit 0. Errors reading stdin = exit 0 (fail open).

## 6. .claude/hooks/ directory listing (10 files, all -rwxr-xr-x)
_lean.py, agent-model-pin.py, context-alert.py, correction-guard.py, haiku-context-guard.py, history-export.py, ledger.py, main-thread-guard.py, session-context.py, style-adapter.py

## 7. Doubts
- haiku-context-guard.py docstring says autoCompactWindow 100000; settings.json line 4 says "autoCompactWindow": 150000.
- settings.json env CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH etc. not relevant here; not analyzed.
- _lean.py not read, so lean_off() logic and what "normal mode" sets are unverified.
