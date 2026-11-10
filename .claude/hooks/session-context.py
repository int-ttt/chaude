#!/usr/bin/env python3
"""SessionStart hook (startup / resume / compact / clear): inject a small snapshot of notes/ so work
continues after auto-compaction or in a new session, without /clear or re-reading files."""
import json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _reconf = getattr(_s, "reconfigure", None)
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

data = json.load(sys.stdin) if not sys.stdin.isatty() else {}
root = os.environ.get("CLAUDE_PROJECT_DIR", ".")
upd = ""
_NOTE = ("CLAUDE.md and agents apply only when Claude Code starts. If this session was reached via /resume, "
         "/exit, then claude -c to restart.")


def _session_start_ts(path):
    """Epoch seconds of the first record in the session transcript (None if unknown)."""
    import datetime
    try:
        with open(path, encoding="utf-8") as f:
            for line in f:
                ts = json.loads(line).get("timestamp")
                if ts:
                    return datetime.datetime.fromisoformat(str(ts).replace("Z", "+00:00")).timestamp()
    except Exception:
        pass
    return None


def emit(text):
    """Context for the model + systemMessage (shown to the user; plain stdout is not)."""
    msg: dict[str, object] = {"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": text}}
    if upd:
        msg["systemMessage"] = upd
    print(json.dumps(msg, ensure_ascii=False))


def _vt(v):                                          # "0.5.1" -> (0, 5, 1); unparseable -> ()
    try:
        return tuple(int(x) for x in v.split("."))
    except ValueError:
        return ()


if data.get("source", "startup") in ("startup", "resume"):   # compaction keeps the loaded CLAUDE.md: don't touch the record
    try:                                             # the version this session's CLAUDE.md/agents came from
        _vf = os.path.join(root, ".claude", "claude-lean.version")
        if not os.path.exists(_vf):                  # dev repo: no installed marker, use the root VERSION
            _vf = os.path.join(root, "VERSION")
        _v = open(_vf, encoding="utf-8").read().strip()
        _sid = "".join(ch for ch in str(data.get("session_id") or "x") if ch.isalnum() or ch in "-_")
        _f = os.path.join(root, "logs", f".lean-ver-{_sid}")
        try:
            _old = open(_f).read().strip()
        except OSError:
            _old = ""
        _lf = os.path.join(root, "logs", ".lean_ver_last")      # newest version any session here started with
        try:
            _last = open(_lf).read().strip()
        except OSError:
            _last = ""
        if data.get("source") == "resume":
            if not _old and _vt(_last) and _vt(_last) < _vt(_v):   # only a real upgrade, not a downgrade/other-session newer version; resumed under a new session id: no per-session record, use the project's last
                upd = f"claude-lean {_v} updated (was {_last}). {_NOTE}"
            elif _vt(_old) and _vt(_old) < _vt(_v):
                upd = f"claude-lean {_v} updated (was {_old}). {_NOTE}"
            elif not _old:                           # session predates the version record (0.4.11 and older)
                _t0 = _session_start_ts(data.get("transcript_path") or "")
                if _t0 is not None and os.path.getmtime(_vf) > _t0:
                    upd = f"claude-lean {_v} updated (this conversation started on an older version). {_NOTE}"
        os.makedirs(os.path.join(root, "logs"), exist_ok=True)
        open(_f, "w").write(_v)
        open(_lf, "w").write(_v)
    except OSError:
        pass
if lean_off():
    emit("[normal mode] The claude-lean policy in CLAUDE.md is OFF: work directly with tools, normal-length replies. "
          "(The user types 절약모드 or \"lean mode\" to turn it back on.)" + (f"\n[update] {upd}" if upd else ""))
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


def pick(rel_new, rel_old):
    """record/ file if it exists (absolute HARNESS_RECORD_DIR wins when set), else the legacy notes/ file."""
    rec = os.environ.get("HARNESS_RECORD_DIR")
    new = os.path.abspath(os.path.join(rec, os.path.basename(rel_new))) if rec else rel_new
    return new if os.path.exists(os.path.join(root, new)) else rel_old


if data.get("source", "startup") == "startup":       # storage: per-agent/session hook state older than 3 days
    import glob as _g0, time as _t0
    for _p in _g0.glob(os.path.join(root, "logs", ".*-*")):
        try:
            if _t0.time() - os.path.getmtime(_p) > 3 * 86400:
                os.remove(_p)
        except OSError:
            pass

out = [f"[session:{data.get('source', 'start')}] snapshot of notes/ + record/ (state lives in files; don't re-read them unless needed):"]
open_items = head("notes/OPEN.md", 20)
if open_items:
    out += ["OPEN:"] + open_items
recent = tail(pick("record/LEDGER.md", "notes/LEDGER.md"), 5)
if recent:
    out += ["Recent agent results (LEDGER):"] + recent
if data.get("source", "startup") in ("startup", "clear"):
    # Carry over the tail of the previous session (plain text, no model): the newest other transcript.
    # On /clear the previous session is the transcript just cleared (a new session id, new transcript file).
    try:
        import glob as _g
        from _lean import history_dir, is_ping
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
                    if c and not c.startswith("<") and not (e.get("type") == "user" and is_ping(c)):
                        who = "user" if e.get("type") == "user" else "claude"
                        msgs.append(f"{who}: {c[:200]}")
        if msgs:
            out += ["Previous session, last messages:"] + msgs[-6:]
    except Exception:
        pass
inbox = tail(pick("record/INBOX.md", "notes/INBOX.md"), 50)
if inbox:
    out.append(f"INBOX: {len(inbox)} queued user notes (delivered with the next instruction).")
if upd:
    out.append(f"[update] {upd} Mention nothing; the user was already told.")
if len(out) > 1 or upd:
    emit("\n".join(out)[:3000])
sys.exit(0)
