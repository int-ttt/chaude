#!/usr/bin/env python3
"""PostToolUse hook (Agent|Task): checks that a caveman-format return (docs/claude-lean/CAVEMAN_FORMAT.md)
has its required keys. Never blocks: always exit 0. CAVEMAN_GUARD: log (default, metrics row only) | warn (row + additionalContext) | off."""
import json, os, re, sys, time
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

TARGETS = {"haiku-scout", "haiku-researcher"}  # S4: extend here only
REQUIRED = {                                    # 3.2~3.4 keys (status header 3.6 is checked for every fmt)
    "R-read": ["claim", "searched", "not_found", "not_checked", "truncated"],
    "R-write": ["changed", "diffstat", "check", "ws"],
    "R-decide": ["decision", "why", "rejected", "risks", "open_q", "plan_path"],
}
STATUS_KEYS = ["status", "reason"]
STATUS_VALUES = {"done", "partial", "fail", "blocked"}


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


def check(prompt, text, fmt):
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
    if re.search(r"^lang:\s*ko\b", prompt, re.M) and not re.search(r"^\s*summary_ko:", text, re.M):
        missing.append("summary_ko")
    return missing


def main():
    data = json.load(sys.stdin)
    if not isinstance(data, dict) or data.get("tool_name") not in ("Agent", "Task"):
        return
    tin = data.get("tool_input") or {}
    agent = tin.get("subagent_type") or ""
    if agent not in TARGETS:
        return
    prompt = tin.get("prompt") or ""
    if not isinstance(prompt, str):
        return
    m = re.search(r"^return:\s*(R-read|R-write|R-decide)", prompt, re.M)
    if not m:
        return                                  # legacy brief: not checked
    fmt = m.group(1)
    missing = check(prompt, resp_text(data.get("tool_response")), fmt)
    mode = os.environ.get("CAVEMAN_GUARD", "log")
    if mode not in ("log", "warn", "off"):
        mode = "log"
    if mode == "off" or not missing:
        return
    root = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    now = time.gmtime()
    row = {"ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", now), "hook": "caveman-format",
           "agent": agent, "fmt": fmt, "missing": missing}
    try:
        d = os.path.join(root, "notes", "metrics")
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, time.strftime("%Y-%m-%d", now) + ".jsonl"), "a", encoding="utf-8") as f:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    except OSError:
        pass
    if mode == "warn":
        out = {"hookSpecificOutput": {"hookEventName": "PostToolUse",
               "additionalContext": f"필수 필드 누락: {', '.join(missing)}, 재요청"}}
        print(json.dumps(out, ensure_ascii=False))


try:
    main()
except Exception:
    pass
sys.exit(0)
