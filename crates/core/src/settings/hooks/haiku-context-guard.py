#!/usr/bin/env python3
"""PreToolUse hook (matcher: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell), v0.8.2.

- Subagent whose agent_type is set and does not start with haiku- (opus-*, built-ins such as Explore,
  custom agents): no direct file or shell access. Every file touch goes through haiku-* agents.
  Escape hatch: env CLAUDE_HARNESS_ALLOW_SONNET_IO=1 disables the file-tool block (never Bash/PowerShell).
  statusline-setup is allowed for file tools only while env CHAUDE_ROTATING=1 is set (session rotation).
- haiku-*: Read's allow_large flag is denied (read in slices with offset/limit, or grep first). No-op in lean-off mode.
- Main thread (no agent_id, no agent_type): left to main-thread-guard.py.
- Fail-closed: if the body raises and the raw input names a subagent that is not haiku-*, exit 2 with the deny message.

Claude Code itself caps each Read at env CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS (8000; observed on a
Read: "cap 8000"), so no size estimate is needed here. allow_large is the only way around that cap, and
it is denied for Haiku so it reads in slices (offset/limit) or greps first. Agents compact on their own
(autoCompactWindow 100000)."""
import json, os, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    _reconf = getattr(_s, "reconfigure", None)
    try:
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__

FILE_TOOLS = ("Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Grep", "Glob")
SHELL_TOOLS = ("Bash", "PowerShell")
ALLOW_IO = ("haiku-", "sonnet-writer")
DENY_MSG = "Opus/non-haiku agents may not access files or shell directly (v0.8.2). Delegate to haiku-scout (read), haiku-batch-worker (write), or haiku-coder (code)."


try:
    from _lean import hook_log
except Exception:  # logging is optional: a missing or broken _lean must not break the hook
    def hook_log(*_a, **_k):
        pass


def deny(reason):
    hook_log(decision="deny")
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": reason}}, ensure_ascii=False))
    sys.exit(0)


raw = ""
try:
    from _lean import lean_off
    hook_log("haiku-context-guard")
    raw = sys.stdin.read()
    data = json.loads(raw)
    if not isinstance(data, dict):
        data = {}
    hook_log(data=data)
    kind = data.get("agent_type") or ""
    tool = data.get("tool_name") or ""
    tin = data.get("tool_input") or {}
    if not kind and not data.get("agent_id"):
        sys.exit(0)  # main thread
    if not kind:
        deny(DENY_MSG)  # agent_id set but agent_type empty (M1)
    if not kind.startswith(ALLOW_IO):
        if tool in SHELL_TOOLS:
            deny(DENY_MSG)  # Bash/PowerShell: no exception
        if os.environ.get("CLAUDE_HARNESS_ALLOW_SONNET_IO") == "1":
            sys.exit(0)
        if kind == "statusline-setup" and os.environ.get("CHAUDE_ROTATING") == "1":
            sys.exit(0)
        if tool in FILE_TOOLS:
            deny(DENY_MSG)
        sys.exit(0)
    if lean_off(data.get("session_id")):
        sys.exit(0)
    if tool == "Read" and tin.get("allow_large"):
        deny("allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first.")
    sys.exit(0)
except SystemExit:
    raise
except Exception:
    if '"agent_id"' in raw and not re.search(r'"agent_type"\s*:\s*"haiku-', raw):
        try:
            hook_log(decision="deny")
        except Exception:
            pass
        print(DENY_MSG, file=sys.stderr)
        sys.exit(2)
    sys.exit(0)
