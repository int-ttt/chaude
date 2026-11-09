#!/usr/bin/env python3
"""PreToolUse hook (matcher: Bash): no file edits through inline Python or node.
Scanned code is only what python/node is given: the -c / -e argument, or a heredoc in the same command
segment as the call (python3 - <<EOF, cat <<EOF | python3 -). Other heredoc bodies (cat <<EOF > notes.md) are data.
Denied when that code holds a write: open(..., 'w'|'a'|'x'|'+'...), .write_text/.write_bytes,
os.replace/rename/remove/unlink, shutil.copy*/move (python); fs.writeFile*/appendFile*/copyFile/rename/unlink/rm (node).
Read-only code, plain test runs (python3 -m pytest) and script files run by path (their code is not in the command)
stay allowed.
Also denied: unbounded wait loops (until, or while that is not a `while read` over a file) with no bound in the
command (alarm / timeout / numeric compare / (( )) ). A background poll that never ends can hang the agent.
Fail-open on unreadable input."""
import json, re, sys
for _s in (sys.stdin, sys.stdout, sys.stderr):
    try:
        _reconf = getattr(_s, "reconfigure", None)
        if _reconf:
            _reconf(encoding="utf-8")
    except Exception:
        pass
sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__

DENY_MSG = "Edit files with the Edit/Write tools, not Python. Use Read + Edit (replace_all for bulk)."
WAIT_MSG = ("Unbounded wait loop denied: bound it (perl -e 'alarm shift; exec @ARGV' <secs> <cmd>, or a counter "
            "limit in the loop condition) so the Bash call cannot hang.")
LOOP_KW = re.compile(r"(?:^|[\s;&|(`])(until|while)\s")
READ_LOOP = re.compile(r"(?:^|[\s;&|(`])while\s+read\b")
BOUND = re.compile(r"\balarm\b|\btimeout\b|\bgtimeout\b|\bseq\b|\(\(|-(?:lt|le|gt|ge|eq|ne)\s")

PY_CALL = re.compile(r"(?:^|[\s;&|(`/])python(?:3(?:\.\d+)?)?(?=[\s;&|)`]|$)")
PY_WRITE = re.compile(
    r"\.write_(?:text|bytes)\s*\("
    r"|\bos\.(?:replace|rename|remove|unlink)\s*\("
    r"|\bshutil\.(?:copy\w*|move)\s*\("
    # open(path, 'w') with one level of nested parens allowed (open(os.path.join(a, b), 'w'))
    r"|\bopen\s*\((?:[^()]|\([^()]*\))*?,\s*(?:mode\s*=\s*)?['\"](?=[rwaxbt+]*[wax+])[rwaxbt+]+['\"]"
    # Path(p).open('w') takes the mode first
    r"|\.open\s*\(\s*['\"](?=[rwaxbt+]*[wax+])[rwaxbt+]+['\"]"
)
NODE_CALL = re.compile(r"(?:^|[\s;&|(`/])node(?=[\s;&|)`]|$)")
NODE_WRITE = re.compile(r"\bfs\.(?:promises\.)?(?:writeFile|appendFile|copyFile|rename|unlink|rm)\w*\s*\(")
HEREDOC = re.compile(r"(?<!<)<<(-?)[ \t]*(['\"]?)([A-Za-z_][\w.-]*)\2")
SEGMENT_SEP = re.compile(r"&&|\|\||;|(?<![>&])&(?![>&])")
PY_OPT_C = re.compile(r"(?:[ \t]+-[\w.-]+)*?[ \t]+-[A-Za-z]*c[ \t]*")
NODE_OPT_E = re.compile(r"(?:[ \t]+-[\w-]+)*?[ \t]+(?:-e|-p|--eval|--print)(?=[ \t]|$)[ \t]*")
QUOTED = re.compile(r"\"((?:[^\"\\]|\\.)*)\"|'([^']*)'", re.S)


def deny(reason):
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                      "permissionDecisionReason": reason}}, ensure_ascii=False))
    sys.exit(0)


def segment_at(line, pos):
    """The shell segment (split on && || ; &) that holds position pos of line."""
    start = 0
    for sep in SEGMENT_SEP.finditer(line):
        if sep.start() >= pos:
            return line[start:sep.start()]
        start = sep.end()
    return line[start:]


def heredoc_split(cmd):
    """Return (text without heredoc bodies, [(kind, body)]) where bodies are fed to a python/node call.
    Bodies of other commands are dropped as data."""
    lines = cmd.split("\n")
    kept, fed = [], []
    i = 0
    while i < len(lines):
        line = lines[i]
        i += 1
        kept.append(line)
        for mk in HEREDOC.finditer(line):
            dash, delim = mk.group(1) == "-", mk.group(3)
            j = i
            while j < len(lines) and (lines[j].lstrip("\t") if dash else lines[j]) != delim:
                j += 1
            body = "\n".join(lines[i:j])
            i = j + 1
            seg = segment_at(line, mk.start())
            if PY_CALL.search(seg):
                fed.append(("py", body))
            if NODE_CALL.search(seg):
                fed.append(("node", body))
    return "\n".join(kept), fed


def code_given(cmd):
    """(kind, code) pairs that python/node will execute: heredoc bodies fed to them, and -c / -e arguments."""
    text, fed = heredoc_split(cmd)
    yield from fed
    for kind, call, opt in (("py", PY_CALL, PY_OPT_C), ("node", NODE_CALL, NODE_OPT_E)):
        for m in call.finditer(text):
            tail = text[m.end():]
            o = opt.match(tail)
            if not o:
                continue
            rest = tail[o.end():]
            q = QUOTED.match(rest)
            if q:
                yield kind, next(g for g in q.groups() if g is not None)
            else:
                yield kind, rest.split("\n", 1)[0]


def unbounded_wait(cmd):
    """True when the command has a until/while loop (with do) that is not `while read` over input and
    the command holds no bound (alarm, timeout, seq, (( )), numeric compare)."""
    for m in LOOP_KW.finditer(cmd):
        tail = cmd[m.start(1):]
        if not re.search(r"\bdo\b", tail):
            continue
        if READ_LOOP.search(cmd, m.start()) and m.group(1) == "while":
            continue
        return not BOUND.search(cmd)
    return False


try:
    data = json.loads(sys.stdin.read())
    if not isinstance(data, dict) or (data.get("tool_name") or "") != "Bash":
        sys.exit(0)
    cmd = (data.get("tool_input") or {}).get("command") or ""
    if not isinstance(cmd, str):
        sys.exit(0)
    if unbounded_wait(cmd):
        deny(WAIT_MSG)
    for kind, code in code_given(cmd):
        if (kind == "py" and PY_WRITE.search(code)) or (kind == "node" and NODE_WRITE.search(code)):
            deny(DENY_MSG)
except SystemExit:
    raise
except Exception:
    pass
sys.exit(0)
