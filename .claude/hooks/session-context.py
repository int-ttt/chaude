#!/usr/bin/env python3
"""SessionStart hook (startup / resume / compact): inject a small snapshot of notes/ so work
continues after auto-compaction or in a new session, without /clear or re-reading files."""
import json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

data = json.load(sys.stdin) if not sys.stdin.isatty() else {}
root = os.environ.get("CLAUDE_PROJECT_DIR", ".")
if lean_off():
    print("[normal mode] The claude-lean policy in CLAUDE.md is OFF: work directly with tools, normal-length replies. "
          "(The user types 절약모드 to turn it back on.)")
    sys.exit(0)


def head(rel, n, skip_comments=True):
    try:
        with open(os.path.join(root, rel), encoding="utf-8") as f:
            ls = [l.rstrip() for l in f if l.strip() and not (skip_comments and l.lstrip().startswith("<!--"))]
    except OSError:
        return []
    return ls[:n]


def tail(rel, n):
    try:
        with open(os.path.join(root, rel), encoding="utf-8") as f:
            return [l.rstrip() for l in f if l.strip()][-n:]
    except OSError:
        return []


if data.get("source", "startup") == "startup":       # storage: per-agent/session hook state older than 3 days
    import glob as _g0, time as _t0
    for _p in _g0.glob(os.path.join(root, "logs", ".*-*")):
        try:
            if _t0.time() - os.path.getmtime(_p) > 3 * 86400:
                os.remove(_p)
        except OSError:
            pass

out = [f"[session:{data.get('source', 'start')}] snapshot of notes/ (state lives in files; don't re-read them unless needed):"]
open_items = head("notes/OPEN.md", 20)
if open_items:
    out += ["OPEN:"] + open_items
recent = tail("notes/LEDGER.md", 12)
if recent:
    out += ["Recent agent results (LEDGER):"] + recent
if data.get("source", "startup") == "startup":
    # Carry over the tail of the previous session (plain text, no model): the newest other transcript.
    try:
        import glob as _g
        sys.path.insert(0, os.path.join(root, "scripts"))
        from claude_history import history_dir
        hdir = history_dir(root)
        cur = os.path.basename(data.get("transcript_path") or "")
        prev = sorted((p for p in _g.glob(os.path.join(hdir, "*.jsonl")) if os.path.basename(p) != cur),
                      key=os.path.getmtime)[-1:] if hdir else []
        msgs = []
        for p in prev:
            with open(p, encoding="utf-8", errors="ignore") as fh:
                for line in fh:
                    try:
                        e = json.loads(line)
                    except Exception:
                        continue
                    if e.get("type") not in ("user", "assistant") or e.get("isMeta"):
                        continue
                    c = (e.get("message") or {}).get("content")
                    if isinstance(c, list):
                        if any(isinstance(x, dict) and x.get("type") == "tool_result" for x in c):
                            continue
                        c = "\n".join(x.get("text", "") for x in c if isinstance(x, dict) and x.get("type") == "text")
                    c = " ".join(str(c or "").split())
                    if c and not c.startswith("<"):
                        who = "user" if e.get("type") == "user" else "claude"
                        msgs.append(f"{who}: {c[:280]}")
        if msgs:
            out += ["Previous session, last messages:"] + msgs[-15:]
    except Exception:
        pass
inbox = tail("notes/INBOX.md", 50)
if inbox:
    out.append(f"INBOX: {len(inbox)} queued user notes (delivered with the next instruction).")
if len(out) > 1:
    print("\n".join(out)[:6000])
sys.exit(0)
