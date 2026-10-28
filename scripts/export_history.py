#!/usr/bin/env python3
"""Export Claude Code conversations as light plain-text files for any viewer (no model calls).

usage: export_history.py [--project DIR] [--file SESSION.jsonl] [--out logs/history] [--width 120] [--split-kb 500]
       export_history.py --hook          (reads the hook's JSON on stdin and exports that session)

Keeps only user messages and Claude's visible replies (no tool output, no thinking, repeated pastes once),
wraps lines at --width and splits files at about --split-kb so PyCharm/Obsidian/VS Code open them instantly.
Writes <out>/<YYYYMMDD-HHMM>-<session8>-NN.txt and refreshes <out>/INDEX.txt.
"""
import argparse, glob, hashlib, json, os, re, sys, textwrap
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from claude_history import history_dir

NOISE = re.compile(r"<(command-name|command-message|command-args|local-command-stdout|system-reminder)>.*?</\1>", re.S)


def text_of(content):
    if isinstance(content, str):
        return content
    if not isinstance(content, list) or any(isinstance(c, dict) and c.get("type") == "tool_result" for c in content):
        return ""
    return "\n".join(c.get("text", "") for c in content if isinstance(c, dict) and c.get("type") == "text")


def wrap(t, width):
    out = []
    for line in t.splitlines() or [""]:
        out.extend(textwrap.wrap(line, width, replace_whitespace=False, drop_whitespace=False,
                                 break_long_words=True, break_on_hyphens=False) or [""])
    return "\n".join(out)


def export(path, out_dir, width, split_bytes):
    sid = os.path.splitext(os.path.basename(path))[0][:8]
    blocks, seen, first_ts = [], set(), ""
    with open(path, encoding="utf-8", errors="ignore") as f:
        for line in f:
            try:
                e = json.loads(line)
            except Exception:
                continue
            kind = e.get("type")
            if kind not in ("user", "assistant") or e.get("isMeta"):
                continue
            t = NOISE.sub("", text_of((e.get("message") or {}).get("content"))).strip()
            if not t:
                continue
            ts = (e.get("timestamp") or "")[:16].replace("T", " ")
            first_ts = first_ts or ts
            if kind == "user" and len(t) > 1500:
                h = hashlib.sha1(t.encode()).hexdigest()[:8]
                if h in seen:
                    t = f"[이전과 같은 붙여넣기 #{h}]"
                seen.add(h)
            who = "압축 요약" if e.get("isCompactSummary") else ("사용자" if kind == "user" else "Claude")
            mark = "====" if who == "사용자" else "----"
            blocks.append(f"{mark} {ts}  {who} {mark}\n{wrap(t, width)}\n\n")
    if not blocks:
        return []
    stamp = re.sub(r"\D", "", first_ts)[:12]
    stamp = stamp[:8] + "-" + stamp[8:12] if len(stamp) >= 12 else stamp
    for old in glob.glob(os.path.join(out_dir, f"*-{sid}-*.txt")):
        os.remove(old)
    files, cur, n = [], [], 0
    for b in blocks:
        size = len(b.encode("utf-8"))
        if cur and n + size > split_bytes:
            files.append(cur)
            cur, n = [], 0
        cur.append(b)
        n += size
    files.append(cur)
    names = []
    for i, part in enumerate(files, 1):
        name = f"{stamp}-{sid}-{i:02d}.txt"
        with open(os.path.join(out_dir, name), "w", encoding="utf-8", newline="\n") as f:
            f.writelines(part)
        names.append(name)
    return names


def write_index(out_dir):
    rows = sorted(os.path.basename(p) for p in glob.glob(os.path.join(out_dir, "*-*-*.txt")))
    with open(os.path.join(out_dir, "INDEX.txt"), "w", encoding="utf-8", newline="\n") as f:
        f.write("대화 기록 (사용자 메시지와 Claude 답변만, 최신이 아래)\n\n")
        f.write("\n".join(rows) + "\n")


ap = argparse.ArgumentParser()
ap.add_argument("--project", default=os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd())
ap.add_argument("--file")
ap.add_argument("--out")
ap.add_argument("--width", type=int, default=120)
ap.add_argument("--split-kb", type=int, default=500)
ap.add_argument("--hook", action="store_true")
args = ap.parse_args()

files = []
if args.hook:
    try:
        data = json.load(sys.stdin)
    except Exception:
        data = {}
    if data.get("transcript_path"):
        files = [data["transcript_path"]]
elif args.file:
    files = [args.file]
else:
    d = history_dir(args.project)
    files = sorted(glob.glob(os.path.join(d, "*.jsonl"))) if d else []

out = args.out or os.path.join(os.path.abspath(os.path.expanduser(args.project)), "logs", "history")
os.makedirs(out, exist_ok=True)
total = 0
for p in files:
    if os.path.exists(p):
        total += len(export(p, out, args.width, args.split_kb * 1024))
write_index(out)
if not args.hook:
    print(f"세션 {len(files)}개 → 파일 {total}개 · {out}")
