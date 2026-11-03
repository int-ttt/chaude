"""hidden checker: Go word-frequency CLI. usage: python3 -I check.py <workdir>"""
import collections
import os
import random
import re
import subprocess
import sys
import tempfile


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


def top(texts, n):
    cnt = collections.Counter()
    for t in texts:
        cnt.update(re.findall(r"[A-Za-z]+", t.lower()))
    return sorted(cnt.items(), key=lambda kv: (-kv[1], kv[0]))[:n]


wd = os.path.abspath(sys.argv[1])
if not os.path.isfile(os.path.join(wd, "main.go")):
    fail("main.go 없음")
if not os.path.isfile(os.path.join(wd, "go.mod")):
    fail("go.mod 없음")
tmp = tempfile.mkdtemp(prefix="gwf-")
exe = os.path.join(tmp, "wf")
r = subprocess.run(["go", "build", "-o", exe, "."], cwd=wd, capture_output=True, text=True)
if r.returncode != 0:
    fail("go build 실패: " + r.stderr[-800:])


def write(name, text):
    p = os.path.join(tmp, name)
    with open(p, "w", encoding="utf-8") as f:
        f.write(text)
    return p


def run(args):
    return subprocess.run([exe] + args, cwd=tmp, capture_output=True, text=True, timeout=120)


def parse(out):
    res = []
    for line in out.splitlines():
        if not line.strip():
            continue
        w, c = line.split()
        res.append((w, int(c)))
    return res


failed = []
a = write("a.txt", "The cat and the hat. the END, don't stop! Cat-dog 42 times.\n")
b = write("b.txt", "cat dog Dog DOG cat's zebra; the the\nQuick brown fox\n")
c = write("c.txt", "fox fox jumps over the lazy dog\n")

# 1) 기본 -n 10, 여러 파일 합산
r = run([a, b])
want = top([open(a).read(), open(b).read()], 10)
if r.returncode != 0 or parse(r.stdout) != want:
    failed.append("기본(-n 10): rc=%d got=%r want=%r" % (r.returncode, parse(r.stdout)[:10], want))

# 2) -n 3, 파일 3개
r = run(["-n", "3", a, b, c])
want = top([open(a).read(), open(b).read(), open(c).read()], 3)
if r.returncode != 0 or parse(r.stdout) != want:
    failed.append("-n 3: rc=%d got=%r want=%r" % (r.returncode, parse(r.stdout), want))

# 3) 동률이면 사전순
t1 = write("t1.txt", "pear apple banana\n")
t2 = write("t2.txt", "banana apple pear\n")
r = run(["-n", "3", t1, t2])
if r.returncode != 0 or parse(r.stdout) != [("apple", 2), ("banana", 2), ("pear", 2)]:
    failed.append("동률 정렬: rc=%d out=%r" % (r.returncode, r.stdout))

# 4) 없는 파일 -> stderr + rc 1
r = run([a, os.path.join(tmp, "nope.txt")])
if r.returncode != 1:
    failed.append("없는 파일: rc=%d want 1" % r.returncode)
elif not r.stderr.strip():
    failed.append("없는 파일: stderr 비어 있음")

# 5) 인자 없음 -> rc 2
r = run([])
if r.returncode != 2:
    failed.append("인자 없음: rc=%d want 2" % r.returncode)

# 6) 많은 파일 (동시 처리/합치기 검증)
rnd = random.Random(99)
vocab = ["w%d" % i for i in range(300)]
vocab = ["".join(chr(ord("a") + (i // 26 ** k) % 26) for k in range(3)) for i in range(300)]
files = []
texts = []
for i in range(60):
    body = " ".join(rnd.choice(vocab) for _ in range(2000))
    body = body.replace(" ", rnd.choice([" ", ", ", "\n", "-", "'", " 7 "]))
    texts.append(body)
    files.append(write("many%02d.txt" % i, body))
r = run(["-n", "25"] + files)
want = top(texts, 25)
if r.returncode != 0 or parse(r.stdout) != want:
    failed.append("60파일 -n 25: rc=%d 불일치 (got[:3]=%r want[:3]=%r)" % (r.returncode, parse(r.stdout)[:3], want[:3]))

if failed:
    fail("; ".join(failed))
print("PASS (6 checks)")
