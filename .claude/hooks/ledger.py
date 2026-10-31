#!/usr/bin/env python3
"""SubagentStop hook: append each agent's short result to notes/LEDGER.md (no model tokens).
Records the model the agent actually ran on (read from its transcript), flagged when it differs
from the model its name says. Gives continuity across auto-compaction and new sessions without /clear."""
import glob, json, os, re, sys, time
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _reconf = getattr(_s, "reconfigure", None)
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass


def agent_transcript(data):
    p = data.get("agent_transcript_path") or ""
    if p and os.path.exists(p):
        return p
    aid = data.get("agent_id")
    base = os.path.dirname(data.get("transcript_path") or "")
    hits = glob.glob(os.path.join(base, "**", f"agent-{aid}.jsonl"), recursive=True) if aid and base else []
    return hits[0] if hits else ""


def model_of(path):
    """Short name of the model in the agent's newest assistant entry, e.g. 'haiku 5.5'."""
    try:
        with open(path, "rb") as f:
            f.seek(0, 2)
            f.seek(max(0, f.tell() - 200_000))
            lines = f.read().decode("utf-8", "ignore").splitlines()
    except OSError:
        return ""
    for line in reversed(lines):
        try:
            e = json.loads(line)
        except Exception:
            continue
        m = (e.get("message") or {}).get("model") if e.get("type") == "assistant" else None
        if m and not m.startswith("<"):
            r = re.search(r"(opus|sonnet|haiku|fable)-(\d{1,2})(?:-(\d{1,2}))?(?!\d)", m)
            return f"{r.group(1)} {r.group(2)}" + (f".{r.group(3)}" if r.group(3) else "") if r else m
    return ""


def first_input(path):
    """Text of the first user message (the request the run started from); "" if unreadable."""
    try:
        with open(path, encoding="utf-8", errors="ignore") as f:
            for line in f:
                try:
                    e = json.loads(line)
                except Exception:
                    continue
                if e.get("type") != "user":
                    continue
                c = (e.get("message") or {}).get("content")
                if isinstance(c, list):
                    c = " ".join(x.get("text", "") for x in c if isinstance(x, dict) and x.get("type") == "text")
                return " ".join(str(c or "").split())
    except OSError:
        pass
    return ""                         # unknown: caller falls back to the agent's own message


data = json.load(sys.stdin)
msg = (data.get("last_assistant_message") or "").strip()
if not msg:
    sys.exit(0)
kind = data.get("agent_type") or "agent"
sys.dont_write_bytecode = True
from _lean import is_ping
_inp = model = ""
for _ in range(4):                    # one retry loop (worst case ~1.6s < hook timeout 5s): SubagentStop can fire before the agent transcript is fully on disk
    _tp = agent_transcript(data)
    _inp = _inp or first_input(_tp)
    model = model or model_of(_tp)
    if _inp and model:
        break
    time.sleep(0.4)
if is_ping(_inp or msg):
    sys.exit(0)                       # the input that started this run was a status ping ("진행상황", "ㅇㅇ"): skip
if not model:                         # fall back to the configured model in the agent file
    try:
        cfg = open(os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), ".claude", "agents", kind + ".md"), encoding="utf-8").read()
        m = re.search(r"^model:\s*(\w+)", cfg, re.M)
        model = m.group(1) + "(설정)" if m else ""
    except OSError:
        pass
tag = kind
if model:
    tag += " · " + model
    want = kind.split("-", 1)[0]
    if want in ("haiku", "sonnet", "opus") and want not in model:
        tag += " ⚠ 이름과 다름"
root = os.environ.get("CLAUDE_PROJECT_DIR", ".")
path = os.path.join(root, "notes", "LEDGER.md")
lines = [l.strip() for l in msg.splitlines() if l.strip()][:3]
entry = f"- {time.strftime('%m-%d %H:%M')} [{tag}] " + " / ".join(lines)[:300] + "\n"
os.makedirs(os.path.dirname(path), exist_ok=True)
try:
    old = open(path, encoding="utf-8").readlines()
except OSError:
    old = []
open(path, "w", encoding="utf-8").writelines((old + [entry])[-300:])
sys.exit(0)
