#!/usr/bin/env python3
"""UserPromptSubmit hook: adapts the session to a short, rapid-fire prompting style.

- "일반모드" / "절약모드" switch claude-lean off / on for this project (no model call). While off,
  every prompt gets a [normal] tag and nothing else below runs.
- Status checks ("진행상황", "끝났어?") are answered from logs/*.status without calling the model.
- Plain information statements are queued in notes/INBOX.md without calling the model, then
  delivered together with the next instruction. Disable with STYLE_INBOX=0; end a message with
  "?" or "!" to send it to the model immediately.
- Everything else gets a one-line tag so the main thread handles it with minimal thinking:
  [ack] approval, [retry] re-run, [frag] short refinement, [paste] long text saved to a file.
"""
import glob, hashlib, json, os, re, sys, time
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off, OFF_FLAG

data = json.load(sys.stdin)
prompt = (data.get("prompt") or "").strip()
root = os.environ.get("CLAUDE_PROJECT_DIR", ".")
notes = os.path.join(root, "notes")
inbox = os.path.join(notes, "INBOX.md")


def block(msg):
    print(json.dumps({"decision": "block", "reason": msg,
                      "hookSpecificOutput": {"hookEventName": "UserPromptSubmit", "suppressOriginalPrompt": True}},
                     ensure_ascii=False))
    sys.exit(0)


def last_reply_asks():
    """True if the main thread's last reply asked something or proposed a default (`다음 할 일:` line, or old ▶ form)."""
    try:
        with open(data.get("transcript_path") or "", "rb") as f:
            f.seek(0, 2)
            f.seek(max(0, f.tell() - 200_000))
            lines = f.read().decode("utf-8", "ignore").splitlines()
    except OSError:
        return False
    for line in reversed(lines):
        try:
            e = json.loads(line)
        except Exception:
            continue
        if e.get("type") != "assistant":
            continue
        content = (e.get("message") or {}).get("content") or []
        text = "".join(c.get("text", "") for c in content if isinstance(c, dict) and c.get("type") == "text")
        if text.strip():
            end = text.strip()[-300:]
            return "?" in end or "▶" in end or "다음 할 일:" in end
    return False


def tail(path, n):
    try:
        with open(path, encoding="utf-8", errors="ignore") as f:
            return [l.rstrip() for l in f.readlines()[-n:]]
    except OSError:
        return []


# 0. Mode switch, zero model tokens. 일반모드: hooks stand down and Opus works directly; 절약모드: back on.
if re.fullmatch(r"(일반|노멀) ?모드|lean ?off", prompt, re.I):
    os.makedirs(os.path.dirname(OFF_FLAG), exist_ok=True)
    open(OFF_FLAG, "w").close()
    block("일반 모드로 바꿨습니다 (모델 호출 없음). 다음 메시지부터 Opus가 직접 작업하고 절약 규칙이 꺼집니다.\n되돌리기: 절약모드")
if re.fullmatch(r"(절약|린) ?모드|lean ?on", prompt, re.I):
    if os.path.exists(OFF_FLAG):
        os.remove(OFF_FLAG)
    msg = "절약 모드로 돌아왔습니다 (모델 호출 없음)."
    if os.environ.get("CLAUDE_LEAN") == "0":
        msg += "\n단, 이 세션은 CLAUDE_LEAN=0으로 시작해서 계속 일반 모드입니다. /exit 후 CLAUDE_LEAN 없이 claude로 다시 시작하세요."
    block(msg)

sid = re.sub(r"[^\w-]", "", data.get("session_id") or "x")
mode_file = os.path.join(root, "logs", f".lean-{sid}")
try:
    was_off = open(mode_file).read().strip() == "off"
except OSError:
    was_off = False
now_off = lean_off()
if now_off != was_off:
    try:
        os.makedirs(os.path.dirname(mode_file), exist_ok=True)
        open(mode_file, "w").write("off" if now_off else "on")
    except OSError:
        pass
if now_off:
    print("[normal] claude-lean is off: work directly with tools, normal-length replies, no `다음 할 일:` line.")
    sys.exit(0)

# 1. Status fast path: zero model tokens.
STATUS = r"^(지금까지 |지금 )?(진행 ?상황|상태)( 보고해| 다시| 알려줘)?[?.]?$|^(작업 |처리 |다 )?(끝났|됐)(어|나|니|는지 확인해(봐)?)[?.]?$|^지금은[?.]?$|^로그( 확인해(봐)?)?[?.]?$"
if re.match(STATUS, prompt):
    files = sorted(glob.glob(os.path.join(root, "logs", "*.status")), key=os.path.getmtime, reverse=True)
    if files:
        out = []
        for st in files[:6]:
            job = os.path.basename(st)[:-7]
            age = int((time.time() - os.path.getmtime(st)) / 60)
            out.append(f"■ {job} (갱신 {age}분 전)")
            out += ["  " + l for l in tail(st, 3)]
            if prompt.startswith("로그"):
                out += ["  │ " + l for l in tail(os.path.join(root, "logs", f"{job}.log"), 8)]
        block("\n".join(out) + "\n(모델 호출 없이 표시. 분석이 필요하면 끝에 ? 를 붙여 다시 보내세요)")

