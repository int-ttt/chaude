# v0.8.2 scope excerpts (read-only extraction)
task: verbatim excerpts for guard/Read/Write/Bash scope, 6 items
status: done; no repo files edited

## 1a notes/runs/261029-scope-v082-impl.md (full, cat -n)
     1	status: ESCALATE (not implemented)
     2	reason: 7 interlocked files must change (.claude/hooks/haiku-context-guard.py, .claude/settings.json, .claude/agents/sonnet-analyst.md, sonnet-reviewer.md, opus-architect.md, CLAUDE.md, CHANGELOG) -> exceeds 5-file limit for analyst role
     3	decision already made by user: option b (sonnet/opus Read/Write/Edit/Grep/Glob/Bash all blocked; Haiku-only file access)
     4	design ref: notes/runs/261029-scope-v082-write-block.md
     5	open: none on design; needs executor with edit rights
     6	options: (1) main session dispatches haiku-coder per file in order: hook(+matcher in settings, Pyright line 11 reconfigure fix) -> agents x3 (tools: Agent only) -> CLAUDE.md rule -> CHANGELOG 0.8.2; then verify hook with sample inputs, record in notes/runs/261029-verify-v082-block.md. (2) split into 2 tasks: hook+settings+verify; then agents+docs.
     7	recommend: option 2 (hook is the risky part; verify before changing agent tools)
     8	Do not touch any other file. Reply 'done'.

## 1b notes/runs/261029-scope-v082-write-block.md (full, cat -n)
     1	# scope: v0.8.2 write-block release (ESCALATE)
     2	status: 구현 미완 / 릴리즈 미진행 (커밋·태그·리뷰·haiku 비교 모두 보류)
     3	evidence (haiku-scout 확인, a1cedfc4):
     4	- sonnet-analyst.md:4 tools: Write, Bash, WebSearch, WebFetch, Agent -> Write·Bash 남아있음 (Read/Grep/Glob만 없음)
     5	- sonnet-reviewer.md:4 tools: Read, Grep, Glob, Bash -> 제거 안 됨
     6	- opus-architect.md:4 tools: Agent -> 이미 OK
     7	- hook matcher: settings.json:70-74 `Read|Bash` -> Write/Edit/Grep/Glob 미차단
     8	- guard.py: Read/Grep/Glob 차단은 sonnet-analyst만; sonnet-/opus- Bash는 읽기/쓰기 패턴 매칭 시만 차단; Write/Edit 차단 없음
     9	- guard.py 11행: `_s.reconfigure(encoding="utf-8")` (stdio 인코딩) -> 로직 아님; 사용자가 말한 "reconfigure 경고"는 별도 확인 필요(실행 시 경고 재현 안 됨/미확인)
    10	- CLAUDE.md: "파일 읽기·쓰기(Bash 포함)는 Haiku 에이전트만" 규칙 없음 (10~11행 유사 문구만)
    11	- CHANGELOG: 0.8.2 항목 없음 (최신 0.8.1)
    12	- tag v0.8.2 없음; 변경 파일: guard.py, settings.json (미커밋)
    13	open decision (요구 충돌):
    14	- sonnet-reviewer에서 Read/Grep/Glob/Bash를 모두 빼면 검토 자체가 불가(검토 = 파일 읽기). 선택지:
    15	  A) reviewer는 Read/Grep/Glob 유지 예외 (규칙을 "analyst/architect는 금지, reviewer는 읽기 전용 허용"으로 완화; Write/Edit/Bash쓰기만 차단)
    16	  B) reviewer tools = Agent만, 읽기는 haiku-checker/scout에 위임 (규칙 일관, 검토 품질은 Haiku 요약에 의존)
    17	  C) 규칙 그대로 두고 reviewer 삭제, haiku-checker가 검토 (사용자 4번 테스트와 연결, 단 "릴리즈 검토 권한 haiku 불부여"와 충돌)
    18	- sonnet-analyst도 Write 제거 시 notes/runs 결과파일 작성 불가 -> haiku-drafter/batch-worker 위임 필요 (브리프 규약과 일치하나 흐름 변경)
    19	- 훅 matcher를 Read|Write|Edit|Grep|Glob|Bash 로 확장 + sonnet-/opus- 전면 차단 시 이 에이전트 자신의 notes 작성도 막힘
    20	recommendation: B (규칙 일관, 최소 변경). confidence: medium.
    21	next if approved: haiku-coder로 tools 4개 파일/훅/matcher/CLAUDE.md/CHANGELOG 수정 -> py_compile, go build/vet/test -> sonnet-reviewer(또는 B면 reviewer 경유) -> 커밋(Co-Authored-By: Claude Haiku 5.5) -> v0.8.2 태그/push/gh release -> haiku-checker 동일 체크리스트 비교.

