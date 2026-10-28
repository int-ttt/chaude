#!/usr/bin/env python3
"""PreToolUse hook: retire every Haiku subagent before its prompt passes HAIKU_CONTEXT_CAP (default 80k,
below the 100k point where Haiku's price rises). Images are counted at their resized size, not file size.

Reads the subagent's own transcript for the last request's real token usage, adds an estimate
of what this tool call (and parallel calls in the same turn) will return, and denies calls that
would cross the limit. Near the limit only Write/Edit are allowed so the agent can save progress.
"""
import glob, json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

CAP = int(os.environ.get("HAIKU_CONTEXT_CAP", "80000"))  # deny a call if used + pending + estimate exceeds this
SAVE_ONLY = CAP - 5_000                                    # above this, only Write/Edit (save progress) are allowed
NON_HAIKU = {"analyst", "reviewer"}                    # pre-2026.10.28 names of the Sonnet agents
READ_CAP = int(os.environ.get("CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS", "25000"))
BASH_CAP = int(os.environ.get("BASH_MAX_OUTPUT_LENGTH", "30000"))
FIXED = {"WebFetch": 4000, "WebSearch": 4000, "Grep": 3000, "Glob": 1500,
         "Write": 300, "Edit": 300, "MultiEdit": 300, "NotebookEdit": 300}


def deny(reason):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                      "permissionDecision": "deny", "permissionDecisionReason": reason}}))
    sys.exit(0)


def transcript(data):
    base = os.path.dirname(data.get("transcript_path") or "")
    aid, sid = data["agent_id"], data.get("session_id", "")
    p = os.path.join(base, sid, "subagents", f"agent-{aid}.jsonl")
    if os.path.exists(p):
        return p
    hits = glob.glob(os.path.join(base, "**", f"agent-{aid}.jsonl"), recursive=True)
    return hits[0] if hits else None


def last_usage(path):
    """(message id, prompt tokens of the next request) from the newest assistant entry with usage."""
    with open(path, "rb") as f:
        f.seek(0, 2)
        f.seek(max(0, f.tell() - 400_000))
        lines = f.read().decode("utf-8", "ignore").splitlines()
    for line in reversed(lines):
        try:
            e = json.loads(line)
        except Exception:
            continue
        u = (e.get("message") or {}).get("usage")
        if e.get("type") == "assistant" and u:
            used = sum(u.get(k) or 0 for k in ("input_tokens", "cache_creation_input_tokens",
                                                "cache_read_input_tokens", "output_tokens"))
            return (e.get("message") or {}).get("id") or e.get("uuid"), used
    return None, 0


IMAGE = (".jpg", ".jpeg", ".png", ".gif", ".webp", ".heic", ".heif", ".bmp", ".tif", ".tiff")
IMAGE_TOKENS = 1700                                         # images are downscaled before the model sees them


def estimate(tool, inp):
    if tool == "Read":
        if inp.get("file_path", "").lower().endswith(IMAGE):
            return IMAGE_TOKENS
        try:
            size = os.path.getsize(inp.get("file_path", ""))
        except OSError:
            return 500
        est = size // 3                      # ~3 bytes per token covers Korean UTF-8
        if inp.get("limit"):
            est = min(est, int(inp["limit"]) * 40)
        return min(est, READ_CAP) + 200
    if tool in ("Bash", "PowerShell"):
        return BASH_CAP // 2 + 200           # worst case: output at the char cap
    return FIXED.get(tool, 3000)


data = json.load(sys.stdin)
kind = data.get("agent_type") or ""
if lean_off() or not data.get("agent_id") or kind.startswith(("sonnet-", "opus-")) or kind in NON_HAIKU:
    sys.exit(0)
tool, inp = data.get("tool_name", ""), data.get("tool_input") or {}

if tool == "Read" and inp.get("allow_large"):
    deny("allow_large is disabled for Haiku agents (context cap). Use offset/limit or grep.")

path = transcript(data)
if not path:
    sys.exit(0)
msg_id, used = last_usage(path)

# Parallel calls in one turn all see the same last usage; accumulate their estimates.
state_dir = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "logs")
state = os.path.join(state_dir, f".ctx-{data['agent_id']}")
pending = 0
try:
    s = json.load(open(state))
    if s.get("msg") == msg_id:
        pending = s.get("pending", 0)
except Exception:
    pass

est = estimate(tool, inp)
k = lambda n: f"{n // 1000}k"
if used >= SAVE_ONLY and tool not in ("Write", "Edit", "MultiEdit"):
    deny(f"CONTEXT-LIMIT: prompt is ~{k(used)}/{k(CAP)}. A fresh agent will continue. Stop reading. Write remaining findings/progress "
         f"to notes/runs/<YYMMDD>-<agent>-<topic>.md, then return 'CONTEXT-LIMIT: <path>' in <=5 lines.")
saving = tool in ("Write", "Edit", "MultiEdit")
if used + pending + est > (CAP + 3_000 if saving else CAP):
    deny(f"This {tool} call (~{k(est)}) would push the prompt past {k(CAP)} (now ~{k(used + pending)}). "
         f"Use grep / head / Read offset+limit for a smaller slice, or save progress to notes/runs/ and return "
         f"'CONTEXT-LIMIT: <path>'.")

try:
    os.makedirs(state_dir, exist_ok=True)
    json.dump({"msg": msg_id, "pending": pending + est}, open(state, "w"))
except Exception:
    pass
sys.exit(0)
