"""hidden checker: intervals.merge_intervals / insert_interval. usage: python3 -I check.py <workdir>"""
import copy
import importlib.util
import os
import random
import sys


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


wd = os.path.abspath(sys.argv[1])
path = os.path.join(wd, "intervals.py")
if not os.path.isfile(path):
    fail("intervals.py 없음")
spec = importlib.util.spec_from_file_location("intervals", path)
mod = importlib.util.module_from_spec(spec)
try:
    spec.loader.exec_module(mod)
except Exception as e:  # noqa: BLE001
    fail("import 실패: %r" % (e,))
for name in ("merge_intervals", "insert_interval"):
    if not hasattr(mod, name):
        fail("%s 없음" % name)
merge = mod.merge_intervals
insert = mod.insert_interval


def ref_merge(xs):
    out = []
    for s, e in sorted((tuple(x) for x in xs), key=lambda p: p[0]):
        if out and s <= out[-1][1]:
            out[-1] = (out[-1][0], max(out[-1][1], e))
        else:
            out.append((s, e))
    return out


def t_examples():
    assert merge([(1, 3), (2, 6), (8, 10), (15, 18)]) == [(1, 6), (8, 10), (15, 18)], "기본 예시"
    assert merge([(1, 2), (2, 3)]) == [(1, 3)], "딱 붙은 구간은 합침"
    assert merge([(5, 6), (1, 2), (2, 4)]) == [(1, 4), (5, 6)], "정렬 안 된 입력"
    assert merge([(3, 3)]) == [(3, 3)], "한 점 구간"
    assert merge([(1, 10), (2, 3)]) == [(1, 10)], "포함 관계"
    assert merge([]) == [], "빈 입력"
    assert merge([[1, 2], [3, 4]]) == [(1, 2), (3, 4)], "리스트 입력도 OK, 결과는 튜플"
    assert merge([(0.5, 1.5), (1.2, 2.0)]) == [(0.5, 2.0)], "실수"


def t_result_is_tuples():
    res = merge([(4, 5), (1, 2)])
    assert all(isinstance(x, tuple) for x in res), "결과 원소는 튜플"


def t_no_mutation():
    src = [(5, 6), (1, 2), (2, 4)]
    before = copy.deepcopy(src)
    merge(src)
    assert src == before, "입력 리스트를 바꾸면 안 됨"


def t_invalid():
    try:
        merge([(3, 1)])
    except ValueError:
        return
    except Exception as e:  # noqa: BLE001
        raise AssertionError("ValueError 여야 함, 실제 %r" % (e,))
    raise AssertionError("start > end 인데 예외 없음")


def t_insert():
    base = [(1, 2), (4, 6), (8, 10)]
    assert insert(base, (2, 5)) == [(1, 6), (8, 10)], "중간을 메우며 병합"
    assert insert(base, (11, 12)) == [(1, 2), (4, 6), (8, 10), (11, 12)], "뒤에 추가"
    assert insert(base, (0, 0)) == [(0, 0), (1, 2), (4, 6), (8, 10)], "앞에 추가"
    assert insert(base, (3, 3)) == [(1, 2), (3, 3), (4, 6), (8, 10)], "틈에 끼움 (안 붙음)"
    assert base == [(1, 2), (4, 6), (8, 10)], "merged 는 바꾸면 안 됨"
    assert insert([], (1, 1)) == [(1, 1)], "빈 merged"


def t_random_vs_reference():
    rnd = random.Random(5)
    for _ in range(500):
        n = rnd.randint(0, 12)
        xs = []
        for _ in range(n):
            s = rnd.randint(-20, 40)
            xs.append((s, s + rnd.randint(0, 8)))
        rnd.shuffle(xs)
        got = merge(xs)
        want = ref_merge(xs)
        assert got == want, "random mismatch input=%r got=%r want=%r" % (xs, got, want)


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
