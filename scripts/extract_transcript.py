#!/usr/bin/env python3
"""Turn old Claude Code transcripts into small text chunks for distillation.

usage: extract_transcript.py <project folder | ~/.claude/projects/<dir> | session.jsonl> [--out notes/distill]
                             [--chunk 7000] [--reply-chars 800]

Keeps: user messages (full; repeated pastes stored once), Claude's visible replies (head + tail),
compaction summaries. Drops: tool calls and results, thinking, command/hook noise, subagent transcripts.
Writes <out>/chunks/NNN.md (≤ --chunk chars each, in time order) and <out>/INDEX.md.
"""
import argparse, glob, hashlib, json, os, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

ap = argparse.ArgumentParser()
ap.add_argument("source")
ap.add_argument("--out", default="notes/distill")
ap.add_argument("--chunk", type=int, default=7000)
ap.add_argument("--reply-chars", type=int, default=800)
args = ap.parse_args()

from claude_history import history_dir, projects_root

src = os.path.abspath(os.path.expanduser(args.source))
if src.endswith(".jsonl"):
    files = [src]
else:
    d = src if os.path.normcase(os.path.dirname(src)) == os.path.normcase(projects_root()) else history_dir(src)
    files = glob.glob(os.path.join(d, "*.jsonl")) if d else []
if not files:
    sys.exit(f"대화 기록을 찾지 못했습니다: {src}\n~/.claude/projects 아래 폴더를 직접 지정해 보세요.")

NOISE = re.compile(r"<(command-name|command-message|command-args|local-command-stdout|system-reminder)>.*?</\1>", re.S)


def text_of(content):
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""
    if any(isinstance(c, dict) and c.get("type") == "tool_result" for c in content):
        return ""
    return "\n".join(c.get("text", "") for c in content if isinstance(c, dict) and c.get("type") == "text")


def clip(s, n):
    return s if len(s) <= n else s[: n * 5 // 8] + f"\n[…{len(s) - n}자 생략…]\n" + s[-(n * 3 // 8):]


entries, seen_paste = [], {}
for path in files:
    sid = os.path.splitext(os.path.basename(path))[0][:8]
    with open(path, encoding="utf-8", errors="ignore") as f:
        for line in f:
            try:
                e = json.loads(line)
            except Exception:
                continue
            kind, msg = e.get("type"), e.get("message") or {}
            if kind not in ("user", "assistant") or e.get("isMeta"):
                continue
            t = NOISE.sub("", text_of(msg.get("content"))).strip()
            if not t:
                continue
            ts = (e.get("timestamp") or "")[:16].replace("T", " ")
            if kind == "user" and e.get("isCompactSummary"):
                label, t = "압축요약", clip(t, 4000)
            elif kind == "user":
                label = "사용자"
                if len(t) > 1500:
                    h = hashlib.sha1(t.encode()).hexdigest()[:8]
                    if h in seen_paste:
                        t = f"[이전과 같은 붙여넣기 #{h} 반복]"
                    else:
                        seen_paste[h] = ts
                        t = f"[붙여넣기 #{h}]\n" + clip(t, 3000)
            else:
                label, t = "Claude", clip(t, args.reply_chars)
            entries.append((ts, sid, label, t))

entries.sort(key=lambda x: x[0])
os.makedirs(os.path.join(args.out, "chunks"), exist_ok=True)
chunks, cur = [], ""
for ts, sid, label, t in entries:
    block = f"### {ts} [{sid}] {label}\n{t}\n\n"
    if cur and len(cur) + len(block) > args.chunk:
        chunks.append(cur)
        cur = ""
    cur += block
if cur:
    chunks.append(cur)

for i, c in enumerate(chunks, 1):
    open(os.path.join(args.out, "chunks", f"{i:03d}.md"), "w", encoding="utf-8").write(c)

total = sum(len(c) for c in chunks)
with open(os.path.join(args.out, "INDEX.md"), "w", encoding="utf-8") as f:
    f.write(f"# 추출 인덱스\n- 원본: {len(files)}개 세션, 메시지 {len(entries)}개\n- 조각: {len(chunks)}개, 총 {total:,}자\n")
    f.write(f"- 기간: {entries[0][0] if entries else '-'} ~ {entries[-1][0] if entries else '-'}\n")
    f.write(f"- 권장 배분: haiku-extractor 1개당 조각 4개 → {max(1, -(-len(chunks) // 4))}개 병렬\n")
print(f"세션 {len(files)}개 · 메시지 {len(entries)}개 → 조각 {len(chunks)}개 ({total:,}자) · {args.out}/INDEX.md")
