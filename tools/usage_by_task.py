#!/usr/bin/env python3
"""Dev-only, read-only: token use per date / model / main-vs-subagent, plus per user request.

usage: usage_by_task.py [PROJECTS_DIR] [--from YYYY-MM-DD] [--to YYYY-MM-DD]
Scans PROJECTS_DIR/**/*.jsonl (default ~/.claude/projects), counts each API response once (message.id),
keeps messages whose timestamp (UTC date) is inside the range (default 2026-10-27..2026-10-28).
Subagent = file under a `subagents/` folder or isSidechain. Writes no files, uses no network.
Not shipped: excluded in tools/build_release.py and win/build_win.py; install.sh does not copy tools/."""
import argparse, glob, json, os, sys
from collections import defaultdict
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

ap = argparse.ArgumentParser()
ap.add_argument("root", nargs="?", default=os.path.expanduser("~/.claude/projects"))
ap.add_argument("--from", dest="d0", default="2026-10-27")
ap.add_argument("--to", dest="d1", default="2026-10-28")
a = ap.parse_args()


def fam(m):
    m = (m or "").lower()
    return next((f for f in ("opus", "sonnet", "haiku") if f in m), "other")


rows = defaultdict(lambda: defaultdict(int))    # (date, family, role) -> counters
seen, user_reqs, files, wt = set(), 0, 0, set()
for p in glob.glob(os.path.join(a.root, "**", "*.jsonl"), recursive=True):
    files += 1
    sub = f"{os.sep}subagents{os.sep}" in p
    proj = os.path.relpath(p, a.root).split(os.sep)[0]
    if "worktree" in proj.lower():
        wt.add(proj)
    with open(p, encoding="utf-8", errors="ignore") as f:
        for line in f:
            try:
                e = json.loads(line)
            except Exception:
                continue
            day = (e.get("timestamp") or "")[:10]
            if not (a.d0 <= day <= a.d1):
                continue
            msg = e.get("message") or {}
            if e.get("type") == "user" and not sub and not e.get("isMeta") and not e.get("isSidechain"):
                c = msg.get("content")
                if isinstance(c, list):
                    if any(isinstance(x, dict) and x.get("type") == "tool_result" for x in c):
                        continue
                    c = "".join(x.get("text", "") for x in c if isinstance(x, dict))
                if isinstance(c, str) and c.strip() and not c.lstrip().startswith("<"):
                    user_reqs += 1
                continue
            u = msg.get("usage")
            if e.get("type") != "assistant" or not u:
                continue
            mid = msg.get("id") or e.get("uuid")
            if mid in seen:
                continue
            seen.add(mid)
            r = rows[(day, fam(msg.get("model")), "sub" if sub or e.get("isSidechain") else "main")]
            r["req"] += 1
            r["in"] += (u.get("input_tokens") or 0) + (u.get("cache_creation_input_tokens") or 0) + (u.get("cache_read_input_tokens") or 0)
            r["out"] += u.get("output_tokens") or 0

print(f"range {a.d0}..{a.d1} (UTC date of message timestamp); files scanned {files}; unique responses {len(seen)}")
print(f"worktree project dirs found: {len(wt)} ({'included' if wt else 'none present'} in the scan)")
print(f"{'date':10} {'model':7} {'role':5} {'requests':>9} {'input+cache':>14} {'output':>10}")
tot = defaultdict(int)
for k in sorted(rows):
    r = rows[k]
    print(f"{k[0]:10} {k[1]:7} {k[2]:5} {r['req']:>9} {r['in']:>14} {r['out']:>10}")
    tot["req"] += r["req"]; tot["in"] += r["in"]; tot["out"] += r["out"]
    tot[k[1]] += r["req"]
print(f"total requests {tot['req']} (opus {tot['opus']}, sonnet {tot['sonnet']}, haiku {tot['haiku']}, other {tot['other']})")
print(f"user requests (main-thread text prompts): {user_reqs}")
if user_reqs:
    print(f"per user request: {tot['req'] / user_reqs:.1f} responses, input+cache {tot['in'] // user_reqs}, output {tot['out'] // user_reqs}")
