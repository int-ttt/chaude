#!/usr/bin/env python3
"""PreToolUse hook (Agent|Task): every subagent runs on the model its name says.

- haiku-* / opus-* agents: a `model` passed by the main thread is removed, so the agent file's
  `model:` line always wins (per-call model would otherwise override it).
- Built-in general-purpose / Explore / Plan from the main thread: denied (they carry a large default prompt);
  the reason names the haiku-* agent to use. claude-code-guide is allowed and pinned to Haiku.
- Called from inside a subagent (Opus handing work down): only haiku-* agents are allowed (v0.8.2).
  The opus caller check runs before lean_off(), so it also applies in default mode.
- haiku-prompter (loop owner): main thread only; it may call opus-architect/haiku-*. (sonnet tier removed 2026-10-31.)
- Other agents (the user's own) are left alone. No-op in normal mode otherwise."""
import json, os, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    _reconf = getattr(_s, "reconfigure", None)
    try:
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # also when run with python -I
from _lean import lean_off

BUILTIN = {"claude-code-guide"}
BLOCKED = {"general-purpose", "Explore", "Plan"}
SUB_CALLERS = {"opus-architect"}

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
if data.get("tool_name") in ("Agent", "Task") and data.get("agent_id"):
    _target = str((data.get("tool_input") or {}).get("subagent_type") or "")
    if _target == "haiku-prompter":
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                          "permissionDecisionReason": "haiku-prompter는 메인 스레드에서만 호출할 수 있습니다 (재귀 금지)."}}, ensure_ascii=False))
        sys.exit(0)
    if data.get("agent_type") == "haiku-prompter":
        if _target == "opus-architect" or (_target.startswith("haiku-") and _target != "haiku-prompter"):
            sys.exit(0)
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                          "permissionDecisionReason": "haiku-prompter는 opus-architect/haiku-* 만 호출할 수 있습니다."}}, ensure_ascii=False))
        sys.exit(0)
if data.get("tool_name") in ("Agent", "Task") and data.get("agent_id") and data.get("agent_type") in SUB_CALLERS:
    target = str((data.get("tool_input") or {}).get("subagent_type") or "")
    if not target.startswith("haiku-"):
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                          "permissionDecisionReason": "opus 서브에이전트는 haiku-* 에이전트만 호출할 수 있습니다 (v0.8.2)."}}, ensure_ascii=False))
        sys.exit(0)
if lean_off() or data.get("tool_name") not in ("Agent", "Task"):
    sys.exit(0)
if os.environ.get("CHAUDE_ORCH", "").strip().lower() == "e" and not data.get("agent_id"):
    _ti = data.get("tool_input") or {}
    _st = str(_ti.get("subagent_type") or "")
    _prompt = str(_ti.get("prompt") or "")
    if _st.startswith("opus-") and "ESCALATE" not in _prompt:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                          "permissionDecisionReason": "E gate: Opus only on escalation after 2 Haiku failures (put ESCALATE in the brief)"}}, ensure_ascii=False))
        sys.exit(0)
    _paths = []
    for _p in re.findall(r'(/[^\s`\'"(),<>]+)', _prompt):
        _p = re.sub(r':\d+(-\d+)?$', '', _p.rstrip(".:;"))
        if _p not in _paths:
            _paths.append(_p)
    _raw = sum(os.path.getsize(_p) for _p in _paths if os.path.isfile(_p))
    _m = re.search(r'RETURN[^\n]*?<=\s*(\d+)\s*chars', _prompt)
    _cap = int(_m.group(1)) if _m else None
    if _raw > 0 and (_cap is None or _cap > 0.25 * _raw):
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                          "permissionDecisionReason": f"E gate: RETURN cap {_cap} > 0.25 x raw {_raw} bytes (or no cap); do it inline with your own tools"}}, ensure_ascii=False))
        sys.exit(0)
inp = dict(data.get("tool_input") or {})
kind = inp.get("subagent_type") or "general-purpose"
if data.get("agent_id") and not kind.startswith("haiku-"):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": f"Subagents may only hand work to haiku-* agents, not {kind}. "
                                                  "For anything else, finish or return ESCALATE / [haiku] steps to the main thread."}}))
    sys.exit(0)
if kind in BLOCKED:
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": f"Built-in {kind} agent is blocked (large default prompt). Use haiku-scout "
                                                  "(search), haiku-coder / haiku-batch-worker (edits), "
                                                  "opus-architect (design, judgment)."}}))
    sys.exit(0)
asked = inp.get("model")
if kind.startswith(("haiku-", "opus-")):
    if not asked:
        sys.exit(0)
    del inp["model"]
    why = f"model '{asked}' removed: {kind} runs on its own model"
elif kind in BUILTIN and asked != "haiku":
    inp["model"] = "haiku"
    why = f"{kind} pinned to haiku" + (f" (asked '{asked}')" if asked else "")
else:
    sys.exit(0)
print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "allow",
                                         "permissionDecisionReason": why, "updatedInput": inp}}, ensure_ascii=False))
sys.exit(0)
