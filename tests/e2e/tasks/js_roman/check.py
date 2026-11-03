"""hidden checker: roman.js (toRoman/fromRoman). usage: python3 -I check.py <workdir>"""
import json
import os
import re
import subprocess
import sys
import tempfile

DRIVER = r"""
const mod = require(process.argv[2]);
const cases = JSON.parse(require('fs').readFileSync(0, 'utf8'));
const SPECIAL = { '__NaN__': NaN, '__Inf__': Infinity, '__-Inf__': -Infinity };
const out = cases.map(([fn, arg]) => {
  try {
    const v = fn === 'to' ? mod.toRoman(arg in SPECIAL ? SPECIAL[arg] : arg) : mod.fromRoman(arg);
    return { ok: v };
  } catch (e) {
    return { err: e && e.name ? e.name : String(e) };
  }
});
process.stdout.write(JSON.stringify(out));
"""


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


def roman(n):
    table = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
             (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]
    s = ""
    for v, sym in table:
        while n >= v:
            s += sym
            n -= v
    return s


def from_roman_canon(s):
    """정규 표기면 정수, 아니면 None (파싱 후 다시 만들었을 때 같아야 정규)."""
    vals = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100, "D": 500, "M": 1000}
    up = s.upper()
    if not up or not re.fullmatch(r"[IVXLCDM]+", up):
        return None
    total = 0
    for i, ch in enumerate(up):
        v = vals[ch]
        if i + 1 < len(up) and v < vals[up[i + 1]]:
            total -= v
        else:
            total += v
    if total < 1 or total > 3999 or roman(total) != up:
        return None
    return total


wd = os.path.abspath(sys.argv[1])
mod_path = os.path.join(wd, "roman.js")
if not os.path.isfile(mod_path):
    fail("roman.js 없음")
tmp = tempfile.mkdtemp(prefix="rom-")
drv = os.path.join(tmp, "driver.js")
with open(drv, "w", encoding="utf-8") as f:
    f.write(DRIVER)

to_cases = [("to", n) for n in range(1, 4000)]
bad_to = [0, -1, 4000, 4000000, 1.5, float("nan"), float("inf"), float("-inf"), -7.25]
to_cases += [("to", n) for n in bad_to]
valid_from = ["MCMXCIV", "mmxxvi", "IV", "MMMCMXCIX", "I", "xlix", "CDXLIV"]
bad_from = ["IIII", "VV", "IC", "IL", "IXI", "MMMM", "", "ABC", "VX", "LCM", "IIV", "XM"]
from_cases = [("from", s) for s in valid_from + bad_from]
rt_cases = [("from", roman(n)) for n in range(1, 4000)]  # 왕복 검사용 (toRoman 결과 기준)
cases = to_cases + from_cases + rt_cases

def enc(v):
    if isinstance(v, float):
        if v != v:
            return "__NaN__"
        if v in (float("inf"), float("-inf")):
            return "__Inf__" if v > 0 else "__-Inf__"
    return v


node = subprocess.run(
    ["node", drv, mod_path],
    input=json.dumps([[k, enc(v)] for k, v in cases]),
    capture_output=True, text=True, timeout=120,
)
if node.returncode != 0:
    fail("node 실행 실패: " + node.stderr[-600:])
got = json.loads(node.stdout)

fails = []
# toRoman
for (k, arg), res in zip(to_cases, got[:len(to_cases)]):
    if isinstance(arg, int) and 1 <= arg <= 3999:
        if res.get("ok") != roman(arg):
            fails.append("toRoman(%r)=%r want %r" % (arg, res, roman(arg)))
    else:
        if res.get("err") != "RangeError":
            fails.append("toRoman(%r) 는 RangeError 여야 함, got %r" % (arg, res))
# fromRoman
off = len(to_cases)
for (k, s), res in zip(from_cases, got[off:off + len(from_cases)]):
    want = from_roman_canon(s)
    if want is not None:
        if res.get("ok") != want:
            fails.append("fromRoman(%r)=%r want %r" % (s, res, want))
    else:
        if res.get("err") != "SyntaxError":
            fails.append("fromRoman(%r) 는 SyntaxError 여야 함, got %r" % (s, res))
# 왕복
off2 = off + len(from_cases)
for n, res in zip(range(1, 4000), got[off2:]):
    if res.get("ok") != n:
        fails.append("fromRoman(toRoman(%d)) = %r" % (n, res))
        if len(fails) > 20:
            break

if fails:
    fail("; ".join(fails[:20]) + (" ... (총 %d건)" % len(fails) if len(fails) > 20 else ""))
print("PASS (%d cases)" % len(cases))
