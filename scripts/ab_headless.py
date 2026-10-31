#!/usr/bin/env python3
"""A/B runner for headless `claude -p` calls (stdlib only).

Subcommands:
  run       --spec SPEC.json --order A,B,... --out RESULT.jsonl [--warmup A,B]
  report    RESULT.jsonl --a A --b B [--json]
  selftest
"""
import argparse
import hashlib
import itertools
import json
import math
import re
import subprocess
import sys
from datetime import datetime, timezone

MODEL_KEYS = ("inputTokens", "outputTokens", "cacheReadInputTokens",
              "cacheCreationInputTokens", "costUSD")
MODEL_REPORT_KEYS = ("cacheCreationInputTokens", "cacheReadInputTokens", "costUSD")
MAX_BUDGET_USD = "1.5"
TIMEOUT_S = 900


def split_list(s):
    return [x.strip() for x in s.split(",") if x.strip()] if s else []


def sha256_text(s):
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def model_usage(raw):
    out = {}
    if isinstance(raw, dict):
        for model, u in raw.items():
            u = u if isinstance(u, dict) else {}
            out[model] = {k: (u.get(k) or 0) for k in MODEL_KEYS}
    return out


def build_argv(arm, default_budget):
    budget = arm.get("max_budget_usd", default_budget)
    return ["claude", "-p", "--model", arm.get("model", "haiku"),
            "--output-format", "json", "--no-session-persistence",
            "--max-budget-usd", str(budget),
            *arm.get("extra_args", [])]


def run_one(label, arm, idx, warmup, expected, default_budget=MAX_BUDGET_USD):
    cmd = build_argv(arm, default_budget)
    row = {"ts": datetime.now(timezone.utc).isoformat(), "arm": label, "idx": idx,
           "warmup": warmup, "rc": None, "is_error": False, "total_cost_usd": 0,
           "num_turns": 0, "duration_ms": 0, "result_sha256": None,
           "result_head": "", "correct": False, "modelUsage": {}}
    try:
        # prompt goes via stdin, never argv (variadic --allowedTools would swallow it)
        p = subprocess.run(cmd, cwd=arm["cwd"], input=arm["prompt"], capture_output=True,
                           text=True, timeout=TIMEOUT_S)
    except subprocess.TimeoutExpired:
        row.update(rc=-9, is_error=True, stderr_head="timeout after %ds" % TIMEOUT_S)
        return row
    row["rc"] = p.returncode
    try:
        d = json.loads(p.stdout)
        if not isinstance(d, dict):
            raise ValueError("stdout JSON is not an object")
    except ValueError:
        row.update(is_error=True, stderr_head=(p.stderr or "")[:500])
        return row
    text = str(d.get("result") or "").strip()
    is_error = bool(d.get("is_error", False))
    row.update(is_error=is_error,
               total_cost_usd=float(d.get("total_cost_usd") or 0),
               num_turns=int(d.get("num_turns") or 0),
               duration_ms=int(d.get("duration_ms") or 0),
               result_sha256=sha256_text(text),
               result_head=text[:200],
               correct=(not is_error) and re.fullmatch(expected, text) is not None,
               modelUsage=model_usage(d.get("modelUsage")))
    return row


