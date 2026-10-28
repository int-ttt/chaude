"""Shared by the hooks: is claude-lean switched off (normal mode) for this project/session?

Off when the session was started with CLAUDE_LEAN=0, or while .claude/lean-off exists
(created by typing 일반모드, removed by typing 절약모드 — see style-adapter.py)."""
import os

ROOT = os.environ.get("CLAUDE_PROJECT_DIR", ".")
OFF_FLAG = os.path.join(ROOT, ".claude", "lean-off")


def lean_off():
    return os.environ.get("CLAUDE_LEAN", "1") == "0" or os.path.exists(OFF_FLAG)
