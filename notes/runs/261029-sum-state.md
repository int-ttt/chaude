# 261029 sum-state
source: notes/DECISIONS.md (41 lines), notes/FACTS.md (0 facts, header only), CLAUDE.md (32 lines), git diff (uncommitted), internal/harness file list
rule: newest DECISIONS line wins on conflict

## git diff --stat (uncommitted, tracked)
- CLAUDE.md 3 +-, internal/harness/repl.go 18 changed lines, internal/harness/tui.go 6, notes/DECISIONS.md +3, notes/LEDGER.md +1, notes/metrics/2026-10-29.jsonl +57
- total: 6 files, 81 insertions, 7 deletions
- untracked: notes/runs/261029-* (plan/review/scout/diag files) and notes/runs/release-rebuild.md

## uncommitted code change (repl.go, tui.go)
- repl.go: const opusSonnetThreshold = 270_000 (line ~27)
- repl.go: new func effectiveThreshold() int (line 689): model opus|sonnet -> 270000; else r.a.Threshold
- repl.go: handle() (line 696) auto-rotate check now uses thr := r.effectiveThreshold() (line ~756)
- repl.go: Main flag -threshold default 270_000 -> 80_000 (line ~860)
- tui.go: welcome() (line 222) "rotate at" uses effectiveThreshold (line ~237)
- tui.go: runSlash /status note (line ~878) uses effectiveThreshold
- tui.go: statusLine() (line 957) ctx denominator uses effectiveThreshold (line ~959)
- effect: haiku main rotates at 80k; opus/sonnet main at 270k (matches DECISIONS L41)
- not in diff: no test added for effectiveThreshold (unverified)

