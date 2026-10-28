#!/usr/bin/env python3
"""Search this project's past Claude Code conversations (current + archived) without loading raw transcripts.

usage: search_history.py "<terms>" [--project DIR] [--since YYYY-MM-DD] [--who user|claude|all] [--limit 12]

All terms must appear in a message (case-insensitive). Searches only user messages and Claude's visible
replies (no tool output, no thinking), including text that auto-compaction has since summarized away.
Looks in ~/.claude/projects/<project>/ and ~/claude-archive/<project-name>-*/.
"""
import argparse, glob, gzip, json, os, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

ap = argparse.ArgumentParser()
ap.add_argument("terms")
ap.add_argument("--project", default=os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
ap.add_argument("--since", default="")
ap.add_argument("--who", choices=["user", "claude", "all"], default="all")
ap.add_argument("--limit", type=int, default=12)
ap.add_argument("--width", type=int, default=300)
args = ap.parse_args()

from claude_history import history_dir

proj = os.path.abspath(os.path.expanduser(args.project))
home = os.path.expanduser("~")
dirs = [d for d in [history_dir(proj)] if d]
dirs += glob.glob(os.path.join(home, "claude-archive", os.path.basename(proj) + "-*"))
files = [f for d in dirs for pat in ("*.jsonl", "*.jsonl.gz") for f in glob.glob(os.path.join(d, pat))]   # archives are gzipped
if not files:
    raise SystemExit(f"대화 기록 없음: {proj}")

terms = [t.lower() for t in args.terms.split() if t]
NOISE = re.compile(r"<(command-name|command-message|command-args|local-command-stdout|system-reminder)>.*?</\1>", re.S)


def text_of(content):
    if isinstance(content, str):
        return content
    if not isinstance(content, list) or any(isinstance(c, dict) and c.get("type") == "tool_result" for c in content):
        return ""
    return "\n".join(c.get("text", "") for c in content if isinstance(c, dict) and c.get("type") == "text")


hits = []
for path in files:
    sid = os.path.basename(path)[:8]
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt", encoding="utf-8", errors="ignore") as f:
        for line in f:
            if not all(t in line.lower() for t in terms):      # cheap pre-filter on the raw line
                continue
            try:
                e = json.loads(line)
            except Exception:
                continue
            kind = e.get("type")
            if kind not in ("user", "assistant") or e.get("isMeta"):
                continue
            if args.who == "user" and kind != "user" or args.who == "claude" and kind != "assistant":
                continue
            ts = (e.get("timestamp") or "")[:16].replace("T", " ")
            if args.since and ts[:10] < args.since:
                continue
            t = " ".join(NOISE.sub("", text_of((e.get("message") or {}).get("content"))).split())
            low = t.lower()
            if not t or not all(x in low for x in terms):
                continue
            i = low.find(terms[0])
            start = max(0, i - args.width // 3)
            snip = ("…" if start else "") + t[start:start + args.width] + ("…" if start + args.width < len(t) else "")
            who = "압축요약" if e.get("isCompactSummary") else ("사용자" if kind == "user" else "Claude")
            hits.append((ts, sid, who, snip))

hits.sort(key=lambda h: h[0], reverse=True)
print(f"'{args.terms}' 일치 {len(hits)}건 (최신순 {min(len(hits), args.limit)}건 표시, 세션 파일 {len(files)}개)")
for ts, sid, who, snip in hits[: args.limit]:
    print(f"- {ts} [{sid}] {who}: {snip}")