## 2 grep -n -i guard|hook|Read|Write|Bash|sonnet|opus|model|agent_type|PreToolUse|block (FACTS)
6:## API 가격 (USD/MTok: input / 5m write / 1h write / cache read / output)
8:- Opus 5.5: 4 / 5 / 8 / 0.20 / 20
9:- Sonnet 5.5: 2 / 2.50 / 4 / 0.10 / 10
11:- budget.go 별칭 가정: opus=Opus 5.5, sonnet=Sonnet 5.5, haiku=Haiku 5.5
## 2 grep (DECISIONS, full lines)
5:- [structure] Hooks = Python (user: "아니 go보다 각 환경에서 계속 수정하려면 python으로 가는게 나을것같아"): .claude/hooks/*.py restored from git (88d8158), stdlib only, Python>=3.8. REPL + scripts = Go (earlier user: "go로 전부 구현하고, 설치 코드에 go를 포함해"): one module, one multi-call binary bin/harness[.exe]: `repl` | `gen-settings <os>` | script subcommands (search-history, charcount, export-history, ...); no `hook` subcommand. Plan: notes/runs/261028-plan-go-port.md
7:- [os-opt] .claude/settings.json = posix source (`python3 "$CLAUDE_PROJECT_DIR/.claude/hooks/X.py"`); windows derived by `harness gen-settings windows [--python CMD]`: exec form command=<py>, args=[..., "${CLAUDE_PROJECT_DIR}/.claude/hooks/X.py"] (no shell, no Git Bash); interpreter: install.ps1 picks first working of $HARNESS_PYTHON / `py -3` / python / python3 (>=3.8 check rejects Store alias), gen-settings default `py -3` if on PATH else python; re-run re-targets already-converted hooks; permission Bash(bin/harness search-history:*) + PowerShell(bin/harness.exe search-history:*) (deduped)
10:- [method] Port work split (user: "설계만 opus로 하고, 각 기능은 파일별로 haiku 돌려서 구현하는게 낫지 않나?"): opus = structure + common pkgs (internal/lean, osenv, settings, scripts registry) + per-file spec (notes/runs/261028-plan-go-port.md); per-file implementation -> haiku-coder (one per file, brief = .py path + plan path); REPL/install -> sonnet. This port: hooks/scripts were already dispatched to sonnet before the correction and passed full difftests, kept as-is.
11:- [cleanup] Python deleted except hooks: scripts/, harness/, spike/, tests/ (recoverable from git 88d8158). Go hook port (internal/hooks) deleted after hooks went back to Python. history-export.py now runs `bin/harness export-history --hook` (stdin passed through; skipped if binary missing); session-context.py uses _lean.history_dir (python copy of claude_history). docs/claude-lean/* untouched.
12:- [tui] user: "클로드 코드와 똑같은 화면 구조를 만들어" -> `harness repl` on a TTY runs bubbletea/lipgloss TUI (internal/harness/tui.go, alt screen): welcome box + scrolling viewport (user `>`, assistant `●` streaming, tool `● Name(arg)` + `⎿` result, dim notes) + bordered input + status line (model · ctx/threshold · rot · key hints). Keys: enter send, esc/ctrl+c interrupt, y/n permission, pgup/pgdn scroll, /rotate /quit /help, ctrl+c idle or ctrl+d quit. Piped stdin or HARNESS_PLAIN=1 keeps the old plain REPL (tests rely on it). Session/rotation/streaming code unchanged (Repl.ui hook only). Deps: bubbletea v1.3.4, lipgloss v1.1.0, bubbles v0.20.0 (go.mod still `go 1.22`). Capture: notes/runs/261028-tui-capture.txt
13:- [tui] slash cmds: /help /clear(new session, no handoff) /status /model [name](switch = rotate with new model) /rotate /exit /quit; typing "/" shows autocomplete list (up/down select, tab complete, enter run). Korean bug cause: bubbletea leaves the real terminal cursor at end of the bottom status line, so the OS IME preedit drew over it; fix = tuiCursorWriter (cursorWriter in tui.go) appends "cursor up 2 + column" after each frame and undoes it before the next, so real cursor sits in the input box; also textinput width halved when value has wide chars (rune count != cell width). Startup delay = bubbletea init() OSC11 background query (5s timeout when terminal doesn't answer; real terminals answer). Multi-line input not done. Capture: notes/runs/261028-tui-capture2.txt
14:- [tui] threshold default 300k (status denominator uses a.Threshold), child auto-compact window 400k (must stay > threshold). Input: textinput replaced by own inputBox (internal/harness/input.go) measuring per-rune cells with go-runewidth (Hangul=2); replaces the "halve width if wide chars" hack; wrapping, cursor, left/right/home/end/backspace/up/down are cell-exact. Multi-line: trailing "\" + enter, alt+enter or ctrl+j = newline; box grows to 5 rows then scrolls. Real cursor placed by cursorWriter (col = border+prompt+cells). Known: >256-byte bulk writes without bracketed paste may drop a char (bubbletea reader chunking), not seen with normal typing. Capture: notes/runs/261028-tui-capture3.txt
16:- [rotate] 261028 user: "최대 상한은 없애자. 그리고 매 답변 후마다 현재 컨텍스트 크기가 270k 이상이면 다음 세션을 전환 작업을 수행하도록 하자" -> single rule, no hard cap (drop 300k cap, prep stage, max-auto-rotations): after every answer if ctx >= 270000 do handoff + switch session; compact-window stays > threshold. Plan: notes/runs/261028-plan-sonnet-conductor.md (impl pending; repl.go still 300k)
18:- [rotate] 261028 user: "이전 프로세스를 끝내지 않는 방식. 에이전트가 끝날 때까지 백그라운드로 남겨 두고, 결과 파일이 생기면 새 세션에 알려 줍니다. 이게 맞는것같다" -> on rotation with running background agents the old claude child is NOT closed: moved to Repl.draining; done = result file exists+size>0+stable 2s (poll) OR old child's completion event (ledger.py to add agent_id); then drain old events ~30s, sess.Close() (stdin close, 15s, Kill); timeouts: 60min total / 10min idle, max 3 drains, harness exit closes all; old child permission requests auto-denied; completion notice "[agent done] ID·task·path" queued and prepended to the next user input of the new session (no auto turn). Replaces resume-based approach. Unverified: agentId/output path in tool_result, bg completion event, child keeps running agents with idle stdin. Plan 3-1: notes/runs/261028-plan-sonnet-conductor.md
19:- [rotate] 261028 user: "15분 이상 입력(키보드 입력을 포함한 모든 인풋)을 안하면 강제 세션 초기화(다음 세션으로 넘길 데이터만 정리)를 수행해. 이후 다음 프롬프트가 입력되면 다음 세션에게 해당 세션 정리치를 먼저 넣은 뒤 프롬프트 입력해서 해." -> TUI idle timer (tea.Tick, lastInput = key/paste/click/wheel; paused during a running turn/permission prompt; agent notices do not reset); at 15m rotate("idle 15m"), snapshot held as pendingSnapshot, sent before the next prompt (order: snapshot, agent notices, prompt); skip if no turns since last rotation; running agents -> draining (see previous line). Cost net ~0 vs continuing (handoff reads expired cache once; next prompt would have paid it), option: deterministic snapshot (0 tokens) when ctx small. Unverified: cache TTL ~5min, mouse-motion counting. Plan 3-2.
20:- [review] 261028 stateless conductor Sonnet (reset every input, inject last 추천: + open workstreams + last 2 turns) is a review item only, unverified; to be a 3rd arm of the Sonnet-vs-Opus experiment (plan 4-b).
21:- [rotate] 261028 user: "캐시 만료가 5분이라면 그냥 5분으로 하자 4:50초" -> idle rotation threshold 15m -> 4m50s (flag --idle-rotate, default 290s, 0=off). Handoff turn now runs inside cache TTL -> billed at cache-read price. Risks: rotations more frequent; TTL may not be 5min (e.g. 1h) -> TTL measurement is a prerequisite. Supersedes the 15m value in the previous idle line. Plan 3-2.
22:- [review] 261028 user: "커맨더에게 최근 5 프롬프트와 답변, 전체 프로그레스를 요약한 정보를 넘겨주면 되는거 아닐까?" + "조사와 요약은 haiku로만 처리하게" -> stateless conductor input = last 5 prompt/answer pairs + full untruncated 추천: + overall progress summary; collection/summary by Haiku only, progress summary updated incrementally per turn (stored in notes), conductor never re-reads raw. Lengths/budget in plan 4-b are proposals, unverified.
26:- [arch] 261028 user: "그냥 사용할 때, 모든 훅을 해제시키고, 훅을 하네스 프로그램에만 묶어서 하네스가 메인 세션에만 지금 훅들을 시작할 때 명시하면 되는거잖아" -> empty hooks in project .claude/settings.json; harness passes hooks via --settings (extend gen-settings) to the main session only; workers get none or a minimal hook set (large-file slice read, model strip); CLAUDE.md loading for workers handled separately (--bare / separate cwd / safe-mode, auth unverified). --settings semantics confirmed in official docs; hook merge behavior and --bare auth with subscription unverified. Plan 4-c.
27:- [arch] 261028 user: "bare 테스트해보고, 일꾼 hook이랑 이런걸 나누면 되는거잖아. 그리고 짧은 일회성 작업인지 판단은 sonnet에서 하면 되는거고" -> hook sets split: main (context injection/tags) vs worker (guards: slice read, model strip), both generated by gen-settings; --bare auth test pending (separate agent); conductor Sonnet decides per task subagent (short one-off, research) vs session worker (continuing workstream, same files reused, long/parallel); default subagent, promote on follow-up. Criteria in plan 4-c, accuracy unverified.
28:- [arch] 261028 user: "hook으로 claude.md 말고 main.md를 읽게 하면 되는거잖아" (--bare fails on subscription login) -> rename CLAUDE.md to main.md; main hook set SessionStart injects it (10,000-char cap per additionalContext, overflow becomes file path + 2,000-char preview; hook failure is non-blocking so main may start rule-less: mitigate with onFailure block / marker check / --append-system-prompt-file); workers get no CLAUDE.md, only worker hooks + --append-system-prompt; subagents lose main.md (docs: they load CLAUDE.md otherwise) - restate shared rules in agent definitions; unverified items in plan 4-c.
29:- [rotate] 261029 user: "어떤 분류 내에 들어가있는 작업을 계속할 경우엔 계속 유지하되, 해당 분류를 넘어갈 경우 새 세션으로 넘기게 해. 분류 구분은 opus 등 메인 세션이 프롬프트를 받고 짧게 판단만 하게 해. 500토큰 이하로" -> main session appends `[cat: X]` / `[cat-switch: X]` to each reply (rule via --append-system-prompt, internal/harness/category.go, <=500 tokens); on switch chaude rotates after that turn with normal handoff + notice; acks (ㅇㅇ etc.) never switch (code guard isAck); same label never rotates; flag --no-category-rotate; metrics stage=category
30:- [conductor] 261029 user: "sonnet 해설자가 모델 세션에 넘기는 프롬프트는 사용자의 프롬프트를 할 일과 어떤 모델로 수행할지만 정한거로 가야지 sonnet 해설자가 답변을 생성하면 안돼" -> conductor (Sonnet, chaude --conductor or /mode conductor|conductor-off, v0.6.7) outputs ONLY a delegation {task = user request verbatim + minimal context, target = haiku-*|sonnet-analyst|opus-architect}; never writes answers/replies/prose (only the harness [cat:] marker line allowed); target agent writes the Korean 존댓말 reply. Judgment/pushback/correction/strategy/'opus로' -> opus-architect. Supersedes earlier brief "conductor writes the Korean replies". Rule: conductorRule in internal/harness/repl.go; metrics field "conductor".
32:- [conductor] 261029 user: "ㅇㅇ 분류기로 바꿔서 다시 테스트해봐" -> --conductor reworked: no conductor chat session. chaude makes one-shot `claude -p --model sonnet` classifier call per prompt (internal/harness/classifier.go, cwd=tmpdir, no tools/session) -> terse {model,task}; task sent straight to a session on that model (switch: restart if no history else rotate with handoff); reply shown unchanged; acks and slash commands go to current session; category rotation unchanged; metrics stage=classify logs cost separately. Supersedes conductor-session rule.
33:- [conductor] 261029 user: "확인하고 0.6.8로 릴리즈해" -> classifier prompt verified live (6 prompts: tradeoff/pushback/correction/strategy -> opus, 2 mechanical -> haiku; TestClassifierLive, CHAUDE_LIVE=1); routed task suffix demands one final reply, no interim status, no 추천 block, foreground agents; released 0.6.8.
35:- [routing] 261029 user: "sonnet=검토·분석 담당, haiku=간단한 조사·질문 담당, '이거 뭐였는지 찾아' 류 검색/조회 프롬프트는 haiku high로 처리. 기존 방향(서브에이전트 지속적 진행, 지속화 후 0.7.0) 유지" -> haiku-scout and haiku-researcher effort high; sonnet-reviewer=검토, sonnet-analyst=분석 (descriptions); agent-model-pin hook now lets subagents call sonnet-reviewer (was haiku-* only; reviewer has no Agent tool so no recursion); CLAUDE.md routing + chaude delegateHint updated; main stays single Opus.
36:- [agents] 261029 user: "모든 에이전트는 작업 끝에 간단한 결과 요약만 반환, 메인 Opus가 세션에 게시" -> all 11 .claude/agents/*.md got one identical line after their Return rule: "Final message = only a brief result summary (a few lines, Korean 존댓말: outcome · changed paths · PASS/FAIL) for the main session to post; full details go to `notes/runs/`, give only the path."
38:- [routing] 261029 user: conductor main session Haiku-fixed, Opus only for design -> pickModel/pickTarget return haiku (forced --model sonnet|opus kept; forced opus keeps delegateHint). Large tasks get planHint: Haiku writes plan -> opus-architect/sonnet-analyst review (improvements only) -> Haiku merges and executes. route() no longer restarts the session when already haiku. CLAUDE.md + opus-architect/sonnet-analyst "Plan review" paragraph updated; test TestHaikuPlanWorkflow. Details notes/runs/haiku-conductor-plan.md.
39:- [routing] 261029 user: "sonnet-analyst는 파일 직접 접근 금지; 유사 파일끼리 묶어 최대 3개 파일/묶음 단위로 Haiku에게 정리 요청 후 요약만 받아 분석 (토큰 절약 목적). Opus 세션은 원래 정책대로 유지, Haiku가 대략적 계획 관리, 과거 구조와 현재 구조 부분 병합(하이브리드)." -> sonnet-analyst reads no files itself; Haiku condenses similar-file groups (max 3 files/group), analyst gets summaries only. Why: token saving. Other rules unchanged.
40:- [main-model] 2026-10-29 user: "chaude already running in GoLand terminal, text input works. Judge whether opus main should be used only for design turns. Apply switching criteria only to haiku; other models are used where long context must be kept." -> (1) Opus main only for design turns, as the original policy says. (2) context-threshold switching (T) applies only to Haiku main; Opus and Sonnet mains keep their context and do not switch by threshold. (3) GoLand terminal input issue resolved: text input works. Plan: notes/runs/261029-plan-hybrid-design.md
41:- [rotate] 2026-10-29 user: rotation threshold Haiku main 80k (new session handoff); Opus and Sonnet main 270k. Overview length cap (user decision, policy; no existing code limit): Haiku overview 3,000 chars; Opus/Sonnet overview 6,000 chars.
42:- [local-cmd] 2026-10-29 user: "알아서 처리, 검증 통과 시 릴리즈" -> held items resolved on safest default (keep current behavior, no code change): /clear deferral = keep (no deferral); /model name validation = none (claude CLI rejects bad names); /version = sync (cheap, no model call); conductor reverting /model = keep as is; unknown "/x" in plain mode = pass through to claude unchanged; /clear with running agents = keep current behavior. Reason: no small clearly-correct fix; avoids regressions. Also aligned agent "Return ≤5 lines" wording to final message ≤3 lines/300 chars; added idle-path specter test.
## 2 grep (LESSONS, full lines)
3:- 2026-10-29 failed: /resume fix rejected earlier because session was draining; never applied. Import must also be shown in TUI blocks, and rotate must not handoff/overwrite a pending import snapshot.

## 3 .claude/hooks/haiku-context-guard.py (full, cat -n)
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

## 4 .claude/settings.json permissions+hooks (cat -n lines 13-125)
    13	  "permissions": {
    14	    "allow": [
    15	      "Bash(bin/chaude search-history:*)"
    16	    ]
    17	  },
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

## 5 agent frontmatter (cat -n)
== .claude/agents/sonnet-analyst.md
1: ---
2: name: sonnet-analyst
3: description: Sonnet (분석 담당). Complexity 1-2 work and implementing opus-architect plans. Reasoning the main thread hands off - long explanations, plans, comparisons, option analysis from given facts; synthesizing many sources; interpreting data or analysis results; multi-file/design-level code; tasks Haiku failed twice.
4: tools: Write, Bash, WebSearch, WebFetch, Agent
5: model: sonnet
6: effort: medium
7: omitClaudeMd: true
8: autoCompactWindow: 100000
9: ---
== .claude/agents/sonnet-reviewer.md
1: ---
2: name: sonnet-reviewer
3: description: Sonnet (검토 담당). Read-only review before anything leaves the project - submitted, published, printed, sent, merged or released (documents, reports, slides, code diffs). Replaces the main thread reading full text.
4: tools: Read, Grep, Glob, Bash
5: model: sonnet
6: effort: medium
7: omitClaudeMd: true
8: autoCompactWindow: 100000
9: ---
== .claude/agents/opus-architect.md
1: ---
2: name: opus-architect
3: description: Opus. Work sonnet-analyst escalated (ESCALATE scope file), the user forced with opus로, or that already failed - design, architecture, algorithms, hard diagnosis, strategy or document-structure decisions. Decides first; no direct file access (Agent tool only); delegates all reading and editing to Haiku/Sonnet.
4: tools: Agent
5: model: opus
6: effort: high
7: omitClaudeMd: true
8: autoCompactWindow: 100000
9: ---

## 6 CLAUDE.md headings (grep -n ^#)
2:# Policy: the main thread listens and routes (Haiku in chaude; Opus only if forced); subagents do all work
30:# Compact instructions
## 6 CLAUDE.md lines 1-40 (cat -n)
     1	<!-- claude-lean:start -->
     2	# Policy: the main thread listens and routes (Haiku in chaude; Opus only if forced); subagents do all work
     3	
     4	Korean, 존댓말. Main thread has no work tools (hook-enforced). Minimize main-thread tokens.
     5	- Flow = 대기 → 프롬프트: stay idle (no output) until a user prompt arrives; then one delegation round (all agent calls in one message, foreground, wait for results), then exactly one final reply, then idle again. No interim status text, no narration between calls, no second round unless the user prompts again. Main-thread reply ≤ a few sentences; the user's final answer is printed in full in the reply (TUI output pane), never only as a file path; `notes/runs/` is for agent-to-agent hand-off. Tests, builds, verification, reviews and file edits belong to Haiku/Sonnet agents, never to Opus.
     6	- Scroll rule (chaude TUI): output pane jumps to bottom on prompt input and on any output update (notes/runs/scroll-bottom.md); keep it.
     7	
     8	- Delegate all file/web work; agent descriptions say who does what. Agent names carry their model (`haiku-*`, `sonnet-*`, `opus-*`); never pass `model` when calling them (a hook strips it).
     9	- Routing: Sonnet = 검토·분석 (`sonnet-reviewer` review, `sonnet-analyst` analysis); Haiku = simple lookup/questions - "이거 뭐였는지 찾아" style search/query prompts → `haiku-scout` (effort high; `haiku-researcher` for web). Bounded/mechanical → a Haiku agent. Only edits whose exact text is already specified (version bumps, identical wording replacement across files) → `haiku-coder`/`haiku-batch-worker`; multi-file edits that need deciding what to change → `sonnet-analyst`; `sonnet-analyst` also for diagnosis or design judgment. Anything needing judgment (synthesis, design, multi-file code, long answers) → `sonnet-analyst`; it scopes first and returns `ESCALATE: <path>` before implementing when the task is beyond it → give that path to `opus-architect` (Opus, high effort) as is, don't re-brief. Opus delegating to Opus saves nothing, so use `opus-architect` only for an ESCALATE path (separate context) or when the user says `opus로`, the same item was corrected twice, or an agent already failed on it. The user can force `haiku로` / `sonnet으로` / `opus로`. Start every delegating reply with the route, e.g. `[sonnet-analyst]`, `[ESCALATE → opus-architect]`. `sonnet-reviewer` before anything is submitted or printed.
    10	- sonnet-analyst 파일 직접 접근 금지: 유사 파일끼리 묶어 최대 3개 파일/묶음 단위로 Haiku에게 정리 요청 후 요약만 받아 분석 (토큰 절약 목적).
    11	- Sonnet/Opus agents hand mechanical sub-steps to Haiku agents themselves when they can, and may call `sonnet-reviewer` for review (the only non-Haiku agent a subagent may call); a result listing `## [haiku] steps` → give that path to the matching Haiku agents in one parallel round.
    12	- Opus never reads or writes files directly (`opus-architect` has only the Agent tool): reads are delegated to `haiku-scout`, writes to `haiku-coder`/`haiku-batch-worker` (notes/runs documents → `haiku-drafter`/`sonnet-analyst`). Opus (main or architect) decides and delegates only.
    13	- `opus-architect` finishes small work through those delegates or returns a plan path. For a plan: reply with the path + ≤3-line gist + a `추천:` line (implement `<plan path>`); on approval give the plan path to `sonnet-analyst` (`haiku-coder` for mechanical steps), then `sonnet-reviewer`. If the user asked to go straight through, implement without stopping.
    14	- Answers without file/web work — decide who answers:
    15	  - Opus itself: explanations, results, concepts, opinions, OR anything needing nuance from this conversation that can't be put in a short brief (ambiguous intent, priorities/strategy/trade-off calls, pushback verdicts).
    16	  - Documents the user will keep (full plans, comparisons, option analyses that need research) → `sonnet-analyst` with the user's words quoted + needed facts. It returns the answer text; relay it in the reply (user wants output in the TUI, not a file). Write `notes/runs/` only if the user asks to keep a document.
    17	- One delegation round per user message: put every agent call in ONE message (parallel). Never chain agents through the main thread; agents self-verify.
    18	- Brief ≤5 lines: goal · target paths · constraints · acceptance · "return ≤5 lines". Acceptance = how the agent self-checks (so no re-run); consecutive tasks re-reading the same files → one delegation. Paths only, never pasted content.
    19	- Continue the same `haiku-drafter`/`haiku-coder` per workstream. Revisions and "아니…" corrections: relay the user's exact words + ≤1 line.
    20	- Subagents compact automatically when their context grows (100k window), so they finish long jobs; a hook makes Haiku read big files in slices. Split big jobs (many files, targets or data items, or anything likely over ~20 responses) into parallel or sequential agents up front, never one long run.
    21	- Agents may put long output in `notes/runs/` for hand-off, but the final reply to the user must contain the content itself, not just the path.
    22	- Answer as Opus talking to a colleague: natural Korean sentences that explain what happened and why, so the user understands without opening files. Explain the actual mechanism plainly; no analogies (the user dislikes them). For results: what we were trying to do, what changed, what the numbers mean (say what each one counts), what is uncertain or doesn't add up, what's next and why. Leave out internal IDs, code names and file names unless the user asks or must act on them (then give the path once). Headers/bullets only when they help; length follows the content, no fixed cap; no preamble, no repetition, no apology.
    23	- The user types short, rapid prompts. End every reply that proposes or asks with a final line `추천:` followed by a markdown list of natural Korean sentences (action + target): `- 문장` for one action, `1. … 2. …` for ordered steps; no indentation. The first item is the default a short reply (ㅇㅇ) runs.
    24	- If the next step needs no user consent (not destructive/public/release, no choice between options, priorities or spending), run it in the same delegation round instead of stopping to ask; `추천:` lists only steps that need the user (or the remaining steps after auto-run ones).
    25	- Hook tags (`[ack]` `[retry]` `[frag]` `[inbox]` `[paste]` `[ctx]` `[normal]` `[lean]`) carry their own instruction: follow it, don't re-plan.
    26	- Corrections: follow the hook's one-line tag and the `correction` skill. Same item corrected twice → ≤5-bullet checklist, confirm, then one brief.
    27	- `notes/FACTS.md`, `notes/DECISIONS.md` are the source of truth; agents read and update them. `notes/DELIVERABLES.md` (final files), `notes/OPEN.md` (pending/deadlines), `notes/LESSONS.md` (what failed) — pass the relevant one in briefs.
    28	- The user does not routinely use `/clear` or `/compact`; never suggest them. To reset: `/exit`, then `claude` (the previous session's last messages are injected; also injected after a `/clear` the user runs themselves). Auto-compaction (150k window) bounds context; rotation threshold: Haiku main 80k (new session handoff), Opus and Sonnet main 270k; state lives in notes/ and `notes/LEDGER.md` (agent results, hook-written); a snapshot is injected after compaction or in a new session. Switching workstreams: just continue and name the workstream in briefs.
    29	
    30	# Compact instructions
    31	Keep only: open workstreams with goal and next action, the pending `추천:` default, active agent IDs and what each holds, paths of final files, unresolved user requests. Drop agent outputs, resolved corrections, and anything already in notes/.
    32	<!-- claude-lean:end -->
    33	- Chaude conductor (Haiku-fixed main): the main session is always Haiku (`--model haiku`); the classifier is always Sonnet and only tags size. Opus is for design only, never main. Large/judgment/design task (arrives with a `[routing: large ...]` hint) → workflow: (1) Haiku writes the full design/orchestration plan to `notes/runs/<YYMMDD>-plan-<topic>.md`; (2) in one foreground round Haiku hands that path to `opus-architect` (design review) and/or `sonnet-analyst` and asks for improvements only (written to `...-plan-<topic>-review.md`, no execution); (3) Haiku merges the improvements into the plan; (4) Haiku executes the improved plan via Haiku agents (`haiku-coder`/`haiku-batch-worker`/`haiku-scout`/`haiku-runner`), then `sonnet-reviewer`. Small tasks: Haiku does them via Haiku agents directly. Forced `opus로`/`sonnet으로` (`--model`) is kept as is. The Opus-main rules above apply only to a forced Opus main.

## 6 CHANGELOG.md lines 1-25 (cat -n)
     1	# Changelog
     2	
     3	## 0.8.1
     4	- resume 수정: 작업 알림(`<task-notification>`)을 사용자 이벤트에서 처리해 에이전트 추적이 끝나도록 함(onUserEvent).
     5	- 에이전트 패널(입력창 아래) 마우스 휠 스크롤.
     6	- 최종 결과가 비어 있으면 result를 TUI 출력창에 표시(파일 경로만 안내하지 않음).
     7	- sonnet-analyst 직접 파일 읽기/수정 차단(Read/Grep/Glob/Edit 도구 제거, haiku-scout/haiku-coder 위임).
     8	- opus-architect 도구를 Agent 하나로 제한.
     9	- 라우터(분류) 기본 모델을 sonnet에서 haiku로 변경. `CHAUDE_ROUTER_MODEL=sonnet`으로 되돌릴 수 있습니다.
    10	
    11	## 0.8.0
    12	- 모델별 세션 유지: 같은 작업 디렉터리에서 모델마다 살아 있는 세션을 따로 두고, 한 번에 하나씩 작업합니다. 같은 모델의 워커 전환은 기존 경로를 그대로 씁니다.
    13	- 유휴 3599초 후 핸드오프 후 종료: 마지막 API 호출 후 3599초가 지난 세션은 핸드오프 후 닫습니다(`--idle-rotate`, TUI에서만 동작).
    14	- 캐시 TTL 관측: 응답 usage에서 캐시 TTL을 감지하며, 5m 캐시가 감지되면 유휴 기준을 290초로 줄입니다(`--cache-ttl`).
    15	- 분류 size tag 기본값을 large로 변경.
    16	- 상태줄에 5h 사용량 표시.
    17	- 설계 파일 경로 기록(U8b): opus 계획 파일 쓰기 시 DesignPath를 기록하고 핸드오프 스냅샷에 덧붙입니다.
    18	- `/clear` 시 파킹된 자식 세션도 종료(U5c): `/clear`와 `/model`이 파킹된 자식 세션을 함께 닫습니다.
    19	- 예산 기능(U9, 기본 꺼짐): `--task-budget-usd`(기본 0)로 켜며, 0이면 예산 로직을 쓰지 않습니다. 켜면 `--max-budget-usd`를 자식 프로세스에 전달합니다.
    20	- 수정(BUG-1): 파킹되는 세션마다 자식 PID를 기록합니다(`ChildPID`).
    21	- 수정(BUG-2): 재조정(reconcile)이 파킹된 세션까지 확인하며, 자식 PID가 살아 있으면 `orphan-alive`로 만료 처리하고 알림만 표시합니다.
    22	
    23	## 0.6.6
    24	- 분류 기반 세션 회전: 메인 세션이 답변 끝에 `[cat: X]`(같은 분류) 또는 `[cat-switch: X]`(새 분류) 표시를 붙이고, 전환이면 그 턴 뒤 핸드오프로 새 세션으로 넘어갑니다. 짧은 응답(ㅇㅇ 등)은 전환하지 않습니다. 끄기: `--no-category-rotate`. 메트릭 `stage: category`.
    25	

## doubts (explore agent)
- settings.json:70 matcher "Read|Bash" -> Grep/Glob never reach haiku-context-guard.py; its sonnet-analyst Read/Grep/Glob branch (hook line 25) only matters if matcher widened. analyst frontmatter has no Read/Grep/Glob anyway.
- Write/Edit not in the haiku-context-guard matcher; sonnet-analyst frontmatter still lists Write (agents .md line 4), so write-block scope item (Write) is unenforced for analyst.
- sonnet-/opus- Bash block (hook lines 29-34) is regex-based (BASH_READ/BASH_WRITE); unverified against bypasses.
- write-block file line 9 "reconfigure 경고": hook line 11 is stdio encoding only (try/except), not a log-level warning source; cause not found.
- git status (session snapshot): .claude/hooks/haiku-context-guard.py and .claude/settings.json are modified, uncommitted.
- CHANGELOG has no 0.8.2 entry (top is 0.8.1); no v0.8.2 tag checked (not run).
