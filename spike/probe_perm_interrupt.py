"""Spike: permission control protocol, interrupt, resume. Records to samples/."""
import json, os, subprocess, sys, time, threading, queue
ENV = {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}
BASE = ["claude", "-p", "--input-format", "stream-json", "--output-format", "stream-json",
        "--verbose", "--include-partial-messages", "--model", "haiku", "--include-hook-events"]

def start(extra):
    p = subprocess.Popen(BASE + extra, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                         stderr=subprocess.PIPE, text=True, env=ENV)
    q = queue.Queue()
    threading.Thread(target=lambda: [q.put(l) for l in p.stdout] or q.put(None), daemon=True).start()
    return p, q

def send(p, obj):
    p.stdin.write(json.dumps(obj) + "\n"); p.stdin.flush()

def user(t): return {"type": "user", "message": {"role": "user", "content": t}}

def pump(q, log, until, on=None, timeout=90):
    end = time.time() + timeout
    while time.time() < end:
        try: l = q.get(timeout=1)
        except queue.Empty: continue
        if l is None: return None
        log.write(l); d = json.loads(l)
        if on: on(d)
        if until(d): return d
    return "TIMEOUT"

mode = sys.argv[1]
if mode == "perm":
    p, q = start(["--permission-prompt-tool", "stdio"]); log = open("samples/probe_perm.jsonl", "w")
    send(p, user("Use the Write tool to create the file hello_probe.txt containing the text hi. Then say done."))
    seen = []
    def on(d):
        if d["type"] == "control_request":
            seen.append(d); print("CONTROL_REQUEST", json.dumps(d)[:700])
            req = d["request"]
            send(p, {"type": "control_response", "response": {"subtype": "success",
                "request_id": d["request_id"],
                "response": {"behavior": "allow", "updatedInput": req.get("input", {})}}})
    r = pump(q, log, lambda d: d["type"] == "result", on)
    print("RESULT", r if isinstance(r, str) else (r["result"], r["permission_denials"]))
    p.stdin.close(); p.wait(30)
elif mode == "interrupt":
    p, q = start(["--permission-prompts", "none"]); log = open("samples/probe_interrupt.jsonl", "w")
    send(p, user("Count slowly from 1 to 300, one number per line, no tools."))
    n = [0]
    def stop(d):
        if d["type"] == "stream_event" and d["event"]["type"] == "content_block_delta":
            n[0] += 1
    pump(q, log, lambda d: n[0] >= 5, stop)
    send(p, {"type": "control_request", "request_id": "int1", "request": {"subtype": "interrupt"}})
    r = pump(q, log, lambda d: d["type"] == "result")
    print("AFTER_INTERRUPT", r if isinstance(r, str) else {k: r.get(k) for k in ("subtype", "is_error", "stop_reason", "terminal_reason", "result")})
    send(p, user("Reply with exactly: STILL-ALIVE"))
    r = pump(q, log, lambda d: d["type"] == "result")
    print("NEXT_TURN", r if isinstance(r, str) else r["result"])
    p.stdin.close(); p.wait(30)
elif mode == "resume":
    sid = sys.argv[2]
    p, q = start(["--resume", sid, "--permission-prompts", "none"] + sys.argv[3:]); log = open("samples/probe_resume.jsonl", "w")
    send(p, user("What exact word did I ask you to reply with in my first message? One word."))
    ids = set()
    r = pump(q, log, lambda d: d["type"] == "result", lambda d: ids.add(d.get("session_id")))
    print("RESUME ids", ids, "orig", sid, "->", r["result"] if isinstance(r, dict) else r)
    p.stdin.close(); p.wait(30)