def cmd_run(args):
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    arms = spec["arms"]
    expected = spec["expected"]
    default_budget = spec.get("max_budget_usd", MAX_BUDGET_USD)
    re.compile(expected)  # fail early on a bad pattern
    try:
        ver = subprocess.run(["claude", "--version"], capture_output=True,
                             text=True, timeout=30)
        claude_version = ver.stdout.strip() if ver.returncode == 0 else "unknown"
    except Exception:
        claude_version = "unknown"
    meta = {"claude_version": claude_version, "expected": expected,
            "spec": args.spec, "order": args.order,
            "ts": datetime.now(timezone.utc).isoformat()}
    with open(args.out + ".meta.json", "w", encoding="utf-8") as mf:
        json.dump(meta, mf, ensure_ascii=False, indent=2)
    order = split_list(args.order)
    warm = split_list(args.warmup)
    for lab in order + warm:
        if lab not in arms:
            sys.exit("unknown arm label: %s" % lab)
    seq = [(lab, True) for lab in warm] + [(lab, False) for lab in order]
    counts = {}
    with open(args.out, "a", encoding="utf-8") as out:
        for lab, is_warm in seq:
            if is_warm:
                idx = 0
            else:
                counts[lab] = counts.get(lab, 0) + 1
                idx = counts[lab]
            row = run_one(lab, arms[lab], idx, is_warm, expected, default_budget)
            out.write(json.dumps(row, ensure_ascii=False) + "\n")
            out.flush()
            print("%s %d %.4f %s" % (lab, idx, row["total_cost_usd"], row["correct"]),
                  flush=True)


def stats(vals):
    n = len(vals)
    if n == 0:
        return {"n": 0, "mean": None, "sd": None, "min": None, "max": None}
    mean = sum(vals) / n
    sd = math.sqrt(sum((x - mean) ** 2 for x in vals) / (n - 1)) if n >= 2 else 0.0
    return {"n": n, "mean": mean, "sd": sd, "min": min(vals), "max": max(vals)}


def midranks(vals):
    order = sorted(range(len(vals)), key=lambda i: vals[i])
    ranks = [0.0] * len(vals)
    i = 0
    while i < len(order):
        j = i
        while j + 1 < len(order) and vals[order[j + 1]] == vals[order[i]]:
            j += 1
        avg = (i + 1 + j + 1) / 2.0
        for k in range(i, j + 1):
            ranks[order[k]] = avg
        i = j + 1
    return ranks


def mw_exact_p(a, b):
    """Exact two-sided Mann-Whitney U p-value (midranks, all splits enumerated)."""
    na, nb = len(a), len(b)
    if na == 0 or nb == 0:
        return None
    pooled = list(a) + list(b)
    ranks = midranks(pooled)
    base = na * (na + 1) / 2.0
    mu = na * nb / 2.0
    d_obs = abs((sum(ranks[:na]) - base) - mu)
    hits = total = 0
    for combo in itertools.combinations(range(len(pooled)), na):
        u = sum(ranks[i] for i in combo) - base
        total += 1
        if abs(u - mu) >= d_obs - 1e-12:
            hits += 1
    return hits / total


def summarize(rows, label):
    rs = [r for r in rows if r.get("arm") == label and not r.get("warmup")]
    costs = [float(r.get("total_cost_usd") or 0) for r in rs]
    per_model = {}
    total_cc = 0
    models = set()
    for r in rs:
        models.update((r.get("modelUsage") or {}).keys())
    for m in sorted(models):
        per_model[m] = {}
        for k in MODEL_REPORT_KEYS:
            vals = [float(((r.get("modelUsage") or {}).get(m) or {}).get(k) or 0) for r in rs]
            per_model[m][k] = sum(vals) / len(vals) if vals else None
    for r in rs:
        for u in (r.get("modelUsage") or {}).values():
            total_cc += float((u or {}).get("cacheCreationInputTokens") or 0)
    turns = [float(r.get("num_turns") or 0) for r in rs]
    return {
        "arm": label,
        "n": len(rs),
        "errors": sum(1 for r in rs if r.get("is_error")),
        "correct": sum(1 for r in rs if r.get("correct")),
        "unique_result_sha256": len({r["result_sha256"] for r in rs if r.get("result_sha256")}),
        "cost_usd": stats(costs),
        "num_turns_mean": (sum(turns) / len(turns)) if turns else None,
        "per_model_mean": per_model,
        "total_cache_creation_tokens": total_cc,
        "_costs": costs,
    }


def ratio(b, a):
    if b is None or a is None or a == 0:
        return None
    return b / a


def load_rows(path):
    rows = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def fmt(x):
    return "n/a" if x is None else ("%.6g" % x)


