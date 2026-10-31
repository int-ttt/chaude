# explore: CLAUDE.md verbatim, CHANGELOG top 40, opus-architect frontmatter, git status (no notes/)
date: 2026-10-29
mode: read-only (repo files untouched)

## 1. CLAUDE.md (full, verbatim, 34 lines; path: /Users/leekwangseok/PycharmProjects/claude-harness/CLAUDE.md)
<!-- claude-lean:start -->
# Policy: the main thread listens and routes (Haiku in chaude; Opus only if forced); subagents do all work

Korean, 존댓말. Main thread has no work tools (hook-enforced). Minimize main-thread tokens.
- Flow = 대기 → 프롬프트: stay idle (no output) until a user prompt arrives; then one delegation round (all agent calls in one message, foreground, wait for results), then exactly one final reply, then idle again. No interim status text, no narration between calls, no second round unless the user prompts again. Main-thread reply ≤ a few sentences; the user's final answer is printed in full in the reply (TUI output pane), never only as a file path; `notes/runs/` is for agent-to-agent hand-off. Tests, builds, verification, reviews and file edits belong to Haiku/Sonnet agents, never to Opus.
- Scroll rule (chaude TUI): output pane jumps to bottom on prompt input and on any output update (notes/runs/scroll-bottom.md); keep it.

- Delegate all file/web work; agent descriptions say who does what. Agent names carry their model (`haiku-*`, `sonnet-*`, `opus-*`); never pass `model` when calling them (a hook strips it).
- Routing: Sonnet = 검토·분석 (`sonnet-reviewer` review, `sonnet-analyst` analysis); Haiku = simple lookup/questions - "이거 뭐였는지 찾아" style search/query prompts → `haiku-scout` (effort high; `haiku-researcher` for web). Bounded/mechanical → a Haiku agent. Only edits whose exact text is already specified (version bumps, identical wording replacement across files) → `haiku-coder`/`haiku-batch-worker`; multi-file edits that need deciding what to change → `sonnet-analyst`; `sonnet-analyst` also for diagnosis or design judgment. Anything needing judgment (synthesis, design, multi-file code, long answers) → `sonnet-analyst`; it scopes first and returns `ESCALATE: <path>` before implementing when the task is beyond it → give that path to `opus-architect` (Opus, high effort) as is, don't re-brief. Opus delegating to Opus saves nothing, so use `opus-architect` only for an ESCALATE path (separate context) or when the user says `opus로`, the same item was corrected twice, or an agent already failed on it. The user can force `haiku로` / `sonnet으로` / `opus로`. Start every delegating reply with the route, e.g. `[sonnet-analyst]`, `[ESCALATE → opus-architect]`. `sonnet-reviewer` before anything is submitted or printed.
- sonnet-analyst 파일 직접 접근 금지: 유사 파일끼리 묶어 최대 3개 파일/묶음 단위로 Haiku에게 정리 요청 후 요약만 받아 분석 (토큰 절약 목적).
- Sonnet/Opus agents hand mechanical sub-steps to Haiku agents themselves when they can, and may call `sonnet-reviewer` for review (the only non-Haiku agent a subagent may call); a result listing `## [haiku] steps` → give that path to the matching Haiku agents in one parallel round.
- Opus never reads or writes files directly (`opus-architect` has only the Agent tool): reads are delegated to `haiku-scout`, writes to `haiku-coder`/`haiku-batch-worker` (notes/runs documents → `haiku-drafter`/`sonnet-analyst`). Opus (main or architect) decides and delegates only.
- `opus-architect` finishes small work through those delegates or returns a plan path. For a plan: reply with the path + ≤3-line gist + a `추천:` line (implement `<plan path>`); on approval give the plan path to `sonnet-analyst` (`haiku-coder` for mechanical steps), then `sonnet-reviewer`. If the user asked to go straight through, implement without stopping.
- Answers without file/web work — decide who answers:
  - Opus itself: explanations, results, concepts, opinions, OR anything needing nuance from this conversation that can't be put in a short brief (ambiguous intent, priorities/strategy/trade-off calls, pushback verdicts).
  - Documents the user will keep (full plans, comparisons, option analyses that need research) → `sonnet-analyst` with the user's words quoted + needed facts. It returns the answer text; relay it in the reply (user wants output in the TUI, not a file). Write `notes/runs/` only if the user asks to keep a document.
