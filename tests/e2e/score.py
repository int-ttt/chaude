#!/usr/bin/env python3
"""e2e 채점기 (run.sh 가 호출). 실행: python3 -I tests/e2e/score.py <task|summary> ...

task    : 과제 하나 채점 -> <out>/<task>.score.json
          - hidden checker(tasks/<t>/check.py) 실행 -> check PASS/FAIL
          - 답변 휴리스틱: 비어 있지 않음 / 한국어 / 산출물 파일명·용어 언급 / 실행 방법 언급
          - E2E_JUDGE=1 이면 haiku 판정(judge_prompt.md) 추가
summary : 전체 표 출력 + <out>/summary.json, 과제 FAIL 이 하나라도 있으면 종료코드 1
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
TASK_ORDER = ["lru_ttl", "c_brackets", "go_wordfreq", "py_intervals", "js_roman"]
RUN_WORDS = ["실행", "테스트", "컴파일", "빌드", "gcc", "go run", "go build", "go test", "node ",
             "python", "pytest", "unittest", "사용법", "사용 방법", "run", "$ "]
HANGUL = re.compile(u"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")


def parse_result(text):
    """claude -p --output-format json 출력(객체 또는 메시지 배열)에서 result 객체를 뽑는다."""
    try:
        data = json.loads(text)
    except Exception as e:  # noqa: BLE001
        return None, "JSON 파싱 실패: %s" % e
    if isinstance(data, list):
        results = [d for d in data if isinstance(d, dict) and d.get("type") == "result"]
        data = results[-1] if results else {}
    return (data if isinstance(data, dict) else {}), None


def tokens_of(data):
    """modelUsage(서브에이전트 포함, 모델별) 합계가 있으면 그걸, 없으면 최상위 usage."""
    keys = ["inputTokens", "outputTokens", "cacheReadInputTokens", "cacheCreationInputTokens"]
    mu = data.get("modelUsage")
    if isinstance(mu, dict) and mu:
        tot = dict((k, 0) for k in keys)
        for m in mu.values():
            for k in keys:
                tot[k] += int(m.get(k, 0) or 0)
        tot["source"] = "modelUsage"
        return tot
    u = data.get("usage") or {}
    return {
        "inputTokens": int(u.get("input_tokens", 0) or 0),
        "outputTokens": int(u.get("output_tokens", 0) or 0),
        "cacheReadInputTokens": int(u.get("cache_read_input_tokens", 0) or 0),
        "cacheCreationInputTokens": int(u.get("cache_creation_input_tokens", 0) or 0),
        "source": "usage",
    }


def run_checker(task_dir, workdir):
    chk = os.path.join(task_dir, "check.py")
    try:
        r = subprocess.run([sys.executable, "-I", chk, workdir], capture_output=True, text=True, timeout=600)
    except subprocess.TimeoutExpired:
        return False, "checker timeout"
    out = (r.stdout + r.stderr).strip()
    return r.returncode == 0, out[-600:]


def answer_check(reply, meta):
    text = reply or ""
    low = text.lower()
    hangul = len(HANGUL.findall(text))
    latin = len(LATIN.findall(text))
    checks = {
        "nonempty": len(text.strip()) >= 20,
        "korean": hangul >= 20 and hangul / float(hangul + latin) >= 0.3,
        "describes": any(f.lower() in low for f in meta.get("files", []))
        or any(t.lower() in low for t in meta.get("terms", [])),
        "how_to_run": any(w.lower() in low for w in RUN_WORDS),
    }
    checks["pass"] = all(checks.values())
    checks["hangul_chars"] = hangul
    return checks


def run_judge(task_prompt, reply):
    tmpl_path = os.path.join(HERE, "judge_prompt.md")
    with open(tmpl_path, encoding="utf-8") as f:
        tmpl = f.read()
    prompt = tmpl.replace("{{TASK}}", task_prompt).replace("{{REPLY}}", reply or "(empty)")
    cwd = tempfile.mkdtemp(prefix="e2e-judge-")  # 빈 폴더: 하네스 CLAUDE.md/hook 영향 없음
    try:
        r = subprocess.run(["claude", "-p", prompt, "--model", "haiku", "--output-format", "json"],
                           cwd=cwd, capture_output=True, text=True, timeout=300)
    except subprocess.TimeoutExpired:
        return {"error": "judge timeout"}
    data, err = parse_result(r.stdout)
    if err:
        return {"error": err}
    text = str(data.get("result", ""))
    found = re.findall(r"\{.*\}", text, re.S)
    verdict = {}
    if found:
        try:
            verdict = json.loads(found[-1])
        except Exception:  # noqa: BLE001
            verdict = {"error": "verdict JSON 파싱 실패", "raw": text[-300:]}
    else:
        verdict = {"error": "verdict JSON 없음", "raw": text[-300:]}
    verdict["cost_usd"] = data.get("total_cost_usd")
    return verdict


def cmd_task(a):
    with open(os.path.join(a.task_dir, "meta.json"), encoding="utf-8") as f:
        meta = json.load(f)
    with open(os.path.join(a.task_dir, "prompt.ko.txt"), encoding="utf-8") as f:
        task_prompt = f.read()
    with open(a.json, encoding="utf-8") as f:
        raw = f.read()
    data, err = parse_result(raw) if raw.strip() else (None, "빈 출력")
    data = data or {}
    reply = data.get("result") or ""
    ok_check, detail = run_checker(a.task_dir, a.workdir)
    ans = answer_check(reply, meta)
    score = {
        "task": a.task,
        "check": "PASS" if ok_check else "FAIL",
        "check_detail": detail,
        "answer": ans,
        "cli_rc": a.cli_rc,
        "cli_error": err,
        "is_error": bool(data.get("is_error", False)),
        "subtype": data.get("subtype"),
        "cost_usd": float(data.get("total_cost_usd") or 0.0),
        "num_turns": data.get("num_turns"),
        "duration_s": round((data.get("duration_ms") or 0) / 1000.0, 1),
        "tokens": tokens_of(data),
        "workdir": a.workdir,
        "reply_chars": len(reply),
        "judge": None,
    }
    if os.environ.get("E2E_JUDGE") == "1":
        score["judge"] = run_judge(task_prompt, reply)
    with open(os.path.join(a.out, a.task + ".score.json"), "w", encoding="utf-8") as f:
        json.dump(score, f, ensure_ascii=False, indent=2)
    print("%-13s check=%s answer=%s cost=$%.4f in=%d out=%d cache_read=%d cache_create=%d turns=%s %ss" % (
        a.task, score["check"], "PASS" if ans["pass"] else "FAIL", score["cost_usd"],
        score["tokens"]["inputTokens"], score["tokens"]["outputTokens"],
        score["tokens"]["cacheReadInputTokens"], score["tokens"]["cacheCreationInputTokens"],
        score["num_turns"], score["duration_s"]))
    print("  checker: %s" % detail.replace("\n", " | ")[:300])
    print("  answer: nonempty=%s korean=%s describes=%s how_to_run=%s" % (
        ans["nonempty"], ans["korean"], ans["describes"], ans["how_to_run"]))
    if score["judge"]:
        print("  judge: %s" % json.dumps(score["judge"], ensure_ascii=False)[:300])
    return 0


def cmd_summary(a):
    scores = []
    for t in TASK_ORDER:
        p = os.path.join(a.out, t + ".score.json")
        if os.path.isfile(p):
            with open(p, encoding="utf-8") as f:
                scores.append(json.load(f))
    keys = ["inputTokens", "outputTokens", "cacheReadInputTokens", "cacheCreationInputTokens"]
    total = dict((k, 0) for k in keys)
    cost = 0.0
    lines = ["| task | check | answer | cost USD | in | out | cache_read | cache_create | turns | sec |",
             "|---|---|---|---|---|---|---|---|---|---|"]
    fails = 0
    for s in scores:
        tk = s["tokens"]
        for k in keys:
            total[k] += tk[k]
        cost += s["cost_usd"]
        if s["check"] != "PASS":
            fails += 1
        lines.append("| %s | %s | %s | %.4f | %d | %d | %d | %d | %s | %s |" % (
            s["task"], s["check"], "PASS" if s["answer"]["pass"] else "FAIL", s["cost_usd"],
            tk["inputTokens"], tk["outputTokens"], tk["cacheReadInputTokens"],
            tk["cacheCreationInputTokens"], s["num_turns"], s["duration_s"]))
    lines.append("| TOTAL | %d/%d | | %.4f | %d | %d | %d | %d | | |" % (
        len(scores) - fails, len(scores), cost, total["inputTokens"], total["outputTokens"],
        total["cacheReadInputTokens"], total["cacheCreationInputTokens"]))
    table = "\n".join(lines)
    print(table)
    with open(os.path.join(a.out, "summary.json"), "w", encoding="utf-8") as f:
        json.dump({"cost_usd": cost, "tokens": total, "tasks": scores}, f, ensure_ascii=False, indent=2)
    missing = [t for t in TASK_ORDER if t not in [s["task"] for s in scores]]
    if missing:
        print("missing scores: %s" % ", ".join(missing))
        return 1
    return 1 if fails else 0


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    t = sub.add_parser("task")
    t.add_argument("--task", required=True)
    t.add_argument("--task-dir", required=True)
    t.add_argument("--workdir", required=True)
    t.add_argument("--json", required=True)
    t.add_argument("--cli-rc", type=int, default=0)
    t.add_argument("--out", required=True)
    s = sub.add_parser("summary")
    s.add_argument("--out", required=True)
    a = ap.parse_args()
    return cmd_task(a) if a.cmd == "task" else cmd_summary(a)


if __name__ == "__main__":
    sys.exit(main())
