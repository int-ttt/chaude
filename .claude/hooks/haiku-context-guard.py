#!/usr/bin/env python3
"""PreToolUse hook: keep Haiku subagents from swallowing huge tool results in one call.

Haiku agents compact automatically (agent frontmatter autoCompactWindow: 100000, below the 100k point
where Haiku's price rises), so they never have to stop. This hook only blocks a single call whose result
would be larger than HAIKU_CALL_MAX tokens (default 20000) and tells the agent to read in slices.
Images are counted at their resized size, not file size. No-op in normal mode."""
import json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

CALL_MAX = int(os.environ.get("HAIKU_CALL_MAX", "20000"))
READ_CAP = int(os.environ.get("CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS", "25000"))
IMAGE = (".jpg", ".jpeg", ".png", ".gif", ".webp", ".heic", ".heif", ".bmp", ".tif", ".tiff")


def deny(reason):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                      "permissionDecision": "deny", "permissionDecisionReason": reason}}))
    sys.exit(0)


try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
kind = data.get("agent_type") or ""
if lean_off() or not data.get("agent_id") or kind.startswith(("sonnet-", "opus-")) or kind in ("analyst", "reviewer"):
    sys.exit(0)
tool, inp = data.get("tool_name", ""), data.get("tool_input") or {}
if tool != "Read":
    sys.exit(0)
if inp.get("allow_large"):
    deny("allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first.")
path = inp.get("file_path", "")
if path.lower().endswith(IMAGE):
    sys.exit(0)
try:
    est = os.path.getsize(path) // 3            # ~3 bytes per token covers Korean UTF-8
except OSError:
    sys.exit(0)
if inp.get("limit"):
    est = min(est, int(inp["limit"]) * 40)
est = min(est, READ_CAP)
if est > CALL_MAX:
    deny(f"This read (~{est // 1000}k tokens) is too large for one call. Grep for the part you need, or Read with "
         f"offset/limit in pieces of about {CALL_MAX // 40} lines. Your context compacts automatically, so keep going.")
sys.exit(0)
