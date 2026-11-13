#!/usr/bin/env python3
"""PreToolUse hook (matcher: Read). Haiku subagents only (agent_type starts with haiku-); everything else passes.

- Full read (no offset, no limit) of a file over 400 lines: denied, use Grep or Read offset/limit.
- Full read of a path this session already read in full, with the same mtime and size: denied.
- State: logs/.read-seen-<sid>.json, {path: [mtime_ns, size]}, one file per session (same isolation as the
  INBOX), read and written under the shared hooks lock (_lean.locked). No session id: no repeat check.
- Fail-open: any error exits 0 with no output. No-op in lean-off mode."""
import json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    _reconf = getattr(_s, "reconfigure", None)
    try:
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # also when run with python -I

MAX_FULL_LINES = 400

try:
    from _lean import ROOT, clean_sid, hook_log, lean_off, locked
except Exception:  # no shared helpers: nothing to guard with, fail open
    sys.exit(0)


def deny(reason):
    hook_log(decision="deny")
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": reason}}, ensure_ascii=False))
    sys.exit(0)


def count_lines(path):
    n, last = 0, b""
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            n += chunk.count(b"\n")
            last = chunk[-1:]
    return n + (1 if last and last != b"\n" else 0)


def load(path):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except Exception:
        return {}


def main():
    raw = sys.stdin.read()
    data = json.loads(raw) if raw.strip() else {}
    if not isinstance(data, dict):
        return
    hook_log("haiku-read-guard")
    hook_log(data=data)
    if not (data.get("agent_type") or "").startswith("haiku-"):
        return
    if data.get("tool_name") != "Read" or lean_off(data.get("session_id")):
        return
    tin = data.get("tool_input") or {}
    fp = tin.get("file_path")
    if not isinstance(fp, str) or not fp or tin.get("offset") is not None or tin.get("limit") is not None:
        return  # a slice, or no path: the tool reports its own error
    path = os.path.normcase(os.path.abspath(fp))
    if not os.path.isfile(path):
        return
    st = os.stat(path)
    sig = [st.st_mtime_ns, st.st_size]
    lines = count_lines(path)
    if lines > MAX_FULL_LINES:
        deny(f"{os.path.basename(path)} has {lines} lines (over {MAX_FULL_LINES}): use Grep or Read offset/limit, not a full read.")
    sid = clean_sid(data.get("session_id"))
    if not sid:
        return
    logs = os.path.join(ROOT, "logs")
    os.makedirs(logs, exist_ok=True)
    state = os.path.join(logs, f".read-seen-{sid}.json")
    with locked(os.path.join(logs, ".hooks.lock")):
        seen = load(state)
        if seen.get(path) == sig:
            deny(f"{os.path.basename(path)} was already read in full this session and is unchanged: use that result, or Read offset/limit for the part you need.")
        seen[path] = sig
        with open(state, "w", encoding="utf-8") as f:
            json.dump(seen, f)


try:
    main()
except SystemExit:
    raise
except Exception:
    sys.exit(0)  # fail open
