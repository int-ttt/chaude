#!/usr/bin/env python3
"""Sum real token usage from Claude Code transcripts (~/.claude/projects/**.jsonl).

usage: usage_report.py [PROJECTS_DIR] [--top N]

Per session (main + its subagents): API requests, peak context, tokens by kind,
and an API list-price estimate. Prices are USD per 1M tokens; edit PRICES if they change.
"""
import argparse, glob, json, os, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
from collections import defaultdict

# input, cache write (1h), cache read, output — per 1M tokens
PRICES = {
    "opus": (4.0, 8.0, 0.20, 20.0),
    "sonnet": (2.0, 4.0, 0.20, 10.0),
    "haiku": (0.10, 0.20, 0.01, 0.50),
}


def family(model):
    m = (model or "").lower()
    return next((f for f in PRICES if f in m), "opus")


def scan(path, agg, seen):
    with open(path, encoding="utf-8", errors="ignore") as f:
        for line in f:
            try:
                e = json.loads(line)
            except Exception:
                continue
            msg = e.get("message") or {}
            u = msg.get("usage")
            if e.get("type") != "assistant" or not u:
                continue
            mid = msg.get("id") or e.get("uuid")
            if mid in seen:              # one API response can span several transcript lines
                continue
            seen.add(mid)
            fam = family(msg.get("model"))
            inp = u.get("input_tokens") or 0
            cw = u.get("cache_creation_input_tokens") or 0
            cr = u.get("cache_read_input_tokens") or 0
            out = u.get("output_tokens") or 0
            a = agg[fam]
            a["req"] += 1
            a["in"] += inp; a["cw"] += cw; a["cr"] += cr; a["out"] += out
            a["peak"] = max(a["peak"], inp + cw + cr)


def cost(fam, a):
    p = PRICES[fam]
    return (a["in"] * p[0] + a["cw"] * p[1] + a["cr"] * p[2] + a["out"] * p[3]) / 1e6


ap = argparse.ArgumentParser()
ap.add_argument("root", nargs="?", default=os.path.expanduser("~/.claude/projects"))
ap.add_argument("--top", type=int, default=15)
args = ap.parse_args()

sessions = {}
for main in glob.glob(os.path.join(args.root, "*", "*.jsonl")):
    sid = os.path.splitext(os.path.basename(main))[0]
    agg = defaultdict(lambda: defaultdict(int))
    seen = set()
    scan(main, agg, seen)
    for sub in glob.glob(os.path.join(os.path.dirname(main), sid, "subagents", "*.jsonl")):
        scan(sub, agg, seen)
    if agg:
        sessions[(os.path.basename(os.path.dirname(main)), sid)] = agg

rows = []
for (proj, sid), agg in sessions.items():
    total = sum(cost(f, a) for f, a in agg.items())
    rows.append((total, proj, sid, agg))
rows.sort(reverse=True)

grand = defaultdict(lambda: defaultdict(int))
for total, proj, sid, agg in rows:
    for f, a in agg.items():
        for k, v in a.items():
            grand[f][k] = max(grand[f][k], v) if k == "peak" else grand[f][k] + v

M = lambda n: f"{n / 1e6:.1f}M"
print("== 전체 (모델별) ==")
for f, a in grand.items():
    c = cost(f, a)
    parts = {"읽기(캐시)": a["cr"] * PRICES[f][2], "저장(캐시)": a["cw"] * PRICES[f][1],
             "입력": a["in"] * PRICES[f][0], "출력·생각": a["out"] * PRICES[f][3]}
    share = ", ".join(f"{k} {v / 1e6 / c * 100:.0f}%" for k, v in parts.items()) if c else ""
    print(f"{f:7s} 요청 {a['req']:>6}  캐시읽기 {M(a['cr'])}  캐시저장 {M(a['cw'])}  출력 {M(a['out'])}  "
          f"최대맥락 {a['peak'] // 1000}k  ≈ ${c:,.2f}  ({share})")

print(f"\n== 비용 상위 세션 {args.top}개 ==")
for total, proj, sid, agg in rows[: args.top]:
    req = sum(a["req"] for a in agg.values())
    peak = max(a["peak"] for a in agg.values())
    cr = sum(a["cr"] for a in agg.values())
    print(f"${total:8.2f}  요청 {req:>5}  최대맥락 {peak // 1000:>4}k  요청당 평균읽기 {cr / max(req, 1) / 1000:>5.0f}k  "
          f"{proj[-40:]} / {sid[:8]}")
