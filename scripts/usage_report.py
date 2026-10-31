#!/usr/bin/env python3
"""Summarize notes/metrics/*.jsonl: turn cost, classify cost, usage series, rate rows.

Stdlib only. Run: python3 -I scripts/usage_report.py [--since D] [--until D] [--session ID]
Row time: "ts" (RFC3339), else "time", else end_ms/1000. Rows without time count only without a window.
--since/--until: YYYY-MM-DD (local midnight; --until date includes that whole day) or ISO datetime (naive = local).
--session filters stage=turn rows only.
"""
import argparse
import glob
import json
import os
import sys
from datetime import datetime, timedelta, timezone


def parse_bound(s, is_until=False):
    try:
        if len(s) == 10:
            dt = datetime.strptime(s, "%Y-%m-%d").astimezone()  # naive -> local
            return dt + timedelta(days=1) if is_until else dt
        dt = datetime.fromisoformat(s.replace("Z", "+00:00"))
    except ValueError:
        raise argparse.ArgumentTypeError("bad date/datetime: %r" % s)
    return dt if dt.tzinfo else dt.astimezone()


def num(v):
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        return None
    return float(v)


def row_time(d):
    for k in ("ts", "time"):
        v = d.get(k)
        if isinstance(v, str):
            try:
                t = datetime.fromisoformat(v.replace("Z", "+00:00"))
            except ValueError:
                continue
            return t if t.tzinfo else t.astimezone()
    em = num(d.get("end_ms"))
    if em is not None:
        return datetime.fromtimestamp(em / 1000.0, tz=timezone.utc)
    return None


