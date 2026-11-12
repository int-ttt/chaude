"""Shared by the hooks: is claude-lean switched off (normal mode) for this project/session?

Off when the session was started with CLAUDE_LEAN=0, or when this session's switch says off.
The switch is per session: logs/.lean-off-<sid> holds "off" or "on" (typed 일반모드 / "normal mode" and
절약모드 / "lean mode" — see style-adapter.py). Without it, the project default applies:
<ROOT>/.chaude/settings.json "lean" (on|off); legacy .claude/lean-off still means off (read only, never written)."""
import os

ROOT = os.environ.get("CLAUDE_PROJECT_DIR", ".")
OFF_FLAG = os.path.join(ROOT, ".claude", "lean-off")  # legacy project flag: read only


def clean_sid(sid):
    return "".join(c for c in (sid or "") if c.isalnum() or c in "_-")


def session_flag(sid):
    return os.path.join(ROOT, "logs", f".lean-off-{clean_sid(sid)}")


def lang():
    """User-facing output language, re-read on every call so /lang reaches a running session.
    <ROOT>/.chaude/settings.json "language" (ko|en) wins; auto, missing or unreadable falls back to CHAUDE_LANG; default "ko"."""
    try:
        import json
        with open(os.path.join(ROOT, ".chaude", "settings.json"), encoding="utf-8") as f:
            v = str(json.load(f).get("language", "")).strip().lower()
        if v in ("ko", "en"):
            return v
    except Exception:
        pass
    return "en" if os.environ.get("CHAUDE_LANG", "ko").strip().lower() == "en" else "ko"


def pick(ko, en):
    return en if lang() == "en" else ko


def trigger(keywords, name):
    """One regex alternation for trigger `name` across every language key of a hook's KEYWORDS dict
    ({"ko": {name: [fragments]}, "en": {...}}). All languages match, so a new language pack is one new key."""
    return "|".join(f"(?:{p})" for lang_kw in keywords.values() for p in lang_kw.get(name, []))


def project_lean():
    """Project default "on" or "off": .chaude/settings.json "lean" wins; else legacy .claude/lean-off means off; else on."""
    try:
        import json
        with open(os.path.join(ROOT, ".chaude", "settings.json"), encoding="utf-8") as f:
            v = str(json.load(f).get("lean", "")).strip().lower()
        if v in ("on", "off"):
            return v
    except Exception:
        pass
    return "off" if os.path.exists(OFF_FLAG) else "on"


def lean_off(sid=""):
    """sid = the hook input's session_id. Empty sid (no session context) uses the project default only."""
    if os.environ.get("CLAUDE_LEAN", "1") == "0":
        return True
    if clean_sid(sid):
        try:
            return open(session_flag(sid), encoding="utf-8").read().strip() == "off"
        except OSError:
            pass
    return project_lean() == "off"


import re
# Korean/English bare status pings (진행상황, 뭐해, 끝났어, ㅇㅇ, 응, progress, done yet?, ok ...): not worth recording.
_PING = re.compile(r"(진행\s*상황|상황|뭐\s*해\??|끝났어\??|다\s*됐어\??|ㅇㅇ|ㅇㅋ|네|응|ok|okay|yes|yep|yeah|sure|progress|status|done( yet)?\??|finished|what(?:'s| are) you doing\??)[\s?!.]*", re.I)


def is_ping(text):
    """User input too short or a bare status question ("ㅇㅇ", "진행상황"): not worth recording."""
    t = (text or "").strip()
    return len(t) < 5 or bool(_PING.fullmatch(t))


def history_dir(project):
    """This project's Claude Code transcript folder (~/.claude/projects/<encoded>), or "" if none.
    Same logic as `bin/chaude claude-history` (src/main/intt/internal/scripts/claude_history.go): encoded name first,
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
