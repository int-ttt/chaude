#!/usr/bin/env python3
"""PreToolUse hook (Agent|Task): every subagent runs on the model its name says.

- haiku-* / sonnet-* / opus-* agents: a `model` passed by the main thread is removed, so the agent file's
  `model:` line always wins (per-call model would otherwise override it).
- Built-in general agents (general-purpose, Explore, Plan, claude-code-guide): run on Haiku unless
  Sonnet or Haiku was asked for, so nothing silently runs on the main thread's Opus.
- Called from inside a subagent (Sonnet/Opus handing work down): only haiku-* agents are allowed.
- Other agents (the user's own) are left alone. No-op in normal mode."""
import json, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

BUILTIN = {"general-purpose", "Explore", "Plan", "claude-code-guide"}

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
if lean_off() or data.get("tool_name") not in ("Agent", "Task"):
    sys.exit(0)
inp = dict(data.get("tool_input") or {})
kind = inp.get("subagent_type") or "general-purpose"
if data.get("agent_id") and not kind.startswith("haiku-"):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": f"Subagents may only hand work to haiku-* agents, not {kind}. "
                                                  "For anything else, finish or return ESCALATE / [haiku] steps to the main thread."}}))
    sys.exit(0)
asked = inp.get("model")
if kind.startswith(("haiku-", "sonnet-", "opus-")):
    if not asked:
        sys.exit(0)
    del inp["model"]
    why = f"model '{asked}' removed: {kind} runs on its own model"
elif kind in BUILTIN and asked not in ("haiku", "sonnet"):
    inp["model"] = "haiku"
    why = f"{kind} pinned to haiku" + (f" (asked '{asked}')" if asked else "")
else:
    sys.exit(0)
print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow",
                                         "permissionDecisionReason": why, "updatedInput": inp}}, ensure_ascii=False))
sys.exit(0)