def load(dirpath):
    rows, bad = [], 0
    for path in sorted(glob.glob(os.path.join(dirpath, "*.jsonl"))):
        with open(path, encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    d = json.loads(line)
                except ValueError:
                    bad += 1
                    continue
                if not isinstance(d, dict):
                    bad += 1
                    continue
                rows.append(d)
    return rows, bad


def cost_at(t, turns):
    """Cumulative turn cost known at time t: per-session max cost_usd among turn rows with time <= t."""
    best = {}
    for tt, d, key in turns:
        c = num(d.get("cost_usd"))
        if tt is None or tt > t or c is None:
            continue
        best[key] = max(best.get(key, c), c)
    return sum(best.values())


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--metrics-dir", default="notes/metrics")
    ap.add_argument("--since", type=lambda s: parse_bound(s), default=None)
    ap.add_argument("--until", type=lambda s: parse_bound(s, True), default=None)
    ap.add_argument("--session", default=None)
    args = ap.parse_args(argv)

    if not os.path.isdir(args.metrics_dir):
        print("metrics dir not found: %s" % args.metrics_dir, file=sys.stderr)
        return 2

    rows, bad = load(args.metrics_dir)
    windowed = args.since is not None or args.until is not None
    timed = [(row_time(d), d) for d in rows]
    if windowed:
        timed = [(t, d) for t, d in timed if t is not None
                 and (args.since is None or t >= args.since)
                 and (args.until is None or t < args.until)]

    # Full (unwindowed) turn set, needed for cumulative baselines before the window start.
    all_turns = []
    for d in rows:
        if d.get("stage") == "turn" and (args.session is None or d.get("session") == args.session):
            all_turns.append((row_time(d), d, d.get("session") or id(d)))

    sel = timed
    if not sel:
        print("no rows in window")
        print("bad_lines: %d" % bad)
        return 0

    print("rows in window: %d" % len(sel))

    # (a) turn cost
    per_sess, naive, n_cost, turn_sum, n_turn = {}, 0.0, 0, 0.0, 0
    for t, d in sel:
        if d.get("stage") != "turn":
            continue
        if args.session is not None and d.get("session") != args.session:
            continue
        c = num(d.get("cost_usd"))
        if c is not None:
            naive += c
            n_cost += 1
            k = d.get("session") or id(d)
            per_sess[k] = max(per_sess.get(k, c), c)
        ct = num(d.get("cost_turn_usd"))
        if ct is not None:
            turn_sum += ct
            n_turn += 1
    print("== turn cost ==")
    print("rows with cost_usd: %d" % n_cost)
    print("per-session max cost_usd sum: %.4f" % sum(per_sess.values()))
    print("naive cost_usd sum: %.4f" % naive)
    print("cost_turn_usd sum: %.4f (rows: %d)" % (turn_sum, n_turn))

    # (b) classify cost
    cls_sum, cls_n = 0.0, 0
    for t, d in sel:
        if d.get("stage") == "classify":
            c = num(d.get("cost_usd"))
            if c is not None:
                cls_sum += c
                cls_n += 1
    print("== classify cost ==")
    print("cost_usd sum: %.4f (rows: %d)" % (cls_sum, cls_n))

    # (c) usage series
    pts = []
    for t, d in sel:
        if d.get("stage") != "usage" or "error" in d or t is None:
            continue
        sp = num(d.get("session_pct"))
        if sp is None:
            continue
        pts.append((t, sp, num(d.get("week_pct"))))
    pts.sort(key=lambda p: p[0])
    print("== usage series ==")
    if not pts:
        print("(no usage rows)")
    for t, sp, wp in pts:
        print("%s  session_pct=%g  week_pct=%s" % (
            t.astimezone().isoformat(timespec="seconds"), sp, "-" if wp is None else "%g" % wp))
    if pts:
        seg_start = 0
        for i in range(1, len(pts)):
            if pts[i][1] < pts[i - 1][1]:  # 5h window reset: start a new monotonic segment
                seg_start = i
        seg = pts[seg_start:]
        if len(seg) < 2:
            print("usd_per_pct: n/a (segment has %d point(s))" % len(seg))
        else:
            dp = seg[-1][1] - seg[0][1]
            if dp == 0:
                print("usd_per_pct: n/a (delta_pct 0)")
            else:
                dc = cost_at(seg[-1][0], all_turns) - cost_at(seg[0][0], all_turns)
                print("last segment: %d points, delta_pct=%g, delta_usd=%.4f, usd_per_pct=%.4f" % (
                    len(seg), dp, dc, dc / dp))

    # (d) rate rows
    rate_n = sum(1 for _, d in sel if d.get("stage") == "rate")
    raw_keys = set()
    for _, d in sel:
        if d.get("stage") == "rate_raw" and isinstance(d.get("keys"), list):
            raw_keys.update(str(k) for k in d["keys"])
    print("== rate ==")
    print("stage=rate rows: %d" % rate_n)
    print("rate_raw keys: %s" % (", ".join(sorted(raw_keys)) if raw_keys else "-"))

    # (e) subagent spend of turn rows (missing keys count as 0)
    sub_keys = ["input_tokens", "output_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"]
    sub_sum = {k: 0.0 for k in sub_keys}
    sub_msgs, spawns, spawn_types, by_source = 0.0, 0, {}, {}
    for _, d in sel:
        if d.get("stage") != "turn":
            continue
        src = d.get("source") or "(none)"
        by_source[src] = by_source.get(src, 0) + 1
        sub_msgs += num(d.get("sub_msgs")) or 0.0
        for k in sub_keys:
            sub_sum[k] += num(d.get("sub_" + k)) or 0.0
        spawns += int(num(d.get("spawns")) or 0)
        st = d.get("spawn_types")
        if isinstance(st, dict):
            for k, v in st.items():
                spawn_types[str(k)] = spawn_types.get(str(k), 0) + int(num(v) or 0)
    print("== subagent spend ==")
    print("sub_msgs sum: %d" % int(sub_msgs))
    for k in sub_keys:
        print("sub_%s sum: %d" % (k, int(sub_sum[k])))
    print("spawns sum: %d" % spawns)
    print("spawn_types: %s" % (", ".join("%s=%d" % kv for kv in sorted(spawn_types.items())) if spawn_types else "-"))
    print("row counts per source: %s" % (", ".join("%s=%d" % kv for kv in sorted(by_source.items())) if by_source else "-"))

    # (f) cost_turn_usd consistency: per-session chain of cumulative cost_usd, counted on rows in the window
    def in_window(t):
        return (args.since is None or t >= args.since) and (args.until is None or t < args.until)

    chain = {}
    for d in rows:
        if d.get("stage") != "turn" or (args.session is not None and d.get("session") != args.session):
            continue
        c, ct, t = num(d.get("cost_usd")), num(d.get("cost_turn_usd")), row_time(d)
        if c is None or ct is None or t is None:
            continue
        chain.setdefault(d.get("session"), []).append((t, c, ct))
    n_rows = mismatched = resets = 0
    for key, items in chain.items():
        items.sort(key=lambda x: x[0])
        prev = None
        for t, c, ct in items:
            if prev is None:
                expected = c
            elif c < prev:
                expected = c
                if in_window(t):
                    resets += 1
            else:
                expected = c - prev
            prev = c
            if windowed and not in_window(t):
                continue
            n_rows += 1
            if abs(ct - expected) > 1e-6:
                mismatched += 1
    print("== cost_turn check ==")
    print("cost_turn check: rows=%d mismatched=%d resets=%d" % (n_rows, mismatched, resets))

    print("bad_lines: %d" % bad)
    return 0


if __name__ == "__main__":
    sys.exit(main())
