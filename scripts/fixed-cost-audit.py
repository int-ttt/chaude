#!/usr/bin/env python3
"""Estimate the fixed context a session pays before the first prompt: CLAUDE.md files, agent and skill descriptions,
the MEMORY.md index, and SessionStart hook output. Tokens are estimated as bytes / 4.

Usage:
  python3 scripts/fixed-cost-audit.py [--project DIR] [--home DIR] [--hooks]

- CLAUDE.md: project CLAUDE.md, .claude/CLAUDE.md, and home .claude/CLAUDE.md (whole file).
- agents and skills: only the name and description from the frontmatter (the body loads on use).
- MEMORY.md: the index under home .claude/projects/<encoded project path>/memory/ (whole file).
- hook: stdout of each SessionStart command in the project and home settings.json. Only with --hooks, because
  running a hook can write under the project's logs/ (the audit session id is "audit"). Without it, hooks are not counted.

Prints one table sorted by estimated tokens, descending, with a total. Stdlib only.
"""
import argparse
import glob
import json
import os
import re
import subprocess
import sys


def frontmatter(text):
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    if end < 0:
        return {}
    out, key = {}, None
    for line in text[3:end].splitlines():
        if line[:1] not in (" ", "\t", "") and ":" in line:
            key, _, val = line.partition(":")
            key = key.strip()
            out[key] = val.strip().strip('"').strip("'")
        elif key and line.strip():
            out[key] = (out[key] + " " + line.strip()).strip()
    return out


def read(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return f.read()


def collect(project, home, hooks):
    rows = []  # (bytes, label)
    for label, path in (
        ("CLAUDE.md (project)", os.path.join(project, "CLAUDE.md")),
        ("CLAUDE.md (project .claude)", os.path.join(project, ".claude", "CLAUDE.md")),
        ("CLAUDE.md (user)", os.path.join(home, ".claude", "CLAUDE.md")),
    ):
        if os.path.isfile(path):
            rows.append((len(read(path).encode("utf-8")), label))
    for scope, base in (("project", os.path.join(project, ".claude")), ("user", os.path.join(home, ".claude"))):
        for path in sorted(glob.glob(os.path.join(base, "agents", "*.md"))):
            meta = frontmatter(read(path))
            text = f"{meta.get('name', os.path.basename(path))}: {meta.get('description', '')}"
            rows.append((len(text.encode("utf-8")), f"agent {scope} {os.path.basename(path)}"))
        for path in sorted(glob.glob(os.path.join(base, "skills", "*", "SKILL.md"))):
            meta = frontmatter(read(path))
            text = f"{meta.get('name', os.path.basename(os.path.dirname(path)))}: {meta.get('description', '')}"
            rows.append((len(text.encode("utf-8")), f"skill {scope} {os.path.basename(os.path.dirname(path))}"))
    encoded = re.sub(r"[^A-Za-z0-9]", "-", os.path.abspath(project))
    memory = os.path.join(home, ".claude", "projects", encoded, "memory", "MEMORY.md")
    if os.path.isfile(memory):
        rows.append((len(read(memory).encode("utf-8")), "memory MEMORY.md"))
    if hooks:
        for scope, base in (("project", project), ("user", home)):
            settings = os.path.join(base, ".claude", "settings.json")
            if not os.path.isfile(settings):
                continue
            try:
                conf = json.loads(read(settings))
            except ValueError:
                continue
            for group in (conf.get("hooks") or {}).get("SessionStart") or []:
                for hook in group.get("hooks") or []:
                    cmd = hook.get("command")
                    if not cmd:
                        continue
                    payload = json.dumps({"session_id": "audit", "hook_event_name": "SessionStart", "source": "startup"})
                    env = dict(os.environ, CLAUDE_PROJECT_DIR=project)
                    try:
                        out = subprocess.run(cmd, shell=True, input=payload, capture_output=True, text=True,
                                             env=env, timeout=10).stdout
                    except (OSError, subprocess.SubprocessError):
                        out = ""
                    rows.append((len(out.encode("utf-8")), f"hook SessionStart {scope} {cmd[:60]}"))
    return rows


def main(argv):
    ap = argparse.ArgumentParser(description="Estimate fixed context tokens (bytes / 4).")
    ap.add_argument("--project", default=os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
    ap.add_argument("--home", default=os.path.expanduser("~"))
    ap.add_argument("--hooks", action="store_true", help="also run SessionStart hooks and count their stdout")
    args = ap.parse_args(argv)
    rows = [(b, label, b // 4) for b, label in collect(args.project, args.home, args.hooks)]
    rows.sort(key=lambda r: (-r[2], r[1]))
    print("fixed-context audit (tokens ~ bytes / 4)")
    print(f"{'tokens':>8} {'bytes':>8}  source")
    for b, label, tok in rows:
        print(f"{tok:>8} {b:>8}  {label}")
    total_b = sum(r[0] for r in rows)
    total_t = sum(r[2] for r in rows)
    print(f"{total_t:>8} {total_b:>8}  total ({len(rows)} sources)")
    if not args.hooks:
        print("note: hook output not counted; rerun with --hooks to include SessionStart hooks")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
