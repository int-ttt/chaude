#!/usr/bin/env python3
"""PostToolUse hook (Agent|Task): checks that a caveman-format return (docs/claude-lean/CAVEMAN_FORMAT.md)
has its required keys. Never blocks: always exit 0. CAVEMAN_GUARD: log (metrics row only) | warn (default: row + additionalContext) | off."""
import json, os, re, sys, time
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # also when run with python -I
try:
    from _lean import hook_log
except Exception:  # logging is optional: a missing or broken _lean must not break the hook
    def hook_log(*_a, **_k):
        pass
from _lean import lang
hook_log("caveman-format")

# watched agents (plan:174; manage the list here only)
TARGETS = {"haiku-scout", "haiku-researcher", "haiku-coder", "haiku-batch-worker", "haiku-runner", "haiku-checker", "haiku-drafter", "sonnet-writer", "opus-architect", "haiku-output", "haiku-prompter", "haiku-extractor", "haiku-test-lead"}  # S4: extend here only
REQUIRED = {                                    # 3.2~3.4 keys (status header 3.6 is checked for every fmt)
    "R-read": ["claim", "searched", "not_found", "not_checked", "truncated"],
    "R-write": ["changed", "diffstat", "check", "ws"],
    "R-decide": ["decision", "why", "rejected", "risks", "open_q", "plan_path"],
}
STATUS_KEYS = ["status", "reason", "relay"]  # 3.6 head + 3.7 relay, every R-* return
STATUS_VALUES = {"done", "partial", "fail", "blocked"}
B_KEYS = ("task", "paths", "limits", "return", "lang")  # plan:160 required B-format keys


def resp_text(r):
    if isinstance(r, str):
        return r
    if isinstance(r, dict):
        if isinstance(r.get("content"), (list, str)):
            return resp_text(r["content"])
        return r["text"] if isinstance(r.get("text"), str) else ""
    if isinstance(r, list):
        parts = []
        for x in r:
            if isinstance(x, str):
                parts.append(x)
            elif isinstance(x, dict) and isinstance(x.get("text"), str):
                parts.append(x["text"])
        return "\n".join(parts)
    return ""


def value_of(text, key):
    m = re.search(rf"^\s*{key}:[ \t]*(.*)$", text, re.M)
    return m.group(1).strip() if m else None


def check(text, fmt):
    missing = []
    for key in STATUS_KEYS + REQUIRED[fmt]:
        if not re.search(rf"^\s*{key}:", text, re.M):
            missing.append(key)
    if "status" not in missing:
        st = (value_of(text, "status") or "").split()
        if not st or st[0] not in STATUS_VALUES:
            missing.append("status(invalid)")
    if fmt == "R-read":
        if "truncated" not in missing:
            tr = (value_of(text, "truncated") or "").split(";")[0].strip()
            if tr not in ("yes", "no"):
                missing.append("truncated(invalid)")
        claims = re.findall(r"^\s*claim:.*$", text, re.M)
        if claims and not all(re.search(r"^\s*claim:\s*\S+:\d+.*\|", c) for c in claims):
            missing.append("claim(invalid)")
    return missing


def check_jondaetmal(text):
    if lang() == "en":  # language en (settings or CHAUDE_LANG): plain English replies, no 존댓말 rule
        return []
    text = re.sub(r"```.*?```", "", text, flags=re.S)  # skip code blocks
    for s in re.split(r"[.!?\n]+", text):
        s = s.strip()
        # Korean plain ending (-다, not -니다) = 존댓말 violation (haiku-output replies are polite form)
        if s and s[-1] == "다" and (len(s) < 2 or s[-2] != "니"):
            return ["존댓말"]
    return []


def check_caveman_leak(text):                   # S2: caveman return keys leaked into a haiku-output reply
    text = re.sub(r"```.*?```", "", text, flags=re.S)
    if re.search(r"^\s*(status|reason|claim|searched|not_found|not_checked|truncated|omitted|relay):\s", text, re.M):
        return ["caveman_leak"]
    return []


def check_brief(prompt):
    return [f"brief:{k}" for k in B_KEYS if value_of(prompt, k) is None]


def main():
    data = json.load(sys.stdin)
    hook_log(data=data)
    if not isinstance(data, dict) or data.get("tool_name") not in ("Agent", "Task"):
        return
    tin = data.get("tool_input") or {}
    agent = tin.get("subagent_type") or ""
    if agent not in TARGETS:
        return
    prompt = tin.get("prompt") or ""
    if not isinstance(prompt, str):
        return
    resp = resp_text(data.get("tool_response"))
    m = re.search(r"^return:\s*(R-read|R-write|R-decide)", prompt, re.M)
    if agent == "haiku-output":                    # polite-form check only; return: filter ignored (plan:160)
        fmt, missing = "tui", check_jondaetmal(resp) + check_caveman_leak(resp)
    elif agent == "haiku-prompter":             # brief: B-format key check; polite-form check skipped (plan:160)
        fmt, missing = "prompter", check_brief(prompt)
        if m:
            missing += check(resp, m.group(1))
    elif not m:
        return                                  # legacy brief: not checked
    else:
        fmt = m.group(1)
        missing = check(resp, fmt)
    mode = os.environ.get("CAVEMAN_GUARD", "warn")
    if mode not in ("log", "warn", "off"):
        mode = "warn"
    if mode == "off" or not missing:
        return
    root = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    now = time.gmtime()
    row = {"ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", now), "hook": "caveman-format",
           "agent": agent, "fmt": fmt, "missing": missing}
    try:
        d = os.path.join(os.environ.get("HARNESS_RECORD_DIR") or os.path.join(root, "record"), "metrics")
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, time.strftime("%Y-%m-%d", now) + ".jsonl"), "a", encoding="utf-8") as f:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    except OSError:
        pass
    if mode == "warn":
        out = {"hookSpecificOutput": {"hookEventName": "PostToolUse",
               "additionalContext": f"missing required fields: {', '.join(missing)}, re-request"}}
        hook_log(decision="inject")
        print(json.dumps(out, ensure_ascii=False))


try:
    main()
except Exception:
    pass
sys.exit(0)
