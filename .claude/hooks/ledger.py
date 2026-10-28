#!/usr/bin/env python3
"""SubagentStop hook: append each agent's short result to notes/LEDGER.md (no model tokens).
Records the model the agent actually ran on (read from its transcript), flagged when it differs
from the model its name says. Gives continuity across auto-compaction and new sessions without /clear."""
import glob, json, os, re, sys, time
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
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


data = json.load(sys.stdin)
msg = (data.get("last_assistant_message") or "").strip()
if len(msg) < 5 or re.fullmatch(r"(진행\s*상황|상황|뭐\s*해\??|끝났어\??|다 됐어\??|ㅇㅇ|ㅇㅋ|네|응|ok|yes)[\s?!.]*", msg, re.I):
    sys.exit(0)                       # too short / status ping: nothing worth carrying over
kind = data.get("agent_type") or "agent"
model = ""
for _ in range(4):                    # the last transcript line can land just after this hook starts
    model = model_of(agent_transcript(data))
    if model:
        break
    time.sleep(0.4)
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
