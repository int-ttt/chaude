#!/usr/bin/env python3
"""PreToolUse hook: the main (Opus) conversation only listens, decides, and delegates.
Work tools are blocked unless the call comes from a subagent (agent_id present).
Agent|Task from the main thread may only call haiku-* agents, opus-architect, claude-code-guide, or sonnet-writer.
Off in normal mode (type 일반모드 or "normal mode", or start with CLAUDE_LEAN=0); CLAUDE_ALLOW_MAIN_TOOLS=1 still works."""
import json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _reconf = getattr(_s, "reconfigure", None)
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # also when run with python -I
from _lean import lean_off

WORK_TOOLS = {"Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Bash", "PowerShell", "Grep", "Glob", "WebSearch", "WebFetch"}
AGENT_TOOLS = {"Agent", "Task"}
MAIN_AGENTS = {"opus-architect", "claude-code-guide", "sonnet-writer"}  # plus any haiku-* name

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
if os.environ.get("CHAUDE_ORCH", "").strip().lower() == "e":
    sys.exit(0)  # E main works inline
if lean_off() or os.environ.get("CLAUDE_ALLOW_MAIN_TOOLS") == "1" or data.get("agent_id"):
    sys.exit(0)
tool = data.get("tool_name", "")
if tool in AGENT_TOOLS:
    kind = str((data.get("tool_input") or {}).get("subagent_type") or "")
    if not (kind.startswith("haiku-") or kind in MAIN_AGENTS):
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": f"Main session may only call haiku-* agents or opus-architect, not {kind or 'the default general-purpose agent'}. Delegate: search->haiku-scout, code->haiku-coder, writing->haiku-drafter, verify->haiku-checker, bulk->haiku-batch-worker. If the user wants you to work directly, tell them to type 일반모드 (or \"normal mode\")."
        }}, ensure_ascii=False))
    sys.exit(0)
if tool in WORK_TOOLS:
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": f"Main session does not use {tool}. Delegate: search->haiku-scout, web->haiku-researcher/`/research`, writing->haiku-drafter, verify->haiku-checker/`/check`, bulk/notes->haiku-batch-worker, jobs->haiku-runner/`/status`, code->haiku-coder, design/judgment->decide yourself (Opus main; Haiku main only: opus-architect), final check->haiku-checker. If the user wants you to work directly, tell them to type 일반모드 (or \"normal mode\")."
    }}))
sys.exit(0)
