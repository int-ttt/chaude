#!/usr/bin/env python3
"""PreToolUse hook (matcher: Read): Haiku subagents may not use Read's allow_large flag.

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
if lean_off() or not data.get("agent_id") or kind.startswith(("sonnet-", "opus-", "fable-")) or kind in ("analyst", "reviewer"):
    sys.exit(0)
if data.get("tool_name") == "Read" and (data.get("tool_input") or {}).get("allow_large"):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": "allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first."}}))
sys.exit(0)
