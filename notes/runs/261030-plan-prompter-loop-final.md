# plan: haiku-prompter loop (final, supersedes options in 261030-plan-prompter-loop.md)
status: decided by opus-architect 261030; NOT implemented
source: notes/runs/261030-scope-prompter-loop.md
user verbatim: "사용자의 명령중 재귀 반복이 있다면 haiku-prompter로 답변 계속 확인하고 다시 질문하는 과정 만들어. haiku-prompter는 하위 에이전트나 다른 모델에게 결과를 받으면 Tui에 출력하고"
user rules: haiku 모델은 재귀/반복 처리 예외로 제외; 횟수·시각 지정 없으면 기본 3회; 횟수 또는 시간 지정되면 그대로 따름; 시각 지정 시 각 작업 종료 즉시 다음 작업 실행

## decisions
- design: B (loop inside haiku-prompter subagent). why: main keeps Flow (1 delegation round = 1 call to haiku-prompter, 1 final reply); A would need a CLAUDE.md Flow exception and multiple main replies.
- Flow conflict: none under B. CLAUDE.md:5/:17 unchanged; only a routing line is added.
- TUI output: prompter handback = per-round log + final answer; main prints it in full (already required by CLAUDE.md:5). per-round progress visible live in the existing agent panel (nested Agent calls). Go code untouched. real-time per-round text in output pane = out of scope (would need A or Go work; phase 2 only if user asks).
- agent-model-pin.py edit: REQUIRED. :41 denies any subagent -> non-haiku target, so prompter -> sonnet/opus is blocked today; and no self-recursion guard exists.
- prompter -> sonnet/opus: ALLOWED (user said "다른 모델에게 결과를 받으면"). targets: sonnet-analyst, sonnet-reviewer, opus-architect, haiku-* except haiku-prompter. this is a scoped D11 exception for caller haiku-prompter only; sonnet/opus called by prompter still may call only haiku-* (:31-36 unchanged).
- "haiku 제외" meaning (default, user to confirm): haiku-* worker results are NOT re-asked; loop/re-ask applies only when the target is sonnet-analyst / sonnet-reviewer / opus-architect. a repeat request whose work is haiku-only -> main handles it normally (no prompter). haiku main thread never loops itself. alt meaning (haiku allowed to loop as exception) = flip one line: LOOP_TARGETS in haiku-prompter.md includes haiku-*.

## round-count rules (encode verbatim in prompter)
- mode=count: user gave a count (예: 5번, 5회, 다섯 번) -> run exactly N rounds; do not stop early on criteria met.
- mode=deadline: user gave a time (예: 5시 20분까지, 30분 동안) -> deadline = absolute epoch (duration -> now + duration via `date +%s`); start next round immediately after previous ends; do not start a round when now >= deadline.
- both count and time given -> stop at whichever comes first.
- mode=default: neither given -> max 3 rounds; stop early when all done-criteria met.
- any mode: 2 consecutive failed rounds (agent error/deny/empty) -> stop, report.
- round 1 = original task; round k>1 = re-ask quoting unmet criteria + previous answer excerpt (<=800 chars); in count/deadline mode with all criteria met, round k asks to continue/repeat the task as the user worded it.

## steps
### 1. new .claude/agents/haiku-prompter.md (haiku-coder)
frontmatter:
---
name: haiku-prompter
description: Haiku. Loop owner for repeat/recursive requests (반복, 재귀, 계속, 될 때까지, N번, ~까지): calls the target agent per round, checks the answer, re-asks; returns per-round log for the TUI.
tools: Agent, Read, Bash
model: haiku
effort: high
omitClaudeMd: true
autoCompactWindow: 100000
---
body (English, terse; include):
- input from main: user verbatim, target agent, task brief.
- LOOP_TARGETS: sonnet-analyst, sonnet-reviewer, opus-architect. if target is haiku-* -> one call, no re-ask, return result.
- never call haiku-prompter. Bash only for `date +%s` / `date`.
- step 0: write done-criteria (<=5 bullets) from user verbatim; parse mode per round-count rules (copy rules section verbatim).
- each round: one foreground Agent call to target; check each criterion met/unmet; log line.
- handback format (Korean 존댓말, printed in full by main):
  `## 반복 기록` then per round `- N회차 (<agent>, HH:MM~HH:MM): 충족 x/y · 미충족: ...`
  `## 최종 답변` then the last (or best) answer in full, not a path.
  `종료 사유: 횟수 도달 | 시각 도달 | 기준 충족 | 연속 실패`
