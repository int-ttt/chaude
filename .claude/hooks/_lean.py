"""Shared by the hooks: is claude-lean switched off (normal mode) for this project/session?

Off when the session was started with CLAUDE_LEAN=0, or while .claude/lean-off exists
(created by typing 일반모드, removed by typing 절약모드 — see style-adapter.py)."""
import os

ROOT = os.environ.get("CLAUDE_PROJECT_DIR", ".")
OFF_FLAG = os.path.join(ROOT, ".claude", "lean-off")


def lean_off():
    return os.environ.get("CLAUDE_LEAN", "1") == "0" or os.path.exists(OFF_FLAG)


import re
_PING = re.compile(r"(진행\s*상황|상황|뭐\s*해\??|끝났어\??|다\s*됐어\??|ㅇㅇ|ㅇㅋ|네|응|ok|yes)[\s?!.]*", re.I)


def is_ping(text):
    """User input too short or a bare status question ("ㅇㅇ", "진행상황"): not worth recording."""
    t = (text or "").strip()
    return len(t) < 5 or bool(_PING.fullmatch(t))


def history_dir(project):
    """This project's Claude Code transcript folder (~/.claude/projects/<encoded>), or "" if none.
    Same logic as `bin/chaude claude-history` (internal/scripts/claude_history.go): encoded name first,
    then the folder whose transcripts record this project as `cwd`."""
    import glob, json
    proj = os.path.abspath(os.path.expanduser(project))
    root = os.path.join(os.path.expanduser("~"), ".claude", "projects")
    d = os.path.join(root, re.sub(r"[^A-Za-z0-9]", "-", proj))
    if os.path.isdir(d):
        return d
    want = os.path.normcase(os.path.normpath(proj))
    for cand in glob.glob(os.path.join(root, "*")):
        for f in glob.glob(os.path.join(cand, "*.jsonl"))[:3]:
            try:
                with open(f, encoding="utf-8", errors="ignore") as fh:
                    for _, line in zip(range(50), fh):
                        if '"cwd"' not in line:
                            continue
                        try:
                            cwd = json.loads(line).get("cwd")
                        except Exception:
                            continue
                        if cwd and os.path.normcase(os.path.normpath(cwd)) == want:
                            return cand
            except OSError:
                pass
    return ""
