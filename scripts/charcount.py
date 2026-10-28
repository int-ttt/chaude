#!/usr/bin/env python3
"""Exact character counts for Korean application text.

usage: charcount.py FILE [--section HEADING] [--limit N]
       cat text | charcount.py -
"""
import argparse, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

p = argparse.ArgumentParser()
p.add_argument("file")
p.add_argument("--section", help="markdown heading text; counts until the next heading of same or higher level")
p.add_argument("--limit", type=int, help="limit to check against (공백 포함, 줄바꿈 제외)")
a = p.parse_args()

text = sys.stdin.read() if a.file == "-" else open(a.file, encoding="utf-8").read()
if a.section:
    m = re.search(rf"^(#+)\s*{re.escape(a.section)}\s*$", text, re.M)
    if not m:
        sys.exit(f"section not found: {a.section}")
    level = len(m.group(1))
    rest = text[m.end():]
    end = re.search(rf"^#{{1,{level}}}\s", rest, re.M)
    text = rest[: end.start()] if end else rest
text = text.strip()

with_nl = len(text)
no_nl = len(text.replace("\r", "").replace("\n", ""))
no_ws = len(re.sub(r"\s", "", text))
utf8 = len(text.encode("utf-8"))
hangul2 = sum(2 if ord(c) > 127 else 1 for c in text.replace("\r", ""))

print(f"공백 포함(줄바꿈 포함): {with_nl}")
print(f"공백 포함(줄바꿈 제외): {no_nl}")
print(f"공백 제외: {no_ws}")
print(f"바이트(UTF-8): {utf8} / 바이트(한글 2byte): {hangul2}")
if a.limit:
    diff = a.limit - no_nl
    print(f"제한 {a.limit}: {'OK' if diff >= 0 else 'OVER'} ({diff:+d})")
