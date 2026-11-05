#!/usr/bin/env python3
# Measure token use and approximate USD for the Rust-port period.
# Usage: python3 -I scripts/rust_port_usage.py [--start 2026-10-28] [--until 2026-11-05T15:00:00Z] [--root DIR] [--ledger FILE]
# Prices: notes/FACTS.md (docs.claude.com pricing, confirmed 2026-10-29), USD per MTok.
import argparse, glob, json, os, re
from collections import defaultdict

PRICES = {  # input, cache write 5m, cache write 1h, cache read, output
    "opus": (4.0, 5.0, 8.0, 0.20, 20.0),
    "sonnet": (2.0, 2.50, 4.0, 0.10, 10.0),
    "haiku_low": (0.10, 0.125, 0.20, 0.01, 0.50),   # context <= 100k
    "haiku_high": (0.50, 0.625, 1.0, 0.05, 2.50),   # context > 100k
}

def family(model):
    m = (model or "").lower()
    if "opus" in m: return "opus"
    if "sonnet" in m: return "sonnet"
    if "haiku" in m: return "haiku"
    return "other"

def cost(p_key, u):
    i, w5, w1, cr, out = PRICES[p_key]
    cw = u["cw5"] * w5 + u["cw1"] * w1
    return (u["in"] * i + cw + u["cr"] * cr + u["out"] * out) / 1e6

def haiku_key(u):
    ctx = u["in"] + u["cr"] + u["cw5"] + u["cw1"]
    return "haiku_high" if ctx > 100_000 else "haiku_low"

def load(root, start, until=""):
    files = sorted(glob.glob(os.path.join(root, "*.jsonl")) +
                   glob.glob(os.path.join(root, "*", "subagents", "*.jsonl")))
    seen = set()
    per = defaultdict(lambda: defaultdict(float))  # model-family -> counters
    sessions, agent_files, turns = set(), set(), 0
    rows = []  # per-request usage for pricing
    for f in files:
        is_sub = "subagents" in f
        had = False
        with open(f, encoding="utf-8") as fh:
            for line in fh:
                try:
                    r = json.loads(line)
                except ValueError:
                    continue
                if r.get("type") != "assistant":
                    continue
                ts = r.get("timestamp", "")
                if ts[:10] < start or (until and ts > until):
                    continue
                msg = r.get("message", {})
                if "usage" not in msg:
                    continue
                key = r.get("requestId") or r.get("uuid")
                if key in seen:
                    continue
                seen.add(key)
                u_raw = msg["usage"]
                cc = u_raw.get("cache_creation") or {}
                cw1 = cc.get("ephemeral_1h_input_tokens", 0)
                cw5 = cc.get("ephemeral_5m_input_tokens", 0)
                if not cc:
                    cw5 = u_raw.get("cache_creation_input_tokens", 0)
                u = {"in": u_raw.get("input_tokens", 0), "out": u_raw.get("output_tokens", 0),
                     "cr": u_raw.get("cache_read_input_tokens", 0), "cw5": cw5, "cw1": cw1}
                if not any(u.values()):
                    continue  # synthetic/empty usage rows
                fam = family(msg.get("model"))
                rows.append((fam, u))
                c = per[fam]
                for k in ("in", "out", "cr", "cw5", "cw1"):
                    c[k] += u[k]
                c["turns"] += 1
                turns += 1
                had = True
                if not is_sub:
                    sessions.add(r.get("sessionId"))
        if is_sub and had:
            agent_files.add(f)
    return per, rows, sessions, agent_files, turns

def ledger_counts(path, start):
    # record/LEDGER.md lines: "- MM-DD HH:MM [agent-name · model] ..."
    n = 0
    if not path or not os.path.exists(path):
        return None
    year = start[:4]
    for line in open(path, encoding="utf-8"):
        m = re.match(r"- (\d\d)-(\d\d) \d\d:\d\d \[([^\]]+)\]", line)
        if not m:
            continue
        if f"{year}-{m.group(1)}-{m.group(2)}" >= start:
            n += 1
    return n

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--start", default="2026-11-03")
    ap.add_argument("--root", default=os.path.expanduser(
        "~/.claude/projects/-Users-leekwangseok-PycharmProjects-claude-harness"))
    ap.add_argument("--until", default="", help="UTC timestamp cutoff, e.g. 2026-11-05T15:00:00Z")
    ap.add_argument("--ledger", default="record/LEDGER.md")
    a = ap.parse_args()

    per, rows, sessions, agent_files, turns = load(a.root, a.start, a.until)
    total = defaultdict(float)
    measured_usd = 0.0
    for fam, u in rows:
        pk = haiku_key(u) if fam == "haiku" else ("opus" if fam == "opus" else
             ("sonnet" if fam == "sonnet" else "opus"))
        measured_usd += cost(pk, u)
    # All-Opus estimate: same request token counts, all priced at Opus rates.
    opus_usd = 0.0
    for fam, u in rows:
        opus_usd += cost("opus", u)

    print(f"period_start: {a.start}")
    print(f"period_until: {a.until or 'open'}")
    print(f"sessions_main: {len(sessions)}")
    print(f"subagent_files: {len(agent_files)}")
    print(f"ledger_entries_since_start: {ledger_counts(a.ledger, a.start)}")
    print(f"assistant_requests: {turns}")
    for fam in sorted(per):
        c = per[fam]
        tok = {k: int(c[k]) for k in ("in", "cw5", "cw1", "cr", "out")}
        print(f"model {fam}: requests={int(c['turns'])} input={tok['in']} "
              f"cache_write_5m={tok['cw5']} cache_write_1h={tok['cw1']} "
              f"cache_read={tok['cr']} output={tok['out']}")
    all_tok = {k: sum(per[f][k] for f in per) for k in ("in", "cw5", "cw1", "cr", "out")}
    grand = sum(all_tok.values())
    print(f"total_tokens: {int(grand)} (input+cache_write+cache_read+output)")
    print(f"measured_usd_approx: {measured_usd:.2f}")
    print(f"all_opus_usd_approx: {opus_usd:.2f}")
    if opus_usd:
        print(f"saved_pct: {(1 - measured_usd / opus_usd) * 100:.1f}")

if __name__ == "__main__":
    main()
