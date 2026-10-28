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
