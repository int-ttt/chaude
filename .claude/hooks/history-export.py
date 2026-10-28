#!/usr/bin/env python3
"""SessionEnd / PreCompact hook: save the conversation as plain text under logs/history/ (no model calls)."""
import os, runpy, sys
root = os.environ.get("CLAUDE_PROJECT_DIR", ".")
script = os.path.join(root, "scripts", "export_history.py")
if os.path.exists(script):
    sys.argv = [script, "--hook", "--project", root]
    try:
        runpy.run_path(script, run_name="__main__")
    except SystemExit:
        pass
    except Exception:
        pass   # never block session end or compaction
sys.exit(0)
