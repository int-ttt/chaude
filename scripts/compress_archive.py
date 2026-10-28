#!/usr/bin/env python3
"""gzip archived Claude Code transcripts (*.jsonl → *.jsonl.gz, about 10x smaller). search_history.py reads both.

usage: compress_archive.py <archive dir> [...]"""
import glob, gzip, os, shutil, sys
for _s in (sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass

before = after = n = 0
for d in sys.argv[1:]:
    for p in glob.glob(os.path.join(d, "**", "*.jsonl"), recursive=True):
        with open(p, "rb") as src, gzip.open(p + ".gz", "wb", compresslevel=6) as dst:
            shutil.copyfileobj(src, dst)
        before += os.path.getsize(p)
        after += os.path.getsize(p + ".gz")
        os.remove(p)
        n += 1
if n:
    print(f"   압축: 기록 {n}개 {before / 1e6:.1f}MB → {after / 1e6:.1f}MB")
