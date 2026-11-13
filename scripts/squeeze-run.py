#!/usr/bin/env python3
"""Run a command and print only the part of its output that matters. The full output goes to logs/runs/<ts>.log.

Usage:
  python3 scripts/squeeze-run.py -- <command> [args...]

Prints the first 8 entries, every error|warning|panic|FAIL line with 2 lines of context, and the last 20 entries,
then "log: <path>" and "exit: <code>". Repeated identical lines collapse to "<line> (xN)". Exits with the command's
exit code (128+signal if killed, 127 if it cannot start). Paths are relative to the current directory. Stdlib only.
"""
import os
import re
import subprocess
import sys
import time

HEAD, TAIL, CONTEXT = 8, 20, 2
KEY = re.compile(r"(?i:error|warning|panic)|FAIL")


def collapse(lines):
    out = []  # [text, count]
    for line in lines:
        if out and out[-1][0] == line:
            out[-1][1] += 1
        else:
            out.append([line, 1])
    return out


def pick(entries):
    keep = set(range(min(HEAD, len(entries))))
    keep.update(range(max(0, len(entries) - TAIL), len(entries)))
    for i, (text, _) in enumerate(entries):
        if KEY.search(text):
            keep.update(range(max(0, i - CONTEXT), min(len(entries), i + CONTEXT + 1)))
    return sorted(keep)


def main(argv):
    if argv[:1] == ["--"]:
        argv = argv[1:]
    if not argv:
        print("usage: squeeze-run.py -- <command> [args...]", file=sys.stderr)
        return 2
    runs = os.path.join(os.getcwd(), "logs", "runs")
    os.makedirs(runs, exist_ok=True)
    stamp = time.strftime("%y%m%d-%H%M%S")
    log, n = os.path.join(runs, stamp + ".log"), 1
    while os.path.exists(log):
        n += 1
        log = os.path.join(runs, f"{stamp}-{n}.log")
    try:
        proc = subprocess.run(argv, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        code = proc.returncode if proc.returncode >= 0 else 128 - proc.returncode
        data = proc.stdout
    except OSError as e:
        code, data = 127, f"squeeze-run: cannot start {argv[0]}: {e}\n".encode()
    text = data.decode("utf-8", "replace")
    with open(log, "w", encoding="utf-8") as f:
        f.write(text)
    lines = text.splitlines()
    entries = collapse(lines)
    shown = pick(entries)
    print(f"squeeze-run: {len(lines)} lines, {len(shown)} shown")
    prev = -1
    for i in shown:
        if prev >= 0 and i != prev + 1:
            print("...")
        line, count = entries[i]
        print(line if count == 1 else f"{line} (x{count})")
        prev = i
    print(f"log: {log}")
    print(f"exit: {code}")
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
