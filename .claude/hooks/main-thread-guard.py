#!/usr/bin/env python3
"""PreToolUse hook: the main (Opus) conversation only listens, decides, and delegates.
Work tools are blocked unless the call comes from a subagent (agent_id present).
Agent|Task from the main thread may only call haiku-* agents, opus-architect, claude-code-guide, or sonnet-writer.
Haiku main (the main transcript's newest assistant model is haiku, i.e. --model haiku) may call only opus-architect and haiku-output.
Off in normal mode (type 일반모드 or "normal mode", or start with CLAUDE_LEAN=0); CLAUDE_ALLOW_MAIN_TOOLS=1 still works."""
import json, os, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _reconf = getattr(_s, "reconfigure", None)
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # also when run with python -I
try:
    from _lean import hook_log
except Exception:  # logging is optional: a missing or broken _lean must not break the hook
    def hook_log(*_a, **_k):
        pass
from _lean import lean_off
hook_log("main-thread-guard")

WORK_TOOLS = {"Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Bash", "PowerShell", "Grep", "Glob", "WebSearch", "WebFetch"}
AGENT_TOOLS = {"Agent", "Task"}
MAIN_AGENTS = {"opus-architect", "claude-code-guide", "sonnet-writer"}  # plus any haiku-* name
HAIKU_MAIN_AGENTS = {"opus-architect", "haiku-output"}


def main_model(data):
    """Family of the main session's model ("haiku", "opus", "sonnet", "fable"), "" when unknown.
    PreToolUse input has no model field: the newest assistant entry of the main transcript (transcript_path)
    names it, e.g. claude-haiku-5-5. The model is fixed by --model at launch, so the newest entry is enough.
    Only read for Agent|Task calls, from the last 1 MB of the transcript."""
    m = str(data.get("model") or "")
    path = str(data.get("transcript_path") or "")
    if not m and path:
        try:
            with open(path, "rb") as f:
                f.seek(0, 2)
                f.seek(max(0, f.tell() - 1_000_000))
                lines = f.read().decode("utf-8", "ignore").splitlines()
        except OSError:
            lines = []
        for line in reversed(lines):
            try:
                e = json.loads(line)
            except Exception:
                continue
            msg = e.get("message") if isinstance(e, dict) else None
            v = str(msg.get("model") or "") if e.get("type") == "assistant" and isinstance(msg, dict) else ""
            if v and not v.startswith("<"):
                m = v
                break
    r = re.search(r"(opus|sonnet|haiku|fable)", m)
    return r.group(1) if r else ""

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
hook_log(data=data)
if os.environ.get("CHAUDE_ORCH", "").strip().lower() == "e":
    sys.exit(0)  # E main works inline
if lean_off(data.get("session_id")) or os.environ.get("CLAUDE_ALLOW_MAIN_TOOLS") == "1" or data.get("agent_id"):
    sys.exit(0)
tool = data.get("tool_name", "")
if tool in AGENT_TOOLS:
    kind = str((data.get("tool_input") or {}).get("subagent_type") or "")
    if main_model(data) == "haiku":
        if kind not in HAIKU_MAIN_AGENTS:
            hook_log(decision="deny")
            print(json.dumps({"hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": f"Haiku main may call only opus-architect (decide, brief) and haiku-output (final reply), not {kind or 'the default general-purpose agent'}. Send the request to opus-architect first; pure acks get a one-line reply with no agent."
            }}, ensure_ascii=False))
        sys.exit(0)
    if not (kind.startswith("haiku-") or kind in MAIN_AGENTS):
        hook_log(decision="deny")
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": f"Main session may only call haiku-* agents or opus-architect, not {kind or 'the default general-purpose agent'}. Delegate: search->haiku-scout, code->haiku-coder, writing->haiku-drafter, verify->haiku-checker, bulk->haiku-batch-worker. If the user wants you to work directly, tell them to type 일반모드 (or \"normal mode\")."
        }}, ensure_ascii=False))
    sys.exit(0)
if tool in WORK_TOOLS:
    hook_log(decision="deny")
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": f"Main session does not use {tool}. Delegate: search->haiku-scout, web->haiku-researcher/`/research`, writing->haiku-drafter, verify->haiku-checker/`/check`, bulk/notes->haiku-batch-worker, jobs->haiku-runner/`/status`, code->haiku-coder, design/judgment->decide yourself (Opus main; Haiku main only: opus-architect), final check->haiku-checker. If the user wants you to work directly, tell them to type 일반모드 (or \"normal mode\")."
    }}))
sys.exit(0)
