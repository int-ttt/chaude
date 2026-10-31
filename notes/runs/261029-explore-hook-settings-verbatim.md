# verbatim read: haiku-context-guard.py, settings.json hooks, sonnet-analyst frontmatter, CLAUDE.md headings
## .claude/hooks/haiku-context-guard.py (full file, 41 lines, cat -n)
```
     1	#!/usr/bin/env python3
     2	"""PreToolUse hook (matcher: Read|Bash): Haiku subagents may not use Read's allow_large flag; sonnet-analyst may not Read/Grep/Glob directly (delegate to haiku-scout); sonnet-/opus- may not read or write files via Bash (delegate to haiku-scout / haiku-coder).
     3	
     4	Claude Code itself caps each Read at env CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS (8000; observed on a
     5	Read: "cap 8000"), so no size estimate is needed here. allow_large is the only way around that cap, and
     6	it is denied for Haiku so it reads in slices (offset/limit) or greps first. Agents compact on their own
     7	(autoCompactWindow 100000). No-op in normal mode."""
     8	import json, re, sys
     9	for _s in (sys.stdin, sys.stdout, sys.stderr):
    10	    try:
    11	        _s.reconfigure(encoding="utf-8")
    12	    except Exception:
    13	        pass
    14	sys.dont_write_bytecode = True  # keep .claude/hooks free of __pycache__
    15	from _lean import lean_off
    16	
    17	BASH_READ = re.compile(r'(?:^|[;&\n(]|\|\||&&)\s*(?:\w+=\S*\s+)*(?:cat|head|tail|sed|awk|gawk|grep|egrep|fgrep|rg|less|more|nl|od|xxd|strings)\b')
    18	BASH_WRITE = re.compile(r'\btee\b|(?<![>&])>>?(?!&)\s*(?!/dev/null\b)\S')
    19	
    20	try:
    21	    data = json.load(sys.stdin)
    22	except Exception:
    23	    sys.exit(0)
    24	kind = data.get("agent_type") or ""
    25	if not lean_off() and data.get("agent_id") and kind == "sonnet-analyst" and data.get("tool_name") in ("Read", "Grep", "Glob"):
    26	    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
    27	                      "permissionDecisionReason": "sonnet-analyst may not read files directly; delegate to haiku-scout."}}))
    28	    sys.exit(0)
    29	if not lean_off() and data.get("agent_id") and kind.startswith(("sonnet-", "opus-")) and data.get("tool_name") == "Bash":
    30	    cmd = (data.get("tool_input") or {}).get("command") or ""
    31	    if BASH_READ.search(cmd) or BASH_WRITE.search(cmd):
    32	        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
    33	                          "permissionDecisionReason": "Sonnet/Opus may not read or write files via Bash (cat/head/tail/sed/awk/grep/redirect/tee). Delegate reads to haiku-scout and writes/edits to haiku-coder or haiku-batch-worker."}}))
    34	        sys.exit(0)
    35	if lean_off() or not data.get("agent_id") or kind.startswith(("sonnet-", "opus-")) or kind in ("analyst", "reviewer"):
    36	    sys.exit(0)
    37	if data.get("tool_name") == "Read" and (data.get("tool_input") or {}).get("allow_large"):
    38	    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
    39	                      "permissionDecisionReason": "allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first."}}))
    40	sys.exit(0)
```
## .claude/settings.json hooks section (file lines 18-125, numbering preserved)
```
    18	  "hooks": {
    19	    "UserPromptSubmit": [
    20	      {
    21	        "hooks": [
    22	          {
    23	            "type": "command",
    24	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/correction-guard.py\"",
    25	            "timeout": 5
    26	          }
    27	        ]
    28	      },
    29	      {
    30	        "hooks": [
    31	          {
    32	            "type": "command",
    33	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/style-adapter.py\"",
    34	            "timeout": 5
    35	          }
    36	        ]
    37	      },
    38	      {
    39	        "hooks": [
    40	          {
    41	            "type": "command",
    42	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/context-alert.py\"",
    43	            "timeout": 5
    44	          }
    45	        ]
    46	      }
    47	    ],
    48	    "PreToolUse": [
    49	      {
    50	        "matcher": "Agent|Task",
    51	        "hooks": [
    52	          {
    53	            "type": "command",
    54	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/agent-model-pin.py\"",
    55	            "timeout": 5
    56	          }
    57	        ]
    58	      },
    59	      {
    60	        "matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Bash|PowerShell|Grep|Glob|WebSearch|WebFetch",
    61	        "hooks": [
    62	          {
    63	            "type": "command",
    64	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/main-thread-guard.py\"",
    65	            "timeout": 5
    66	          }
    67	        ]
    68	      },
    69	      {
    70	        "matcher": "Read|Bash",
    71	        "hooks": [
    72	          {
    73	            "type": "command",
    74	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/haiku-context-guard.py\"",
    75	            "timeout": 5
    76	          }
    77	        ]
    78	      }
    79	    ],
    80	    "SubagentStop": [
    81	      {
    82	        "hooks": [
    83	          {
    84	            "type": "command",
    85	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/ledger.py\"",
    86	            "timeout": 5
    87	          }
    88	        ]
    89	      }
    90	    ],
    91	    "SessionStart": [
    92	      {
    93	        "matcher": "startup|resume|compact|clear",
    94	        "hooks": [
    95	          {
    96	            "type": "command",
    97	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/session-context.py\"",
    98	            "timeout": 5
    99	          }
   100	        ]
   101	      }
   102	    ],
   103	    "SessionEnd": [
   104	      {
   105	        "hooks": [
   106	          {
   107	            "type": "command",
   108	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/history-export.py\"",
   109	            "timeout": 60
   110	          }
   111	        ]
   112	      }
   113	    ],
   114	    "PreCompact": [
   115	      {
   116	        "hooks": [
   117	          {
   118	            "type": "command",
   119	            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/history-export.py\"",
   120	            "timeout": 60
   121	          }
   122	        ]
   123	      }
   124	    ]
   125	  }
```
## .claude/agents/sonnet-analyst.md frontmatter (lines 1-15)
```
     1	---
     2	name: sonnet-analyst
     3	description: Sonnet (분석 담당). Complexity 1-2 work and implementing opus-architect plans. Reasoning the main thread hands off - long explanations, plans, comparisons, option analysis from given facts; synthesizing many sources; interpreting data or analysis results; multi-file/design-level code; tasks Haiku failed twice.
     4	tools: Write, Bash, WebSearch, WebFetch, Agent
     5	model: sonnet
     6	effort: medium
     7	omitClaudeMd: true
     8	autoCompactWindow: 100000
     9	---
    10	
    11	Questions handed off by the main thread: the brief quotes the user's words and the needed context. Write the answer for the user (Korean 존댓말) to `notes/runs/<YYMMDD>-answer-<topic>.md`; return only the path + a 1-line gist.
    12	Read failure reports in the brief first; never repeat tried approaches. Documents: read FACTS.md / DECISIONS.md; never invent facts.
    13	Separate evidence from inference; mark uncertainty. Code: smallest complete change, narrow verification, exact results.
    14	Full write-up → `notes/runs/<YYMMDD>-analyst-<topic>.md`.
    15	Otherwise return ≤5 lines: conclusion · confidence · path · open question.
```
## CLAUDE.md headings (grep -n '^#'; file is 33 lines)
```
2:# Policy: the main thread listens and routes (Haiku in chaude; Opus only if forced); subagents do all work
30:# Compact instructions
```
