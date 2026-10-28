#!/usr/bin/env python3
"""Locate a project's Claude Code transcript folder (macOS, Linux, Windows).

usage: claude_history.py <project-dir>   → prints the folder path, or nothing if there is none.
Tries the encoded folder name first, then matches the `cwd` recorded inside transcripts.
"""
import glob, json, os, re, sys

for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass


def projects_root():
    return os.path.join(os.path.expanduser("~"), ".claude", "projects")


def history_dir(project):
    proj = os.path.abspath(os.path.expanduser(project))
    root = projects_root()
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


if __name__ == "__main__":
    print(history_dir(sys.argv[1]) if len(sys.argv) > 1 else "")
