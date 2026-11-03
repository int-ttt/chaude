#!/usr/bin/env python3
"""SessionEnd / PreCompact hook: save the conversation as plain text under logs/history/ (no model calls).
The exporter itself is the Go helper `bin/chaude export-history` (scripts stay Go); hook JSON is passed on stdin."""
import os, subprocess, sys
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