### 2. .claude/hooks/agent-model-pin.py (haiku-coder)
- insert BEFORE line 31 (so it runs before lean_off() at :37, same as D11 plan D5):
  - if tool in (Agent, Task) and agent_id and target == "haiku-prompter": deny, reason "haiku-prompter는 메인 스레드에서만 호출할 수 있습니다 (재귀 금지)."
  - if tool in (Agent, Task) and agent_id and agent_type == "haiku-prompter": allow (exit 0 without output) when target in {"sonnet-analyst","sonnet-reviewer","opus-architect"} or (target startswith "haiku-" and target != "haiku-prompter"); else deny with reason "haiku-prompter는 sonnet-analyst/sonnet-reviewer/opus-architect/haiku-* 만 호출할 수 있습니다."
- update docstring lines 8-9 with one line on the prompter exception.
- keep :31-36 and :41-50 behavior for all other callers unchanged; ensure haiku-prompter is not in BLOCKED.
### 3. CLAUDE.md (haiku-coder)
- add one line after line 11:
  `- Repeat/recursive requests (반복, 재귀, 계속, 될 때까지, N번/N회, ~까지/~동안) whose work needs sonnet/opus → one call to \`haiku-prompter\` (pass user verbatim + target agent); it loops (default 3, given count/time followed exactly, next round right after the previous when a time is given) and the main prints its handback in full. Haiku-only work is not looped. Still one delegation round.`
### 4. notes/DECISIONS.md (haiku-batch-worker)
- append: `- [routing] 261030 user: repeat/recursive requests -> haiku-prompter loop (B); prompter may call sonnet-analyst/sonnet-reviewer/opus-architect (scoped D11 exception); no recursion into haiku-prompter; default 3 rounds, count/time followed exactly; "haiku 제외" = haiku-* results not re-asked (pending user confirm). plan notes/runs/261030-plan-prompter-loop-final.md`
### 5. CHANGELOG.md (haiku-batch-worker): one line under unreleased/0.8.2: `haiku-prompter: 반복/재귀 요청 루프 담당 에이전트 추가, agent-model-pin 예외`.

## edge cases
- deadline already passed or < now -> 0 rounds, report "시각 이미 지남", ask nothing.
- deadline across midnight ("1시까지" at 23:00) -> next day.
- long deadline: foreground subagent may hit tool/session timeout; prompter logs completed rounds in each step so partial log survives; note limit in handback if cut.
- target denies/errors -> counts as failed round.
- user gives count 0 or nonsense -> default mode.
- answer huge -> 최종 답변 in full, round log entries <=1 line each.

## acceptance checks (haiku-checker)
- python3 -c "import ast;ast.parse(open('/Users/leekwangseok/PycharmProjects/claude-harness/.claude/hooks/agent-model-pin.py').read())"
- pipe JSON into the hook (python3 .claude/hooks/agent-model-pin.py) and confirm:
  a) {"tool_name":"Agent","agent_id":"x","agent_type":"haiku-prompter","tool_input":{"subagent_type":"sonnet-analyst"}} -> no deny
  b) same with subagent_type "haiku-prompter" -> deny
  c) {"tool_name":"Agent","agent_id":"x","agent_type":"sonnet-analyst","tool_input":{"subagent_type":"haiku-prompter"}} -> deny
  d) agent_type "haiku-coder" -> subagent_type "sonnet-analyst" -> deny (unchanged)
  e) agent_type "sonnet-analyst" -> "opus-architect" -> deny (unchanged)
  f) no agent_id (main) -> "haiku-prompter" -> no deny
- grep -n 'haiku-prompter' CLAUDE.md .claude/agents/haiku-prompter.md notes/DECISIONS.md -> each >=1
- go test ./internal/... unchanged pass (Go untouched; run only to confirm)

## must NOT change
- CLAUDE.md:5 and :17 Flow wording; Go code under internal/; other agent files; sonnet/opus -> haiku-* only rule for their own calls.

## open (user only)
- "haiku 제외" meaning: default = haiku-* 결과는 재질문 안 함. 반대 의미(haiku도 반복 허용)면 LOOP_TARGETS 한 줄 수정.