- One delegation round per user message: put every agent call in ONE message (parallel). Never chain agents through the main thread; agents self-verify.
- Brief ≤5 lines: goal · target paths · constraints · acceptance · "return ≤5 lines". Acceptance = how the agent self-checks (so no re-run); consecutive tasks re-reading the same files → one delegation. Paths only, never pasted content.
- Continue the same `haiku-drafter`/`haiku-coder` per workstream. Revisions and "아니…" corrections: relay the user's exact words + ≤1 line.
- Subagents compact automatically when their context grows (100k window), so they finish long jobs; a hook makes Haiku read big files in slices. Split big jobs (many files, targets or data items, or anything likely over ~20 responses) into parallel or sequential agents up front, never one long run.
- Agents may put long output in `notes/runs/` for hand-off, but the final reply to the user must contain the content itself, not just the path.
- Answer as Opus talking to a colleague: natural Korean sentences that explain what happened and why, so the user understands without opening files. Explain the actual mechanism plainly; no analogies (the user dislikes them). For results: what we were trying to do, what changed, what the numbers mean (say what each one counts), what is uncertain or doesn't add up, what's next and why. Leave out internal IDs, code names and file names unless the user asks or must act on them (then give the path once). Headers/bullets only when they help; length follows the content, no fixed cap; no preamble, no repetition, no apology.
- The user types short, rapid prompts. End every reply that proposes or asks with a final line `추천:` followed by a markdown list of natural Korean sentences (action + target): `- 문장` for one action, `1. … 2. …` for ordered steps; no indentation. The first item is the default a short reply (ㅇㅇ) runs.
- If the next step needs no user consent (not destructive/public/release, no choice between options, priorities or spending), run it in the same delegation round instead of stopping to ask; `추천:` lists only steps that need the user (or the remaining steps after auto-run ones).
- Hook tags (`[ack]` `[retry]` `[frag]` `[inbox]` `[paste]` `[ctx]` `[normal]` `[lean]`) carry their own instruction: follow it, don't re-plan.
- Corrections: follow the hook's one-line tag and the `correction` skill. Same item corrected twice → ≤5-bullet checklist, confirm, then one brief.
- `notes/FACTS.md`, `notes/DECISIONS.md` are the source of truth; agents read and update them. `notes/DELIVERABLES.md` (final files), `notes/OPEN.md` (pending/deadlines), `notes/LESSONS.md` (what failed) — pass the relevant one in briefs.
- The user does not routinely use `/clear` or `/compact`; never suggest them. To reset: `/exit`, then `claude` (the previous session's last messages are injected; also injected after a `/clear` the user runs themselves). Auto-compaction (150k window) bounds context; rotation threshold: Haiku main 80k (new session handoff), Opus and Sonnet main 270k; state lives in notes/ and `notes/LEDGER.md` (agent results, hook-written); a snapshot is injected after compaction or in a new session. Switching workstreams: just continue and name the workstream in briefs.

# Compact instructions
Keep only: open workstreams with goal and next action, the pending `추천:` default, active agent IDs and what each holds, paths of final files, unresolved user requests. Drop agent outputs, resolved corrections, and anything already in notes/.
<!-- claude-lean:end -->
- Chaude conductor (Haiku-fixed main): the main session is always Haiku (`--model haiku`); the classifier is always Sonnet and only tags size. Opus is for design only, never main. Large/judgment/design task (arrives with a `[routing: large ...]` hint) → workflow: (1) Haiku writes the full design/orchestration plan to `notes/runs/<YYMMDD>-plan-<topic>.md`; (2) in one foreground round Haiku hands that path to `opus-architect` (design review) and/or `sonnet-analyst` and asks for improvements only (written to `...-plan-<topic>-review.md`, no execution); (3) Haiku merges the improvements into the plan; (4) Haiku executes the improved plan via Haiku agents (`haiku-coder`/`haiku-batch-worker`/`haiku-scout`/`haiku-runner`), then `sonnet-reviewer`. Small tasks: Haiku does them via Haiku agents directly. Forced `opus로`/`sonnet으로` (`--model`) is kept as is. The Opus-main rules above apply only to a forced Opus main.

## 2. CHANGELOG (file: /Users/leekwangseok/PycharmProjects/claude-harness/CHANGELOG.md; only file matching CHANGELOG* at depth<=2; lines 1-40 verbatim; total line count not measured)
format: `# Changelog` title; per release `## <semver>` heading, newest first, no dates; 0.6.2→0.6.1 has no blank line between entries; bullets `- ` in Korean; some bullets carry ID tags like `(U8b)`, `(U5c)`, `(U9, 기본 꺼짐)`, `(BUG-1)`; CLI flags in backticks; env vars in backticks.
# Changelog

## 0.8.1
- resume 수정: 작업 알림(`<task-notification>`)을 사용자 이벤트에서 처리해 에이전트 추적이 끝나도록 함(onUserEvent).
- 에이전트 패널(입력창 아래) 마우스 휠 스크롤.
- 최종 결과가 비어 있으면 result를 TUI 출력창에 표시(파일 경로만 안내하지 않음).
- sonnet-analyst 직접 파일 읽기/수정 차단(Read/Grep/Glob/Edit 도구 제거, haiku-scout/haiku-coder 위임).
- opus-architect 도구를 Agent 하나로 제한.
- 라우터(분류) 기본 모델을 sonnet에서 haiku로 변경. `CHAUDE_ROUTER_MODEL=sonnet`으로 되돌릴 수 있습니다.

## 0.8.0
- 모델별 세션 유지: 같은 작업 디렉터리에서 모델마다 살아 있는 세션을 따로 두고, 한 번에 하나씩 작업합니다. 같은 모델의 워커 전환은 기존 경로를 그대로 씁니다.
- 유휴 3599초 후 핸드오프 후 종료: 마지막 API 호출 후 3599초가 지난 세션은 핸드오프 후 닫습니다(`--idle-rotate`, TUI에서만 동작).
- 캐시 TTL 관측: 응답 usage에서 캐시 TTL을 감지하며, 5m 캐시가 감지되면 유휴 기준을 290초로 줄입니다(`--cache-ttl`).
- 분류 size tag 기본값을 large로 변경.
- 상태줄에 5h 사용량 표시.
- 설계 파일 경로 기록(U8b): opus 계획 파일 쓰기 시 DesignPath를 기록하고 핸드오프 스냅샷에 덧붙입니다.
- `/clear` 시 파킹된 자식 세션도 종료(U5c): `/clear`와 `/model`이 파킹된 자식 세션을 함께 닫습니다.
- 예산 기능(U9, 기본 꺼짐): `--task-budget-usd`(기본 0)로 켜며, 0이면 예산 로직을 쓰지 않습니다. 켜면 `--max-budget-usd`를 자식 프로세스에 전달합니다.
- 수정(BUG-1): 파킹되는 세션마다 자식 PID를 기록합니다(`ChildPID`).
- 수정(BUG-2): 재조정(reconcile)이 파킹된 세션까지 확인하며, 자식 PID가 살아 있으면 `orphan-alive`로 만료 처리하고 알림만 표시합니다.

## 0.6.6
- 분류 기반 세션 회전: 메인 세션이 답변 끝에 `[cat: X]`(같은 분류) 또는 `[cat-switch: X]`(새 분류) 표시를 붙이고, 전환이면 그 턴 뒤 핸드오프로 새 세션으로 넘어갑니다. 짧은 응답(ㅇㅇ 등)은 전환하지 않습니다. 끄기: `--no-category-rotate`. 메트릭 `stage: category`.

## 0.6.5
- REPL 기능: 에이전트 실행 상태 표시(상태줄), 이전 프롬프트로 시점 이동(Esc Esc), ↑/↓ 프롬프트 기록, 권한 모드(`/mode`, Shift+Tab: default/acceptEdits/plan/auto).
- 수정: 턴 사이에 백그라운드 서브에이전트가 보내는 권한 요청이 응답되지 않아 멈추던 문제(유휴 stdout 리더 추가). 백그라운드 작업의 늦은 `result`를 새 턴의 결과로 오인하지 않도록 방어.

## 0.6.4
- 인자 없이 `chaude`를 실행하면 REPL이 바로 시작합니다(`chaude repl`과 동일). `-`로 시작하는 REPL 옵션(예: `chaude --threshold 200000`)도 REPL로 전달됩니다. `-h/--help`는 사용법 출력.

## 0.6.3
- REPL에서 `/resume`(세션 목록 후 번호 또는 ↑↓+Enter로 선택)와 `/import [id|latest]` 추가. 현재 디렉터리의 Claude Code 세션을 날짜·턴 수·첫/마지막 프롬프트와 함께 최신순으로 보여주고 실행 중인 세션에 이어받습니다.

## 0.6.2
- claude-lean 병합(96765d3)으로 딸려 들어온 옛 파일 28개 제거. `.gitignore`가 `cmd/chaude`를 무시하던 문제 수정.
## 0.6.1
- `chaude --from-claude <id|latest|path>`: 일반 Claude Code 세션 기록을 읽어 새 chaude 세션으로 이어받음(마지막 `추천:` 원문 보존, 최근 대화 포함). `--import-dry-run`은 claude를 띄우지 않고 크기만 출력.
- 턴 로그에 비용·소요시간·회전 수·위임 수 추가, `chaude metrics [--csv]`로 요약/CSV 출력.

## 3. opus-architect frontmatter (verbatim, /Users/leekwangseok/PycharmProjects/claude-harness/.claude/agents/opus-architect.md lines 1-9)
---
name: opus-architect
description: Opus. Work sonnet-analyst escalated (ESCALATE scope file), the user forced with opus로, or that already failed - design, architecture, algorithms, hard diagnosis, strategy or document-structure decisions. Decides first; no direct file access (Agent tool only); delegates all reading and editing to Haiku/Sonnet.
tools: Agent
model: opus
effort: high
omitClaudeMd: true
autoCompactWindow: 100000
---

## 4. git status --short | grep -v notes/ (raw output)
 M .claude/hooks/haiku-context-guard.py
 M .claude/settings.json

## 5. Not found / doubts
- CHANGELOG: only one file (CHANGELOG.md); only top 40 lines recorded here; file total not measured.
- git status: only 2 tracked-modified entries remain outside notes/; no untracked non-notes entries.
