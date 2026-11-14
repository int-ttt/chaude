#!/usr/bin/env python3
"""Token usage for one release cycle, this project only, from local Claude Code transcripts.

Usage: python3 -I scripts/release-usage.py --from <tag|ISO> --to <tag|ISO> [--out PATH]
  e.g. python3 -I scripts/release-usage.py --from v0.12.6 --to v0.12.7 --out notes/usage/0.12.7.md

Reads only this project's transcripts: <project dir>/*.jsonl and <project dir>/*/subagents/*.jsonl.
Local only (transcripts are not in CI).

What the numbers count:
- turns: unique assistant messages (message.id). Streamed duplicates of one message are merged
  (per-field max of usage, earliest timestamp).
- input, cache_creation, cache_read, output: raw API usage sums per token type. No USD, no weighting.
- total: input + cache_creation + cache_read + output (cache_read usually dominates it).
- 5h blocks: the first message after the previous block ends starts a new block; a block lasts 5h
  from its first message. Built from this cycle's messages only.
- weeks: 7-day windows from the cycle start; the last one may be partial.
- top agents: per block, sources ranked by total tokens; "main" is the top-level session.
"""
import argparse
import glob
import json
import math
import os
import re
import subprocess
import sys
from collections import defaultdict
from datetime import datetime, timedelta, timezone

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# Claude Code stores a project under ~/.claude/projects/<abs path with non-alphanumerics as "-">.
PROJECT_DIR = os.path.expanduser(
    os.path.join("~/.claude/projects", re.sub(r"[^A-Za-z0-9]", "-", REPO)))
BLOCK = timedelta(hours=5)
WEEK = timedelta(days=7)
FIELDS = ("input", "cache_creation", "cache_read", "output")
TS_RE = re.compile(
    r"^(\d{4})-(\d{2})-(\d{2})"
    r"(?:[T ](\d{2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?(Z|[+-]\d{2}:?\d{2})?)?$")


def parse_ts(text):
    """ISO 8601 date or date-time (Python 3.9 safe). Naive times are taken as local time."""
    m = TS_RE.match(text.strip())
    if not m:
        raise ValueError("not an ISO time and not an existing tag: %s" % text)
    y, mo, d = (int(x) for x in m.groups()[:3])
    h, mi, s = (int(x or 0) for x in m.groups()[3:6])
    micro = int((m.group(7) or "0")[:6].ljust(6, "0"))
    tz_text = m.group(8)
    if tz_text is None:
        tz = None
    elif tz_text == "Z":
        tz = timezone.utc
    else:
        digits = tz_text[1:].replace(":", "")
        sign = 1 if tz_text[0] == "+" else -1
        tz = timezone(sign * timedelta(hours=int(digits[:2]), minutes=int(digits[2:])))
    dt = datetime(y, mo, d, h, mi, s, micro, tzinfo=tz)
    return dt if tz else dt.astimezone()