# 2. Inbox: information statements wait for the next instruction.
CORRECTION = r"^(아니|근데|아 |그게 아니|아니면|그냥)"
# Any instruction or question anywhere in the message sends it to the model.
IMPERATIVE = (r"(해|해봐|해줘|해라|해놔|돌려|넘겨|바꿔|만들어|넣어|빼|줄여|늘려|옮겨|올려|고쳐|써|줘|찾아|봐|봐야|가자|하자|시켜|"
              r"채워|지워|맞춰|나눠|합쳐|붙여|둬|놔|보내|열어|잖아|라고|라니깐|해야지|해야|하고)(?=[\s.,!~]|$)"
              r"|확인|비교|분석|정리|검토|조사|작성|수정|추가|삭제|변경|포함|있을거야|있을텐데")
QUESTION = r"\?|아니야|아냐|않아|않나|나요|까요|는지|는데$|ㄴ데$|건가|일까|할까|아니면|왜|뭐|어디|어느|언제|어떻게|몇|무슨|누가"
DECLARATIVE = r"(거야|이야|이다|거든|그래|였어|었어|았어|상태|상태였어|임|음)[.~]?$"
if (os.environ.get("STYLE_INBOX", "1") != "0" and 8 <= len(prompt) <= 120 and "\n" not in prompt
        and prompt.count(".") <= 1 and not prompt.endswith("!") and not re.match(CORRECTION, prompt)
        and not re.search(IMPERATIVE, prompt) and not re.search(QUESTION, prompt)
        and re.search(DECLARATIVE, prompt) and not last_reply_asks()):
    os.makedirs(notes, exist_ok=True)
    with open(inbox, "a", encoding="utf-8") as f:
        f.write(f"- {time.strftime('%m-%d %H:%M')} {prompt}\n")
    n = len(tail(inbox, 1000))
    block(f"메모 {n}건 보관 (모델 호출 없음). 다음 지시와 함께 전달됩니다.\n바로 답을 원하면 끝에 ? 또는 ! 를 붙여 다시 보내세요.")

tags = ["[lean] claude-lean is back on: delegate per policy."] if was_off else []
pending = tail(inbox, 1000)

# 0b. Upgraded while this session was open: CLAUDE.md/agents/settings are read at launch, so don't inject the policy;
# tell the user once to restart (the resumed session loads the new version and session-context.py reports it).
def _session_version():
    p = os.path.join(root, "logs", f".lean-ver-{sid}")
    try:
        return open(p).read().strip(), p
    except OSError:
        pass
    try:                                       # sessions started before this check existed: compare times
        with open(data.get("transcript_path") or "", encoding="utf-8", errors="ignore") as fh:
            first = json.loads(fh.readline() or "{}").get("timestamp", "")
        started = time.mktime(time.strptime(first[:19], "%Y-%m-%dT%H:%M:%S")) - time.timezone
        vf = os.path.join(root, ".claude", "claude-lean.version")
        return ("?" if os.path.getmtime(vf) > started else None), p
    except Exception:
        return None, p


try:
    installed = open(os.path.join(root, ".claude", "claude-lean.version"), encoding="utf-8").read().strip()
except OSError:
    installed = ""
loaded, ver_state = _session_version()
if installed and loaded and loaded != installed:
    notified = os.path.join(os.path.dirname(ver_state), f".lean-notified-{sid}")
    try:
        done = open(notified).read().strip() == installed
    except OSError:
        done = False
    if not done:
        tags.append(f"[update] claude-lean {installed} was installed while this session was open; this session still runs the old rules, agents and settings. "
              "Add exactly one line to your reply telling the user: 새 버전이 설치됐습니다. /exit 후 claude -c (또는 claude --resume)로 이 대화를 이어가면 적용됩니다. "
              "Do not apply or restate the new policy now.")
        try:
            os.makedirs(os.path.dirname(ver_state), exist_ok=True)
            open(notified, "w").write(installed)
        except OSError:
            pass

if pending:
    tags.append("[inbox] user notes since your last turn (apply them; facts → FACTS.md via agent):\n" + "\n".join(pending))
    os.remove(inbox)

# 3. Long pasted text → file, so briefs carry a path instead of the text.
if len(prompt) > 1500 or prompt.count("\n") > 20 or "<pasted_content" in prompt:
    h = hashlib.sha1(prompt.encode()).hexdigest()[:8]
    path = os.path.join(notes, "pastes", f"{h}.md")
    dup = os.path.exists(path)
    if not dup:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        open(path, "w", encoding="utf-8").write(prompt)
    tags.append(f"[paste] {'same text as before, already at' if dup else 'saved to'} notes/pastes/{h}.md — give agents this path; never quote the text in briefs.")

# 4. One-line routing tags for short prompts.
p = prompt.rstrip(".!~ ")
if re.fullmatch(r"ㅇㅇ|ㅇㅋ|응|어|그래|좋아|오케이?|ok|[AB12]|[AB](안)?(으로|로) ?(가자|가|해)?|그렇게 해|그대로 (가|해)|(전부 |다 )?맞아|진행해|진행|해|해봐|수행해|전부 수행해|넣어|일단 넣어|바꿔|계속해?|ㄱㄱ?", p, re.I):
    tags.append("[ack] = approve the default in the `다음 할 일:` list of your last reply (first item; old forms: indented sentence, `▶ 기본:`). Execute it in one delegation round; reply ≤2 lines; no re-analysis.")
elif re.match(r"^(다시|재시도|재시작)", p):
    tags.append("[retry] re-run the last brief on the same agent; failed/unfinished items only.")
elif len(p) < 30 and not re.search(QUESTION, prompt) and not re.match(CORRECTION, prompt) and not pending:
    tags.append("[frag] short reply: if it answers your last question, use it; else relay verbatim to the active agent with the target named in the current `다음 할 일:` line. Don't re-plan.")

if tags:
    print("\n".join(tags))
sys.exit(0)