def cmd_report(args):
    rows = load_rows(args.file)
    sa = summarize(rows, args.a)
    sb = summarize(rows, args.b)
    result = {
        "a": sa, "b": sb,
        "ratio_b_over_a": {
            "cost_mean": ratio(sb["cost_usd"]["mean"], sa["cost_usd"]["mean"]),
            "cache_creation_total": ratio(sb["total_cache_creation_tokens"],
                                          sa["total_cache_creation_tokens"]),
        },
        "mannwhitney_p_cost": mw_exact_p(sa["_costs"], sb["_costs"]),
    }
    for s in (sa, sb):
        del s["_costs"]
    if args.json:
        print(json.dumps(result, ensure_ascii=False))
        return
    for s in (sa, sb):
        c = s["cost_usd"]
        print("arm %s: n=%d errors=%d correct=%d unique_sha=%d"
              % (s["arm"], s["n"], s["errors"], s["correct"], s["unique_result_sha256"]))
        print("  cost_usd mean=%s sd=%s min=%s max=%s"
              % (fmt(c["mean"]), fmt(c["sd"]), fmt(c["min"]), fmt(c["max"])))
        print("  num_turns mean=%s" % fmt(s["num_turns_mean"]))
        print("  total_cache_creation_tokens=%s" % fmt(s["total_cache_creation_tokens"]))
        for m, vals in s["per_model_mean"].items():
            print("  model %s mean: cacheCreation=%s cacheRead=%s costUSD=%s"
                  % (m, fmt(vals["cacheCreationInputTokens"]),
                     fmt(vals["cacheReadInputTokens"]), fmt(vals["costUSD"])))
    r = result["ratio_b_over_a"]
    print("ratio b/a: cost_mean=%s cache_creation_total=%s"
          % (fmt(r["cost_mean"]), fmt(r["cache_creation_total"])))
    print("mann-whitney exact two-sided p (total_cost_usd)=%s"
          % fmt(result["mannwhitney_p_cost"]))


def cmd_selftest(_args):
    fails = []
    p1 = mw_exact_p([1, 2, 3, 4, 5], [6, 7, 8, 9, 10])
    if p1 is None or abs(p1 - 2 / 252) >= 1e-9:
        fails.append("p(1..5 vs 6..10)=%r expected %r" % (p1, 2 / 252))
    p2 = mw_exact_p([1] * 5, [1] * 5)
    if p2 != 1.0:
        fails.append("p(identical 5v5)=%r expected 1.0" % (p2,))
    dummy_prompt = "SELFTEST_PROMPT_SENTINEL"
    dummy = {"model": "haiku", "cwd": ".", "prompt": dummy_prompt,
             "extra_args": ["--allowedTools", "Read"]}
    argv = build_argv(dummy, MAX_BUDGET_USD)
    if any(dummy_prompt in a for a in argv):
        fails.append("prompt leaked into argv: %r" % (argv,))
    if "--max-budget-usd" not in argv:
        fails.append("argv missing --max-budget-usd: %r" % (argv,))
    if fails:
        for f in fails:
            print("selftest FAIL: " + f, file=sys.stderr)
        sys.exit(1)
    print("selftest ok")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    pr = sub.add_parser("run", help="run A/B arms and append jsonl rows")
    pr.add_argument("--spec", required=True)
    pr.add_argument("--order", required=True, help="comma list of arm labels")
    pr.add_argument("--out", required=True)
    pr.add_argument("--warmup", default="", help="comma list of warmup arm labels")
    pr.set_defaults(func=cmd_run)

    prep = sub.add_parser("report", help="summarize an A/B jsonl file")
    prep.add_argument("file")
    prep.add_argument("--a", required=True)
    prep.add_argument("--b", required=True)
    prep.add_argument("--json", action="store_true")
    prep.set_defaults(func=cmd_report)

    ps = sub.add_parser("selftest", help="check the exact Mann-Whitney routine")
    ps.set_defaults(func=cmd_selftest)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
