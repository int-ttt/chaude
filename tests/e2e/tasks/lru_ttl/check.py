"""hidden checker: lru_ttl.TTLCache. usage: python3 -I check.py <workdir>"""
import importlib.util
import os
import random
import sys


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


wd = os.path.abspath(sys.argv[1])
path = os.path.join(wd, "lru_ttl.py")
if not os.path.isfile(path):
    fail("lru_ttl.py 없음")
spec = importlib.util.spec_from_file_location("lru_ttl", path)
mod = importlib.util.module_from_spec(spec)
try:
    spec.loader.exec_module(mod)
except Exception as e:  # noqa: BLE001
    fail("import 실패: %r" % (e,))
if not hasattr(mod, "TTLCache"):
    fail("TTLCache 없음")
TTLCache = mod.TTLCache


class Clock:
    def __init__(self):
        self.t = 0.0

    def __call__(self):
        return self.t


def make(cap, ttl):
    clk = Clock()
    return clk, TTLCache(cap, ttl, clock=clk)


def t_basic():
    clk, c = make(3, 100)
    c.put("a", 1)
    c.put("b", 2)
    assert c.get("a") == 1, "get a"
    assert c.get("zz") is None, "missing -> None"
    assert c.get("zz", "d") == "d", "default 인자"
    assert len(c) == 2, "len"


def t_lru_eviction():
    clk, c = make(2, 1000)
    c.put("a", 1)
    c.put("b", 2)
    assert c.get("a") == 1
    c.put("c", 3)  # b 가 가장 오래 안 쓰임
    assert c.get("b") is None, "b 는 버려져야"
    assert c.get("a") == 1 and c.get("c") == 3
    assert len(c) == 2


def t_put_counts_as_use():
    clk, c = make(2, 1000)
    c.put("a", 1)
    c.put("b", 2)
    c.put("a", 9)  # put 도 최근 사용
    c.put("c", 3)
    assert c.get("b") is None, "b 가 버려져야"
    assert c.get("a") == 9, "a 값 갱신 반영"


def t_ttl_expiry():
    clk, c = make(10, 10)
    c.put("k", "v")
    clk.t = 9.5
    assert c.get("k") == "v", "9.5초 시점엔 살아 있어야"
    clk.t = 10.5
    assert c.get("k") is None, "10.5초 시점엔 만료"
    assert len(c) == 0, "만료 항목은 len 에서 빠져야"


def t_put_refreshes_ttl():
    clk, c = make(10, 10)
    c.put("k", 1)
    clk.t = 8
    c.put("k", 2)
    clk.t = 15
    assert c.get("k") == 2, "재put 으로 만료 연장 (18초까지)"
    clk.t = 19
    assert c.get("k") is None, "19초엔 만료"


def t_expired_purged_before_lru():
    # a 는 만료, b 는 살아 있음. a 가 최근 사용이고 b 가 LRU 여도 만료된 a 를 먼저 치워야 함.
    clk, c = make(2, 10)
    c.put("a", "A")          # t=0, 만료 10
    clk.t = 2
    c.put("b", "B")          # t=2, 만료 12
    clk.t = 3
    assert c.get("a") == "A"  # a 가 최근 사용, b 가 LRU
    clk.t = 11               # a 만료, b 살아 있음
    c.put("c", "C")
    assert c.get("b") == "B", "살아 있는 b 는 남아야 (만료된 a 를 먼저 치워야)"
    assert c.get("c") == "C"


def t_randomized_model():
    # 참조 모델과 무작위 연산 비교 (정수 시각, ttl=10.5 라 경계에서 안 겹침)
    rnd = random.Random(1234)
    cap, ttl = 3, 10.5
    clk, c = make(cap, ttl)
    store = {}  # key -> [value, expires, last_used_tick]
    tick = 0
    now = 0

    def live(k):
        return k in store and store[k][1] > now

    for step in range(3000):
        now += rnd.randint(0, 3)
        clk.t = float(now)
        tick += 1
        op = rnd.random()
        k = rnd.randint(0, 7)
        if op < 0.4:
            v = rnd.randint(0, 99)
            if live(k):
                store[k] = [v, now + ttl, tick]
            else:
                for x in [x for x in store if not live(x)]:
                    del store[x]
                while len(store) >= cap:
                    victim = min(store, key=lambda x: store[x][2])
                    del store[victim]
                store[k] = [v, now + ttl, tick]
            c.put(k, v)
        elif op < 0.8:
            want = None
            if live(k):
                want = store[k][0]
                store[k][2] = tick
            got = c.get(k)
            assert got == want, "step %d get(%r): got %r want %r" % (step, k, got, want)
        else:
            want = len([x for x in store if live(x)])
            assert len(c) == want, "step %d len: got %r want %r" % (step, len(c), want)


tests = [(name, fn) for name, fn in sorted(globals().items()) if name.startswith("t_") and callable(fn)]
failed = []
for name, fn in tests:
    try:
        fn()
    except AssertionError as e:
        failed.append("%s: %s" % (name, e))
    except Exception as e:  # noqa: BLE001
        failed.append("%s: %r" % (name, e))
if failed:
    fail("; ".join(failed))
print("PASS (%d tests)" % len(tests))