## relevant decisions (DECISIONS.md line numbers)
- L41 (2026-10-29, newest): rotation threshold Haiku main 80k (new session handoff); Opus and Sonnet main 270k. Overview cap (policy): Haiku 3,000 chars; Opus/Sonnet 6,000 chars (no code limit exists).
- L40 (2026-10-29): Opus main only for design turns; context-threshold switching only for Haiku main; Opus/Sonnet keep context. CONFLICT with L41 (Opus/Sonnet do rotate at 270k) -> L41 wins; code matches L41. Plan: notes/runs/261029-plan-hybrid-design.md
- L38 (2026-10-29): conductor main Haiku-fixed; Opus only for design. pickModel/pickTarget return haiku (forced --model sonnet|opus kept; forced opus keeps delegateHint). Large tasks: planHint (Haiku writes plan -> opus-architect/sonnet-analyst review, improvements only -> Haiku merges and executes). route() no restart when already haiku. Test: TestHaikuPlanWorkflow.
- L35 (2026-10-29): sonnet = 검토·분석 (sonnet-reviewer review, sonnet-analyst analysis); haiku = 간단한 조사·질문; 검색/조회 류 ("이거 뭐였는지 찾아") -> haiku-scout effort high; haiku-researcher high for web. agent-model-pin hook lets subagents call sonnet-reviewer.
- L39 (2026-10-29): sonnet-analyst reads no files itself; Haiku condenses similar-file groups (max 3 files/group), analyst gets summaries only (token saving).
- L36 (2026-10-29): all agents return brief result summary only (outcome · changed paths · PASS/FAIL, Korean 존댓말); details to notes/runs/.
- L33 (2026-10-29): TestClassifierLive (CHAUDE_LIVE=1) verified 6 prompts: tradeoff/pushback/correction/strategy -> opus, 2 mechanical -> haiku. Pre-L38 result; target now Haiku-fixed.
- L30-L32 (2026-10-29, conductor): classifier = one-shot `claude -p --model sonnet` per prompt (internal/harness/classifier.go, cwd tmpdir, no tools) -> terse {model,task}. Conductor never writes answers; target agent writes the Korean 존댓말 reply. conductor->agent prompt terse/machine-style (no honorifics). L32 supersedes L30 conductor-session rule. Acks and slash commands go to current session (no classify). Metrics stage=classify.
- L29 (2026-10-29): category rotation: main appends [cat: X]/[cat-switch: X] (<=500 tokens rule); switch rotates after turn; acks never switch (isAck); flag --no-category-rotate.
- L37 (2026-10-29): /usage works while busy (separate one-shot claude -p /usage, popup); other slash cmds while busy -> note "생각 중에는 /usage 외의 명령을 사용할 수 없습니다. 답변이 끝난 뒤 다시 입력해 주세요." Test: usagebusy_test.go.
- L13 (tui, 2026-10-28): slash cmds /help /clear (fresh session, no handoff) /status /model [name] (switch = rotate with handoff) /rotate /exit /quit; "/" shows autocomplete popup (up/down, tab, enter).
- L15 (tui, 2026-10-28): passthrough to child raw: /usage /cost /context /compact /config /mcp /agents /init /review /doctor. Unsupported (hidden): /memory /permissions. DRIFT: L15 says /resume unsupported, but code has /resume handler (tui.go:889, repl.go:834; likely 0.7.1 "/resume claude filter"). Not in L15: /import /mode /mouse /version are handled in code.
- L16 (2026-10-28): single rule 270k, superseded by L41.
- L19/L21/L23 (2026-10-28): idle handoff at 59:59 (--idle-rotate 3599s), CLAUDE_CODE_PROMPT_CACHE_TTL=1h. Unverified (TTL 5m vs 1h risk).
- L24-L28 (2026-10-28): research (haiku-researcher, haiku-scout) stays subagents; main.md replaces CLAUDE.md via SessionStart hook (plan 4-c, unverified).
- L5 structure: hooks = Python (.claude/hooks/*.py); REPL + scripts = Go (bin/harness).
- L12 tui: bubbletea/lipgloss TUI on TTY; HARNESS_PLAIN=1 or piped stdin = plain REPL (tests rely on it).

## specter
- no mention of "specter" in CLAUDE.md, DECISIONS.md, FACTS.md, internal/, .claude/ (grep).
- related untracked files only (not read): notes/runs/261029-plan-specter.md, 261029-plan-specter-review.md.
- decision: NOT FOUND.

## CLAUDE.md constraints (summary)
- Korean, 존댓말. Main thread has no work tools (hook-enforced); minimize main-thread tokens.
- Flow: idle until prompt; one delegation round (all agent calls in one message, foreground); one final reply; no interim text.
- Main reply <= a few sentences; details in notes/runs/, give path.
- Scroll rule (chaude TUI): output pane jumps to bottom on prompt input and output update (notes/runs/scroll-bottom.md). Keep.
- Agent names carry model (haiku-*, sonnet-*, opus-*); never pass `model` (hook strips it).
- Routing: sonnet-reviewer = review; sonnet-analyst = analysis; haiku-scout = lookup (effort high); haiku-researcher = web; bounded/mechanical -> haiku-coder/haiku-batch-worker; multi-file judgment -> sonnet-analyst; ESCALATE: <path> -> opus-architect as-is.
- opus-architect only for ESCALATE path, `opus로`, same item corrected twice, or agent failed twice.
- sonnet-analyst no direct file access; group similar files (max 3 per group) -> Haiku summarizes.
- Start delegating reply with route tag e.g. `[sonnet-analyst]`.
- Brief <=5 lines: goal · target paths · constraints · acceptance · "return <=5 lines".
- Reply ending with proposal/question: final line `추천:` + markdown list.
- Auto-run steps needing no user consent in same round.
- Never suggest /clear or /compact to user. Reset = /exit then claude.
- Haiku main rotation 80k (new session handoff); Opus/Sonnet main 270k; auto-compaction "150k window" stated in CLAUDE.md, but code default compact-window 400_000 (repl.go flag) -> DOUBT, mismatch.
- Chaude conductor: main always Haiku (--model haiku); classifier always Sonnet, only tags size; Opus design only. Large task ([routing: large ...] hint) -> plan -> review -> merge -> execute workflow.
- Note: CLAUDE.md line 32 says "Opus ... never main"; L40 says Opus main for design turns. Forced opus main is allowed (CLAUDE.md L2).

## TUI slash command handling (today)
Entry chain:
- tui.go Update (line 366): Enter key path -> m.key (line 472).
- tui.go key() Enter (~lines 636-675): if m.busy and "/" -> only "/usage" allowed (one-shot, usageSend); others note busySlashMsg (tui.go ~647-651). If busy and not "/" -> queue prompt.
- tui.go ~663-672: if "/" and popup suggestion differs from typed name -> complete; /model completes to "/model "; else m.runSlash(line) (line 672).
- tui.go m.sendPrompt(line) (line 706) -> m.submit channel -> repl loop; used for non-slash prompts (line 674).
- tui.go suggestions() (line 798): popup list; slashCmds table at tui.go ~151-158 (/help /clear /status /model ...).
- tui.go runSlash(line) (line 853): switch on f[0]:
  - 857 /quit,/exit -> m.quit() (line 952)
  - 859 /help (local note)
  - 865 /mouse (local toggle)
  - 873 /status (local note, ctx/threshold via effectiveThreshold)
  - 879 /model (no arg: show; with arg: switch = rotate with handoff)
  - 889 /resume (local)
  - 899 /import (local)
  - 905 /mode (local, setMode)
  - 924 /version (exec claude --version, local note)
  - 927 /clear,/rotate,/usage,/cost,/context,/compact,/config,/mcp,/agents,/init,/review,/doctor -> busy=true, append user block, m.submit <- line (sent to repl loop)
  - 936 default: unsupportedCmds (/memory, /permissions) note; else "unknown command ... (try /help)"
- tui.go statusLine() (line 957): ctx denominator effectiveThreshold.
- tui.go welcome() (line 222): threshold text.

repl.go (plain path and TUI submit both reach loop):
- repl.go loop() (line 795): reads lines; order of checks:
  - "/quit" break (820)
  - "/rotate" -> r.rotate("manual") (823)
  - "/mode" -> r.modeLine (827)
  - "/resume" -> r.resumeLine (834); "/import" -> r.importSession (837)
  - passthroughCmds[f[0]] -> r.passthrough(line) (842)
  - else r.handle(line) (852)
- repl.go passthroughCmds (line 763-764): /usage /cost /context /compact /config /mcp /agents /init /review /doctor.
- repl.go unsupportedCmds (line 767): /memory /permissions.
- repl.go passthrough(line) (line 770): sends slash to child as-is, no handoff; /usage special-cased (line 779/784).
- repl.go handle(line) (line 696): pollDrains; if Conductor -> r.route(line) (line ~699); route() (classifier.go:138) skips classify for acks (isAck) and "/" lines -> returns line as-is.
- /clear: not in loop dispatch; reaches handle(); clear() at repl.go:919 (unverified call path: grep for r.clear() found no direct caller; /clear handling in repl not confirmed).
- repl.go setConductor (932); modeLine (938).
- repl.go Main (line 855): flags; -threshold default now 80_000 (uncommitted); -conductor default true (line 875).
- Plain REPL (no TUI): HARNESS_PLAIN=1 or non-TTY.

## routing code (classifier.go / category.go)
- classifier.go:27 const delegateHint (forced opus main, simple side task -> haiku-scout etc.)
- classifier.go:31 const planHint (large/judgment -> Haiku plan workflow)
- classifier.go:38 pickModel(cur, topic, size): haiku-> haiku; opus -> opus, delegate only if topic=="new" && size=="small"; else cur
- classifier.go:49 pickTarget -> pickModel (Haiku always unless forced)
- classifier.go:65 parseRouteFull; :109 classify (one-shot Sonnet classifier); :138 route(); :193 switchSession
- category.go:10 categoryRule; :22 isAck; :39 categoryRotation; ackWords list at :18.

## existing test files in internal/harness (21 *_test.go)
acceptance_test.go, agentstore_test.go, agentview_test.go, batchinput_test.go, bold_test.go, category_test.go, conductor_test.go (TestClassifierLive L22 [CHAUDE_LIVE=1], TestHaikuPlanWorkflow L71), delegation_test.go, drain_test.go, handoff_test.go, history_test.go, idle_test.go, input_test.go, metrics_test.go, mode_test.go, panel_test.go, resume_test.go, resumeshift_test.go, unit_test.go, usagebar_test.go, usagebusy_test.go
- no test file for: haikupool.go, effectiveThreshold (uncommitted), TUI runSlash (no slash-dispatch test found in list by name; unverified).
- no classifier_test.go; classifier tests live in conductor_test.go.

## doubts / not found
- specter: not in DECISIONS/CLAUDE/FACTS/code.
- FACTS.md empty (no confirmed facts).
- Opus main policy: L40 "design turns only" vs CLAUDE.md line 32 "never main" (forced opus allowed) vs L38 "only design".
- /resume: L15 unsupported vs code handler present.
- compact window: CLAUDE.md "150k" vs code default 400_000.
- uncommitted threshold change: no test; L40 vs L41 conflict resolved by L41 (newest).
