#!/usr/bin/env python3
"""UserPromptSubmit hook: when the main context passes LEAN_RESET_AT tokens (0 or unset = off), ask the main
thread to add a one-line notice suggesting /exit → claude (no compaction; the terminal keeps the history).
Reminds again only after every further 25k."""
import json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _reconf = getattr(_s, "reconfigure", None)
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off, pick

if lean_off():
    sys.exit(0)
data = json.load(sys.stdin)
path = data.get("transcript_path") or ""
limit = int(os.environ.get("LEAN_RESET_AT", "0"))
if limit <= 0:
    sys.exit(0)
used = 0
try:
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
            break
except OSError:
    sys.exit(0)
if used < limit:
    sys.exit(0)

state = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "logs", ".ctx-alert-" + str(data.get("session_id", "x")))
last = 0
try:
    last = int(open(state).read())
except Exception:
    pass
if last and used < last + 25_000:
    sys.exit(0)
try:
    os.makedirs(os.path.dirname(state), exist_ok=True)
    open(state, "w").write(str(used))
except OSError:
    pass
k = used // 1000
print(f"[ctx {k}k] After your reply, add exactly one line in "
      + pick("Korean (polite form): ", "plain English: ")
      + "suggest /exit, then claude in the same terminal, to restart without compaction (history stays on screen and in logs/history).")
sys.exit(0)
