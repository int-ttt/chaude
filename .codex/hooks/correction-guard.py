#!/usr/bin/env python3
"""UserPromptSubmit hook: detects correction/pushback prompts and injects a short type-specific reminder.
Also counts consecutive corrections per session; on the 2nd+ in a row, forces the checklist step."""
import json, os, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _s.reconfigure(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
from _lean import lean_off

if lean_off():
    sys.exit(0)
try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)
prompt = (data.get("prompt") or "").strip()
sid = re.sub(r"[^\w-]", "", data.get("session_id") or "default")

RULES = [
    ("C", r"(이|가)? ?아니라 .+(이다|야|이야|거든|였어|이고)|^아 .*(이다|이야|거든)[.!]?$", "fact -> /fact <old> → <new>"),
    ("D", r"안 ?바뀐|안 ?바꼈|바꾼 ?거 ?맞|했어\?|했지\?|어디 ?갔|빠졌", "missing change -> /check, FAILs to the same haiku-drafter/haiku-coder"),
    ("F", r"^(아니면|그냥|[AB]로 가자|[AB]안)", "direction change -> DECISIONS.md line via agent, re-brief"),
    ("A", r"그거 ?말고|그게 ?아니(라|고)|라니(깐|까)|(하|이|으)라고[.!]?$|말고", "misread -> '이해: <target>에 <action>', relay user's exact words to the same agent"),
    ("E", r"^근데|같은데[?.]?$|이상한데|아니야\?$|않아\?$", "pushback -> verdict first (맞습니다/일부/아닙니다) + <=3 lines, don't flip without evidence"),
    ("B", r"(수 ?있|가능하|되)(나|니|냐|어|지)\?|어때\??$", "question -> answer only, no action"),
]
hit = next(((k, msg) for k, pat, msg in RULES if re.search(pat, prompt)), None)
generic = hit is None and re.match(r"^(아니|그게 아니)", prompt) is not None
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
