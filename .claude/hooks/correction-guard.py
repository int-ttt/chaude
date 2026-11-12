#!/usr/bin/env python3
"""UserPromptSubmit hook: detects correction/pushback prompts and injects a short type-specific reminder.
Also counts consecutive corrections per session; on the 2nd+ in a row, forces the checklist step."""
import json, os, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _reconf = getattr(_s, "reconfigure", None)
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))  # also when run with python -I
from _lean import lean_off, pick, trigger

if lean_off():
    sys.exit(0)
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
prompt = (data.get("prompt") or "").strip()
sid = re.sub(r"[^\w-]", "", data.get("session_id") or "default")
# Task notifications and wrapper text (<task-notification>, <system-reminder>, <command-name>, ...) are not user prompts:
# no hit, and the counter is left as it is.
if re.search(r"<(task-notification|system-reminder|command-[\w-]+|local-command-[\w-]+|pasted_content)[\s>]", prompt):
    sys.exit(0)

# Trigger keywords per language (one dict; every language key is matched). Classes: C fact, D missing change,
# F direction, A misread, E pushback, B question, G preference/tone; "generic" = untyped 아니 / no.
# Adding a language = adding one key here (regex fragments per class).
KEYWORDS = {
    "ko": {
        "C": [r"(이|가)? ?아니라 .+(이다|야|이야|거든|였어|이고)", r"^아 .*(이다|이야|거든)[.!]?$"],
        "D": [r"안 ?바뀐|안 ?바꼈|바꾼 ?거 ?맞|했어\?|했지\?|어디 ?갔|빠졌"],
        "F": [r"^(아니면|그냥|[AB]로 가자|[AB]안)"],
        "A": [r"그거 ?말고|그게 ?아니(라|고)|라니(깐|까)|(?:^|[.!?] ?)(?:그|이|저)(?:거|게|렇게|건|것)[^.!?]{0,6}(하|이|으)라고[.!]?$|(?:^|[.!?] ?)말고"],
        "E": [r"^근데|같은데[?.]?$|이상한데|아니야\?$|않아\?$"],
        "B": [r"(수 ?있|가능하|되)(나|니|냐|어|지)\?|어때\??$"],
        "G": [r"너무 어려워|딱딱하게|내가 쓴 것처럼"],
        "generic": [r"^(아니|그게 아니)"],
    },
    "en": {
        "C": [r"(?i:\bit'?s\b.+\bnot\b|\bnot\b.+\bit'?s\b)"],
        "D": [r"(?i:\bdidn'?t (change|update|apply)|\bunchanged\b|\bnot changed\b|\bstill the same\b|\bwhere did .* go\b|\bmissing\b|\bdid you (change|do|apply|update) it\b)"],
        "F": [r"(?i:^(or else|otherwise|instead|just|go with [AB]|let'?s go with [AB])\b)"],
        "A": [r"(?i:\bnot that (one|file)\b|\bi meant\b|\bnot what i (said|meant|asked)\b|\bthe other (one|file)\b|\binstead of\b)"],
        "E": [r"(?i:^but\b|\bdoesn'?t (look|seem) right\b|\bare you sure\b|\bisn'?t it\b)"],
        "B": [r"(?i:\bis it possible\b|\bcan (it|you|we)\b.*\?$|\bhow about\b.*\?$)"],
        "G": [r"(?i:\btoo (hard|stiff)\b|\bsound like me\b)"],
        "generic": [r"(?i:^(no|not that)\b)"],
    },
}
# Message text is English; literals kept on purpose: '이해: ...' (misread restatement) and 맞습니다/일부/아닙니다
# (pushback verdict) are output formats documented in .claude/skills/correction/SKILL.md.
MSG = {
    "C": "fact -> /fact <old> → <new>",
    "D": "missing change -> /check, FAILs to the same haiku-drafter/haiku-coder",
    "F": "direction change -> DECISIONS.md line via agent, re-brief",
    "A": pick("misread -> '이해: <target>에 <action>', relay user's exact words to the same agent",
              "misread -> restate as 'Understanding: <action> for <target>', relay user's exact words to the same agent"),
    "E": pick("pushback -> verdict first (맞습니다/일부/아닙니다) + <=3 lines, don't flip without evidence",
              "pushback -> verdict first (right / partly / not right) + <=3 lines, don't flip without evidence"),
    "B": "question -> answer only, no action",
    "G": "preference/tone -> record rule in DECISIONS.md, relay to haiku-drafter, then revise",
}
RULES = [(k, trigger(KEYWORDS, k), MSG[k]) for k in ("C", "D", "F", "A", "E", "B", "G")]
hit = next(((k, msg) for k, pat, msg in RULES if re.search(pat, prompt)), None)
generic = hit is None and re.match(trigger(KEYWORDS, "generic"), prompt) is not None
# Only misread-type corrections (A, D, untyped 아니) signal a loop; anything else resets the count.
miss = generic or (hit is not None and hit[0] in ("A", "D"))

state_dir = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "logs")
state = os.path.join(state_dir, f".corr-{sid}")
try:
    n = int(open(state).read()) if os.path.exists(state) else 0
except Exception:
    n = 0
n = n + 1 if miss else 0
try:
    os.makedirs(state_dir, exist_ok=True)
    open(state, "w").write(str(n))
except Exception:
    pass

out = []
if hit:
    out.append(f"[corr {hit[0]}] {hit[1]}")
elif generic:
    out.append("[corr] relay user's exact words to the same agent; delta only")
if n >= 2:
    out.append(f"[corr x{n}] stop: <=5-bullet checklist in user's words, confirm, then one brief")
if out:
    print("\n".join(out))
sys.exit(0)