def resolve(spec):
    """Return (aware datetime, label). A tag resolves to its creation time (tagger date for annotated tags)."""
    tag_ref = "refs/tags/" + spec
    found = subprocess.run(["git", "-C", REPO, "rev-parse", "-q", "--verify", tag_ref],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if found.returncode == 0:
        out = subprocess.run(
            ["git", "-C", REPO, "for-each-ref", "--format=%(creatordate:iso-strict)", tag_ref],
            capture_output=True, text=True, check=True).stdout.strip()
        return parse_ts(out), spec
    return parse_ts(spec), spec


def sources():
    for pattern, is_sub in (("*.jsonl", False), ("*/subagents/*.jsonl", True)):
        for path in sorted(glob.glob(os.path.join(PROJECT_DIR, pattern))):
            yield path, is_sub


def agent_label(path, is_sub):
    if not is_sub:
        return "main"
    try:
        with open(path[:-len(".jsonl")] + ".meta.json", encoding="utf-8") as f:
            agent_type = json.load(f).get("agentType")
        if agent_type:
            return agent_type
    except (OSError, ValueError, AttributeError):
        pass
    return "subagent:" + os.path.basename(path)[:-len(".jsonl")][-8:]


def as_int(value):
    return int(value) if isinstance(value, (int, float)) else 0


def collect():
    """Return (messages sorted by time, stats) with one entry per assistant message.id."""
    msgs = {}
    stats = {"files": 0, "sub_files": 0, "usage_lines": 0}
    for path, is_sub in sources():
        stats["files"] += 1
        stats["sub_files"] += int(is_sub)
        label = agent_label(path, is_sub)
        with open(path, encoding="utf-8", errors="replace") as f:
            for line in f:
                if '"usage"' not in line:
                    continue
                try:
                    rec = json.loads(line)
                except ValueError:
                    continue
                msg = rec.get("message")
                if rec.get("type") != "assistant" or not isinstance(msg, dict):
                    continue
                usage, mid, ts_text = msg.get("usage"), msg.get("id"), rec.get("timestamp")
                if not isinstance(usage, dict) or not mid or not ts_text:
                    continue
                stats["usage_lines"] += 1
                ts = parse_ts(ts_text)
                vals = {"input": as_int(usage.get("input_tokens")),
                        "cache_creation": as_int(usage.get("cache_creation_input_tokens")),
                        "cache_read": as_int(usage.get("cache_read_input_tokens")),
                        "output": as_int(usage.get("output_tokens"))}
                cur = msgs.get(mid)
                if cur is None:
                    msgs[mid] = dict(ts=ts, model=msg.get("model") or "unknown",
                                     agent=label, sub=is_sub, **vals)
                else:
                    cur["ts"] = min(cur["ts"], ts)
                    for k in FIELDS:
                        cur[k] = max(cur[k], vals[k])
    stats["unique_ids"] = len(msgs)
    return sorted(msgs.values(), key=lambda r: r["ts"]), stats


def new_acc():
    return dict(turns=0, **{k: 0 for k in FIELDS})


def add(acc, r):
    acc["turns"] += 1
    for k in FIELDS:
        acc[k] += r[k]


def total(acc):
    return sum(acc[k] for k in FIELDS)


def num(n):
    return "{:,}".format(n)


def local(dt):
    return dt.astimezone().strftime("%m-%d %H:%M")


def row(cells):
    return "| " + " | ".join(str(c) for c in cells) + " |"


def token_row(label, acc, extra=()):
    return row([label, num(acc["turns"]), num(acc["input"]), num(acc["cache_creation"]),
                num(acc["cache_read"]), num(acc["output"]), num(total(acc))] + list(extra))


TOKEN_HEAD = ["turns", "input", "cache_creation", "cache_read", "output", "total"]


def build_report(frm, to, from_label, to_label, recs, stats):
    out = []
    days = (to - frm).total_seconds() / 86400
    out.append("# Usage: %s -> %s" % (from_label, to_label))
    out.append("")
    out.append("- cycle: %s -> %s (%.2f days; times shown in local time)"
               % (frm.isoformat(timespec="seconds"), to.isoformat(timespec="seconds"), days))
    out.append("- source: local Claude Code transcripts of this project only (main + subagents); not in CI")
    out.append("- scanned: %d files (%d subagent), %d usage lines, %d unique message ids in the whole project history"
               % (stats["files"], stats["sub_files"], stats["usage_lines"], stats["unique_ids"]))
    out.append("- turns: unique assistant messages (message.id); streamed duplicates merged")
    out.append("- tokens: raw API usage sums per type (input, cache_creation, cache_read, output); "
               "total = their sum; not weighted; no USD; cache_read usually dominates total")
    out.append("- 5h blocks: first message after the previous block ends starts a new block; block = 5h from its first message; this cycle only")
    out.append("- weeks: 7-day windows from the cycle start; the last one may be partial")
    out.append("- top agents: per block, sources ranked by total tokens; main = top-level session")
    out.append("")

    main_acc, sub_acc, all_acc = new_acc(), new_acc(), new_acc()
    per_model = defaultdict(lambda: {"main": new_acc(), "sub": new_acc()})
    for r in recs:
        add(all_acc, r)
        add(sub_acc if r["sub"] else main_acc, r)
        add(per_model[r["model"]]["sub" if r["sub"] else "main"], r)

    out.append("## Cycle totals")
    out.append("")
    out.append(row(["source"] + TOKEN_HEAD))
    out.append(row(["---"] * (len(TOKEN_HEAD) + 1)))
    out.append(token_row("main", main_acc))
    out.append(token_row("subagents", sub_acc))
    out.append(token_row("**all**", all_acc))
    out.append("")
    out.append("### Per model")
    out.append("")
    out.append(row(["model"] + TOKEN_HEAD + ["of which subagents total"]))
    out.append(row(["---"] * (len(TOKEN_HEAD) + 2)))
    for model in sorted(per_model, key=lambda m: -total(per_model[m]["main"]) - total(per_model[m]["sub"])):
        both = new_acc()
        for part in ("main", "sub"):
            for k in FIELDS + ("turns",):
                both[k] += per_model[model][part][k]
        out.append(token_row(model, both, [num(total(per_model[model]["sub"]))]))
    out.append("")

    blocks = []
    for r in recs:
        if not blocks or r["ts"] >= blocks[-1]["end"]:
            blocks.append(dict(start=r["ts"], end=r["ts"] + BLOCK, acc=new_acc(), agents=defaultdict(int)))
        b = blocks[-1]
        add(b["acc"], r)
        b["agents"][r["agent"]] += sum(r[k] for k in FIELDS)

    def top3(b):
        ranked = sorted(b["agents"].items(), key=lambda kv: (-kv[1], kv[0]))[:3]
        return ", ".join("%s %s" % (name, num(tok)) for name, tok in ranked)

    out.append("## 5h blocks")
    out.append("")
    if not blocks:
        out.append("No assistant usage in this cycle.")
        out.append("")
    out.append(row(["#", "start", "end", "turns", "input", "cache_creation", "cache_read",
                    "output", "total", "top 3 agents (total tokens)"]))
    out.append(row(["---"] * 10))
    for i, b in enumerate(blocks, 1):
        a = b["acc"]
        out.append(row([i, local(b["start"]), local(b["end"]), num(a["turns"]), num(a["input"]),
                        num(a["cache_creation"]), num(a["cache_read"]), num(a["output"]),
                        num(total(a)), top3(b)]))
    out.append("")
    if blocks:
        peak_i = max(range(len(blocks)), key=lambda i: (total(blocks[i]["acc"]), -i))
        pb = blocks[peak_i]
        out.append("Peak 5h block: #%d, %s -> %s, total %s tokens, %s turns (top agents: %s)"
                   % (peak_i + 1, local(pb["start"]), local(pb["end"]), num(total(pb["acc"])),
                      num(pb["acc"]["turns"]), top3(pb)))
        out.append("")

    n_weeks = max(1, math.ceil((to - frm) / WEEK))
    weeks = [dict(start=frm + i * WEEK, end=min(frm + (i + 1) * WEEK, to), acc=new_acc())
             for i in range(n_weeks)]
    for r in recs:
        add(weeks[int((r["ts"] - frm) // WEEK)]["acc"], r)
    out.append("## Weeks (7-day windows from cycle start)")
    out.append("")
    out.append(row(["#", "start", "end"] + TOKEN_HEAD))
    out.append(row(["---"] * (len(TOKEN_HEAD) + 3)))
    for i, w in enumerate(weeks, 1):
        out.append(row([i, local(w["start"]), local(w["end"])] + [
            num(w["acc"]["turns"]), num(w["acc"]["input"]), num(w["acc"]["cache_creation"]),
            num(w["acc"]["cache_read"]), num(w["acc"]["output"]), num(total(w["acc"]))]))
    out.append("")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--from", dest="frm", required=True, help="start: tag (e.g. v0.12.6) or ISO time")
    ap.add_argument("--to", dest="to", required=True, help="end: tag (e.g. v0.12.7) or ISO time")
    ap.add_argument("--out", help="write markdown here (default: stdout)")
    args = ap.parse_args()

    try:
        frm, frm_label = resolve(args.frm)
        to, to_label = resolve(args.to)
    except (ValueError, subprocess.CalledProcessError) as e:
        sys.exit("release-usage: %s" % e)
    if to <= frm:
        sys.exit("release-usage: --to must be after --from")
    if not os.path.isdir(PROJECT_DIR):
        sys.exit("release-usage: no transcript dir for this project: %s" % PROJECT_DIR)

    recs, stats = collect()
    recs = [r for r in recs if frm <= r["ts"] < to]
    report = build_report(frm, to, frm_label, to_label, recs, stats)

    if args.out:
        out_dir = os.path.dirname(os.path.abspath(args.out))
        os.makedirs(out_dir, exist_ok=True)
        with open(args.out, "w", encoding="utf-8") as f:
            f.write(report)
        print("release-usage: wrote %s (%d messages in cycle)" % (args.out, len(recs)), file=sys.stderr)
    else:
        sys.stdout.write(report)


if __name__ == "__main__":
    main()
