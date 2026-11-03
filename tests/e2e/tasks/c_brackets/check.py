"""hidden checker: balanced.c (stdin lines -> balanced/unbalanced). usage: python3 -I check.py <workdir>"""
import os
import random
import subprocess
import sys
import tempfile


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


def ref(line):
    pairs = {")": "(", "]": "[", "}": "{"}
    st = []
    for ch in line:
        if ch in "([{":
            st.append(ch)
        elif ch in pairs:
            if not st or st.pop() != pairs[ch]:
                return False
    return not st


wd = os.path.abspath(sys.argv[1])
src = os.path.join(wd, "balanced.c")
if not os.path.isfile(src):
    fail("balanced.c 없음")
tmp = tempfile.mkdtemp(prefix="cbr-")
exe = os.path.join(tmp, "balanced")
r = subprocess.run(["gcc", "-O2", "-Wall", "-o", exe, src], capture_output=True, text=True)
if r.returncode != 0:
    fail("gcc 실패: " + r.stderr[-800:])


def run(text):
    r = subprocess.run([exe], input=text.encode(), capture_output=True, timeout=120)
    if r.returncode != 0:
        raise AssertionError("exit %d" % r.returncode)
    return r.stdout.decode().splitlines()


def expect(text):
    return ["balanced" if ref(l) else "unbalanced" for l in text.splitlines()]


cases = [
    "()",
    "([)]",
    "{[()()]}",
    "(",
    ")(",
    "a(b)c",
    "[ { } ]x",
    "((]",
    "}",
    "()\n(\n\n",
    "()\n[(]",  # 마지막 개행 없음
    "\n\n",
    "x y z\n{}\n",
]
rnd = random.Random(7)
cases.append("".join(rnd.choice("()[]{}ab ") for _ in range(200000)) + "\n")
cases.append("".join(rnd.choice("()[]{}") for _ in range(20000)) + "\n" + "(" * 300000 + ")" * 300000 + "\n")
cases.append("(" * 300000 + "\n" + "[" * 1000000 + "]" * 1000000 + "\n")

failed = []
for i, text in enumerate(cases):
    want = expect(text)
    try:
        got = run(text)
    except AssertionError as e:
        failed.append("case %d: %s" % (i, e))
        continue
    except subprocess.TimeoutExpired:
        failed.append("case %d: timeout" % i)
        continue
    if got != want:
        shown = repr(text[:40])
        failed.append("case %d (%s...): got %d lines, first diff at %s" % (
            i, shown, len(got), next((j for j in range(min(len(got), len(want))) if got[j] != want[j]), min(len(got), len(want)))))
if failed:
    fail("; ".join(failed))
print("PASS (%d cases)" % len(cases))
