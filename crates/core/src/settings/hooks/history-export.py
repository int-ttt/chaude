#!/usr/bin/env python3
"""SessionEnd / PreCompact hook: save the conversation as plain text under logs/history/ (no model calls).
The exporter itself is the Go helper `bin/chaude export-history` (scripts stay Go); hook JSON is passed on stdin."""
import os, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # also when run with python -I
try:
    from _lean import hook_log
except Exception:  # logging is optional: a missing or broken _lean must not break the hook
    def hook_log(*_a, **_k):
        pass
hook_log("history-export")
root = os.environ.get("CLAUDE_PROJECT_DIR", ".")
exe = os.path.join(root, "bin", "chaude.exe" if os.name == "nt" else "chaude")
if os.path.exists(exe):
    try:
        raw = sys.stdin.buffer.read() if not sys.stdin.isatty() else b""
        subprocess.run([exe, "export-history", "--hook", "--project", root], input=raw,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=50)
    except Exception:
        pass   # never block session end or compaction
sys.exit(0)
