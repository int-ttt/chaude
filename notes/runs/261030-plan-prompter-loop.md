# plan: haiku-prompter loop (DESIGN ONLY, awaiting user choice)
status: not implemented; CLAUDE.md and hooks untouched (policy change = user decision)
request(verbatim): "사용자 명령에 재귀/반복 요구가 있으면 haiku-prompter로 답변을 계속 확인하고 재질문하는 루프 구현; haiku-prompter는 하위 에이전트나 다른 모델에게 결과를 받으면 TUI에 출력하도록 구성"
scope: notes/runs/261030-scope-prompter-loop.md

## facts checked
- CLAUDE.md:5,17 Flow: one delegation round per user prompt, one final reply, no second round, no interim text.
- agent-model-pin.py L31-36: sonnet/opus subagents -> haiku-* only. L41-45: any subagent -> haiku-* only. So a haiku subagent CAN call haiku-*, CANNOT call sonnet-*/opus-*. No name whitelist: new haiku-prompter passes by prefix, model pinned from agent file.
- haiku-prompter -> haiku-prompter self-recursion is currently NOT blocked by any hook.
- main-thread-guard.py: no round counter. No Go code limits Agent calls per prompt (classifier.go hints are prompt text only).
- Subagents cannot write to the TUI reply directly; only the main final reply (and the harness agent panel, agentview.go, maxAgentLog=400) are visible.
- haiku agents today have no Agent tool; haiku-prompter needs `tools: Agent, Read` (Read for scope/result files).

## options
A: Flow exception in CLAUDE.md ("user says 반복/재귀/될 때까지 -> main may run up to N=3 rounds"). Main itself re-asks.
  + simple, each round visible in TUI. - breaks one-round rule, main (Haiku) re-delegates = more main-context tokens; interim text rule also must relax; classifier hints need wording.
B: loop inside haiku-prompter. Main keeps one round (calls haiku-prompter once); prompter loops: delegate -> check result vs done-criteria -> re-ask, max N=3; returns per-round log + final answer; main prints it in full.
  + Flow literally kept; cost isolated in subagent. - prompter can only call haiku-* (hook) -> "다른 모델" (sonnet/opus) needs hook change; live per-round TUI output only via agent panel, not reply; new self-recursion guard needed (hook edit).
C: do not add. Keep current: user re-prompts manually; sonnet-analyst/opus-architect already self-verify.
recommendation: B (minimal), with hook changes deferred to user approval.

## B steps (only after user picks B)
1. haiku-coder: new .claude/agents/haiku-prompter.md. frontmatter: name: haiku-prompter; model: haiku; tools: Agent, Read; description: "Haiku. 반복/재귀 요구 루프 담당: 하위 에이전트 결과를 완료 기준과 비교, 미충족 시 재질문(최대 3회), 매 라운드 결과 요약 반환."
   body rules: (a) derive done-criteria from user verbatim first, write them as checklist; (b) each round: one Agent call to a haiku-* agent, quote prior result gaps in re-ask; (c) stop when all criteria met or round 3; (d) never call haiku-prompter; (e) handback format: `## round k` (agent, ask, result <=10 lines, unmet items) per round, then `## final` full answer, then `unmet:` list or `none`.
2. CLAUDE.md (user-approved wording only): add one line "반복/재귀/계속/될 때까지/loop 요구 -> call haiku-prompter once in the round; print its handback in full." No Flow round-count change.
3. agent-model-pin.py (user-approved): deny caller==haiku-prompter and target==haiku-prompter ("haiku-prompter는 자기 자신을 호출할 수 없습니다"). Optional, only if user wants 다른 모델: allow caller haiku-prompter -> sonnet-analyst/sonnet-reviewer (not opus).
4. Optional Go: agentview shows nested child results live already? haiku-scout verify agentview.go renders nested (depth>1) agent results; if not, separate plan.
detection: main judgment on keywords 반복, 재귀, 계속, 될 때까지, 확인될 때까지, loop, until; no new UserPromptSubmit hook (style-adapter/correction-guard/context-alert untouched).

## edge cases
- vague done-criteria -> prompter asks nothing of user mid-loop; reports criteria it assumed in `## final`.
- round 3 still unmet -> return unmet list, no 4th round.
- sub-agent failure/denied -> count as a round, record reason.
- token cost: N=3 cap; each round result summarized <=10 lines to keep prompter context small.

## acceptance checks (haiku-checker)
- agent file exists, frontmatter tools include Agent, model haiku.
- hook test: haiku-prompter -> haiku-prompter denied; haiku-prompter -> haiku-scout allowed; (if opted) -> sonnet-analyst allowed, -> opus-architect denied.
- go test ./internal/harness/... still PASS.

## must NOT change
- Flow one-round rule for main (under B); classifier Sonnet tag-only role; other agents' tool lists; D11 (sonnet/opus -> haiku-* only).
