# v0.8.2 write-block: S1..S13 steps extracted from final plan
source: notes/runs/261030-plan-v082-write-block-final.md (sections 2, 3.2, 4, 5, 7, 8, 9, 10, 11, 12, 14)
status: extraction only. no repo file edited.

## order and dependencies
- order: S0 -> G1 (main) -> S1 -> S2 -> S3..S12 -> S13 -> G2 (main). stop after G2. no commit, tag, push, release.
- G1: main session live check, one sonnet-analyst Read; record agent_type. if absent: STOP, ask user (사용자 확인 필요 F).
- S2 depends on S1 (new spawn after RETURN rule added).
- S3 (haiku-context-guard.py) and S4 (agent-model-pin.py) independent files, both haiku-coder.
- S5 covers the 6 other hooks only (S4 covers agent-model-pin.py reconfigure). no overlap.
- S7 acceptance uses RETURN rule from S1.
- S10 acceptance needs escape hatch name, depends on S3 decision.
- S12 depends on S11 hit list.
- S13 depends on S1..S12. G2 depends on S13.

## S1 (haiku-batch-worker): RETURN rule in 8 files
- files: .claude/agents/haiku-{scout,checker,drafter,batch-worker,runner,researcher,extractor,coder}.md
- edit: add one line after the existing "Return" rule:
  "If the brief contains `RETURN: verbatim <=N chars`, the final message is verbatim excerpts with path:line, up to N chars (max 6000); the 300-char cap does not apply."
- verify: `grep -l "RETURN: verbatim" .claude/agents/haiku-*.md` = 8 files; each contains "max 6000".
- haiku tool lists unchanged (haiku-researcher keeps WebSearch/WebFetch).

## S2 (haiku-scout): acceptance call
- brief: "Read notes/runs/261030-plan-v082-write-block.md section 3.2. RETURN: verbatim <=4000 chars."
- verify: final msg >= 1000 chars of quoted lines with path:line.

## S3 (haiku-coder): rewrite .claude/hooks/haiku-context-guard.py
- matcher (settings, see S6): Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell
- before: matcher Read|Bash; BASH_READ/BASH_WRITE/PS_FILE regex; sonnet-analyst Read branch; analyst/reviewer exemption; allow_large Haiku deny.
- after rules (in order):
  1. stdin reconfigure: getattr(_s, "reconfigure", None); call only if present; try/except kept.
  2. sys.dont_write_bytecode = True
  3. whole body in try/except (fail-closed). on exception: raw stdin matches "agent_id" AND does not match "agent_type"\s*:\s*"haiku- -> exit 2, stderr = DENY_MSG. else exit 0.
  4. parse JSON. no agent_id -> exit 0 (main thread, D7).
  5. agent_id set and agent_type empty -> deny.
  6. agent_id set and agent_type not startswith "haiku-":
     - env CLAUDE_HARNESS_ALLOW_SONNET_IO == "1" -> exit 0 (escape hatch).
     - tool in FILE_TOOLS or tool in (Bash, PowerShell) -> deny(DENY_MSG).
     - else exit 0 (e.g. Agent tool). lean_off() irrelevant here.
  7. agent_type startswith "haiku-": if lean_off() -> exit 0. tool == Read and allow_large truthy -> deny("allow_large is disabled for Haiku agents. Read in slices with offset/limit, or grep first.").
  8. else exit 0.
- FILE_TOOLS = ("Read", "Write", "Edit", "MultiEdit", "NotebookEdit", "Grep", "Glob")
- ALLOW_IO = ("haiku-",)   (no ALLOW_IO_EXACT; see statusline-setup below)
- DENY_MSG (verbatim): "sonnet/opus/비-haiku 에이전트는 파일·셸 직접 접근 금지 (v0.8.2). haiku-scout(읽기)/haiku-batch-worker(쓰기)/haiku-coder(코드)에 위임하세요."
- deny output: hookSpecificOutput permissionDecision deny JSON (same as before), exit 0. exception path exit 2.
- remove: BASH_READ, BASH_WRITE, PS_FILE, _SEP, analyst/reviewer exemption.
- verify: `python3 -I -m py_compile` OK, then remove __pycache__; `grep -c "BASH_READ\|BASH_WRITE\|PS_FILE"` = 0; `grep -c "_s.reconfigure("` = 0; `grep -c "getattr(_s"` >= 1.

## S4 (haiku-coder): .claude/hooks/agent-model-pin.py
- new check BEFORE `if lean_off() or ...` (D11):
  - data.agent_id set AND caller agent_type in {sonnet-reviewer, sonnet-analyst, opus-architect} AND tool_input.subagent_type not startswith "haiku-" -> deny: "sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출할 수 있습니다 (v0.8.2)."
- sonnet-reviewer -> sonnet-reviewer denied by this rule.
- existing rules unchanged (model strip for haiku-/sonnet-/opus-; built-in BLOCKED; claude-code-guide pin).
- docstring: "Called from inside a subagent ... sonnet-reviewer" -> "haiku-* only".
- reconfigure: getattr fix (same form as S3).
- verify: py_compile OK; `grep -c "_s.reconfigure("` = 0; new check line index < lean_off line index.

## S5 (haiku-batch-worker): reconfigure getattr in 6 hooks
- files: context-alert, correction-guard, main-thread-guard, ledger, session-context, style-adapter (.claude/hooks/*.py)
- edit: replace `_s.reconfigure(encoding="utf-8")` with getattr form. no other change.
- verify: `grep -n "_s.reconfigure(" .claude/hooks/*.py` = 0 hits; py_compile all hooks OK.

## S6 (haiku-coder): .claude/settings.json line 70
- before: `"matcher": "Read|Bash",`
- after: `"matcher": "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell",`
- other entries, permissions, env unchanged. agent-model-pin entry `Agent|Task` unchanged.
- then: `go test ./internal/settings/...` and `go run ./cmd/chaude gen-settings windows`
- verify: settings JSON valid; `grep -c "Read|Bash"` in settings.json = 0; new matcher present; go test PASS; gen-settings output has new matcher.

## S7 (haiku-coder): sonnet-analyst.md, sonnet-reviewer.md, opus-architect.md
- re-grep line numbers first (L3).
- sonnet-analyst: `tools: Write, Bash, WebSearch, WebFetch, Agent` -> `tools: Agent`. "Scope first: you have no Read/Grep/Glob/Edit." -> "Scope first: you have no Read/Write/Edit/Grep/Glob/Bash/WebSearch/WebFetch." file reads via haiku-scout (RETURN: verbatim <=4000 chars); web via haiku-researcher (same RETURN); answer draft via haiku-drafter (user-voice prose); verbatim save via haiku-batch-worker (M4).
- sonnet-reviewer: `tools: Read, Grep, Glob, Bash` -> `tools: Agent`. add: "Read nothing directly: haiku-scout reads and returns excerpts with RETURN: verbatim <=4000 chars." charcount and git diff via haiku-checker (RETURN: verbatim <=4000 chars). issue list via haiku-batch-worker.
- opus-architect: tools unchanged (Agent). "No direct file access (v0.8.1):" -> "No direct file access (v0.8.2):". "reading/search -> haiku-scout (or sonnet-analyst for synthesis)" -> "reading/search -> haiku-scout (RETURN: verbatim <=4000 chars)". "plans/answers written to notes/runs/ -> haiku-drafter/sonnet-analyst" -> "... -> haiku-batch-worker". plan save via haiku-batch-worker verbatim. plan review: haiku-scout reads (RETURN: verbatim <=4000 chars), haiku-batch-worker writes concrete improvements only.
- verify: `grep -n "^tools:"` on the 3 files = `tools: Agent` only; `grep -c "WebSearch\|WebFetch\|Bash"` in tools lines = 0; each file has "RETURN: verbatim <=4000 chars"; `grep -c "v0.8.1" opus-architect.md` = 0.

## S8 (haiku-coder): CLAUDE.md lines 10-12
- line 10 after: "- sonnet/opus 에이전트 파일 직접 접근 전면 금지 (Read·Write·Edit·Grep·Glob 및 Bash/PowerShell 포함, 훅 강제): 파일 읽기는 haiku-scout(RETURN: verbatim <=4000 chars), 쓰기는 haiku-batch-worker/haiku-coder, 웹은 haiku-researcher가 수행."
- line 11: remove "may call sonnet-reviewer for review (the only non-Haiku agent a subagent may call)"; replace with "hand work only to haiku-* agents".
- line 12: remove haiku-drafter/sonnet-analyst as writers for notes/runs; state haiku-batch-worker (verbatim) and haiku-drafter (user-voice prose only).
- lines 8-9 unchanged.
- verify: `grep -c "may call"` near sonnet-reviewer = 0; `grep -c "haiku-drafter\`/\`sonnet-analyst"` = 0.

## S9 (haiku-batch-worker): notes/DECISIONS.md
- append exactly one line, format `- [agents] ...`. content per final sec 9: "v0.8.2 user: sonnet/opus/비-haiku 서브에이전트 파일·셸 전면 차단 (haiku-* 허용 목록); update to 35: sonnet-reviewer는 haiku-*만 호출 가능 (subagent caller); sonnet-reviewer/analyst/opus tools = Agent only."
- verify: exactly one new line.

## S10 (haiku-coder): CHANGELOG.md
- insert above `## 0.8.1`:
  ## 0.8.2
  - sonnet/opus(비-haiku) 서브에이전트의 파일·셸 직접 접근을 훅으로 전면 차단; 모든 읽기/쓰기는 haiku 에이전트 경유
  - haiku-context-guard 매처 확장: Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell
  - sonnet-analyst/sonnet-reviewer tools: Agent 전용
  - sonnet/opus 서브에이전트는 haiku-* 에이전트만 호출 가능 (agent-model-pin)
  - haiku 에이전트: `RETURN: verbatim <=N chars` 지정 시 원문 발췌 반환 허용 (max 6000)
  - 훅 Pyright 경고 수정(reconfigure getattr, 모든 훅)
  - 탈출구: 환경변수 CLAUDE_HARNESS_ALLOW_SONNET_IO=1 (기본 off)
  - harness-only change (cmd/chaude version 0.8.1 유지)
- docs/claude-lean/CHANGELOG.md untouched.
- verify: `head -3` shows `## 0.8.2` then bullets; "harness-only change" present; "CLAUDE_HARNESS_ALLOW_SONNET_IO=1" present.

## S11 (haiku-scout): read-only L5 grep
- grep "sonnet-analyst", "sonnet-reviewer", "notes/runs" write routing in internal/, cmd/, README.md, docs/, .claude/hooks/main-thread-guard.py
- output: hit list path:line saved to verify file; no edits.

## S12 (haiku-coder): fix S11 hits
- only hits telling sonnet/opus to write files directly.
- verify: re-run S11 grep = 0 write-routing hits. if S11 had 0 hits: record "no change", skip.

## S13 (haiku-scout): verification
- pyright .claude/hooks/ -> 0 errors, 0 warnings.
- `grep -n "_s.reconfigure(" .claude/hooks/*.py` -> 0.
- `grep -n "Read|Bash" .claude/settings.json` -> 0; `grep -n "Read|Write|Edit|MultiEdit|NotebookEdit|Grep|Glob|Bash|PowerShell" .claude/settings.json` -> 1.
- `grep -n "^tools:" .claude/agents/sonnet-*.md .claude/agents/opus-*.md` -> tools: Agent only.
- `grep -l "RETURN: verbatim" .claude/agents/haiku-*.md` -> 8.
- `grep -n "BASH_READ\|BASH_WRITE\|PS_FILE" .claude/hooks/haiku-context-guard.py` -> 0.
- `grep -n "may call" CLAUDE.md` -> no sonnet-reviewer call permission.
- `grep -n "0.8.1" docs/claude-lean/CHANGELOG.md` -> unchanged.
- `git status --short` -> no commit/tag; only files in sec 2 + notes/runs/.
- hook sample rows (sec 11): 23 rows + pin rows P1-P5 expected as listed in final sec 11.
- results saved to verify file.

## Bash option 2
- NOT FOUND in final plan. no "option 2" for Bash anywhere in 261030-plan-v082-write-block-final.md.
- decision recorded: D1 + review H3: Bash and PowerShell denied entirely for blocked agents (all commands, no regex). haiku agents unaffected.
- related "option B" = reviewer tools Agent only (D2, 확인 #2 권장안); original plan 261030-plan-v082-write-block.md:203 lists options A/B/C for reviewer, plan uses B.

## env escape hatch
- name: CLAUDE_HARNESS_ALLOW_SONNET_IO=1; default off.
- behavior: in haiku-context-guard.py rule 6, checked before deny; skips block for blocked agents only (D5).
- documented in CHANGELOG (S10). not in CLAUDE.md.
- status: 사용자 확인 필요 B: keep or remove. plan includes it per 확인 #3 권장안.
- row 21 expects allow with env set; row 22 expects deny with lean_off true (default).

## statusline-setup
- decision: NO exemption in plan. ALLOW_IO_EXACT not included. only ALLOW_IO = ("haiku-",).
- row 16: agent_type statusline-setup, Edit -> expected deny (literal rule).
- status: 사용자 확인 필요 A open. review H2 proposed ALLOW_IO_EXACT = {"statusline-setup"} and allow Edit for it. if adopted, row 16 flips to allow.

## reconfigure / getattr
- cause (inferred): sys.stdin/stdout/stderr typed TextIO; typeshed TextIO lacks reconfigure. Pyright warning text from user not captured (사용자 확인 필요 D).
- fix: `_reconf = getattr(_s, "reconfigure", None)`; call only if present; try/except kept. runtime same.
- scope: 8 hooks with `_s.reconfigure(` (D12, review L1): haiku-context-guard.py (S3), agent-model-pin.py (S4), 6 others (S5).
- acceptance: pyright .claude/hooks/ 0 errors 0 warnings; grep 0 hits.

## doubts found in final plan (not fixed here)
- sec 14 E text: "CHAUDE_MAIN_MODEL" (typo, likely CHAUDE or CHAUDE_MAIN_MODEL; not in any step).
- sec 9 DECISIONS line contains "update to 35:" (odd wording, copied as written).
- sec 14 미정 1 and 2 still open (/usage model-call nature; budget display location). not needed for S1-S13.
- Go version stays 0.8.1 (D13); S6 runs go test but no Go code edited.
