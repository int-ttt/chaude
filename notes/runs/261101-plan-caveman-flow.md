# plan: caveman write/read flow (261101)
user: "caveman-structure.md 워크플로 수정 ... 요청 2: 쓰기 흐름 변경 ... 전달 내용은 '완료했다'와 '어떤 부분을 바꿨다'만. 요청 3: 읽기 흐름 변경 ... 출력창에는 caveman 결과 요약 후 표시"
targets_now: plan/caveman-structure.md, CLAUDE.md
targets_later(S6, not executed): docs/claude-lean/CAVEMAN_FORMAT.md, .claude/agents/{haiku-coder,haiku-batch-worker,haiku-checker,haiku-scout,haiku-researcher,haiku-tui,opus-architect}.md, .claude/hooks/caveman-format.py, tests/hooks

## facts (scout 261101)
- E1: haiku-coder.md:4, haiku-scout.md, haiku-researcher.md: no Agent tool -> cannot call haiku-tui directly
- E2: agent-model-pin.py:44-50: opus subagent may call haiku-* (incl haiku-tui), but haiku-tui output then returns to opus, not to output pane; output pane = main reply only (CLAUDE.md:16)
- E3: haiku-tui.md:4 tools: Read; :11 input = R-* blocks / summary_ko (brief or relayed by main)
- E4: haiku-coder.md:14,22-30 return = cause/changed/check/risk; R-write = status/reason/changed/diffstat/check/ws/summary_ko
- E5: caveman-format.py:13-17 REQUIRED R-write = changed,diffstat,check,ws; :105 CAVEMAN_GUARD default log; :131 always exit 0 (never blocks)
- E6: main-thread-guard.py:17,28-32 main denied Read/Write/Edit/Bash/Grep/Glob/Web*; unchanged by this plan
- E7: CLAUDE.md:10 "user prompt -> opus-architect (R-decide) -> Haiku relays summary_ko via haiku-tui"
- E8: caveman-structure.md:48 rule_4 summary_ko only for main-direct returns, 메인은 항상 lang: ko
- E9: LESSONS.md failed entries (261029 x2) unrelated (TUI cursor, /resume)

## 요청 1: 처음 쓰는 사용자용 설명 (outline)
- 1. 사용자가 입력 -> 메인(Haiku)이 받음. 메인은 길 안내만, 파일 못 읽고 못 고침 (훅이 막음)
- 2. 설계·판단이 필요하면 haiku-prompter/메인이 원문 + 요약을 opus에 넘김
- 3. opus는 생각만 하고, 읽기는 haiku-scout/haiku-researcher, 쓰기는 haiku-coder/haiku-batch-worker에 맡김
- 4. 에이전트끼리는 caveman(짧은 key: value)으로 주고받음 -> 토큰 절약
- 5. 마지막에 haiku-tui가 caveman 결과를 한국어 존댓말로 바꿔 출력창에 표시. 사용자는 이것만 봄

## decision
- choice: A relay-through-caller. sub return -> caller(opus) -> opus puts same block unchanged in R-decide field `relay:` -> main passes relay unchanged to haiku-tui -> haiku-tui output = main reply -> output pane. main-direct small task: main passes sub block to haiku-tui itself
- why: E1 subs cannot call haiku-tui; E2 only main reply reaches pane; no hook change needed; main stays routing-only (E6)
- rejected B relay file: sub writes notes/runs/<id>-relay.md, haiku-tui reads it. scout is read-only; extra disk hop; stale-file risk
- rejected C hook relay: SubagentStop/ledger.py injects returns into main context. needs hook rewrite; hidden path hard to verify; larger change than asked
- trade-off: relay passes through opus return and main context -> more main tokens (cap 1500 chars); "동시에 두 곳" is logical, physically sequential

## 요청 2: write flow
- path: opus (or main for small task) -> haiku-coder (code) | haiku-batch-worker (docs, notes, bulk) -> return -> opus + haiku-tui (via relay)
- return block (new R-write):
  status: done | partial | fail | blocked   (= 완료 여부)
  reason: <partial·fail·blocked일 때 한 줄, 아니면 ->
  changed: <경로>: <바꾼 부분 한 줄>   (파일마다 1줄)
  ws: <작업 공간 id>   (worktree 쓴 경우만, 아니면 줄 생략)
- removed from write return: diffstat, check, summary_ko (ws optional)
- verification: haiku-checker separate; return = status/reason + check: <명령> -> PASS|FAIL; also relayed
- output pane (haiku-tui): "<작업> 완료했습니다." 1줄 + 파일마다 "<경로>: <바꾼 부분>" 1줄 (+ 검증 1줄). fail/blocked: status + reason 그대로

## 요청 3: read flow
- path: opus -> haiku-scout (local files, past sessions) | haiku-researcher (web) -> R-read (3.2 unchanged, caveman) -> opus + haiku-tui (via relay)
- opus uses claim/not_found/not_checked as evidence; does not reword relay
- output pane (haiku-tui): first 존댓말 1~3줄 요약 of caveman result, then items (경로:행 + 내용) list; not_found -> "없음 확인" 1줄, not_checked -> "미확인" 1줄; no facts beyond input
- relay cap 1500 chars: over -> opus trims claim lines, keeps not_found/not_checked, sets truncated: yes

## file changes (now)
### plan/caveman-structure.md
- S-a: insert "### 2.1 쓰기·읽기 흐름 (261101, 사용자 요청 2·3)" before "## 3. 메시지 형식 정의"
- S-b: 3.3 R-write block -> changed (경로: 바꾼 부분) + optional ws; note after block
- S-c: rule_4 append: 쓰기·읽기 에이전트 브리프는 메인 직속이어도 lang: caveman, 한국어화는 haiku-tui
- S-d: §4 table rows 메인 세션, opus-architect, scout/researcher, coder/batch/runner, checker, haiku-tui
- S-e: §8 add S6 line (follow-up, 미실행)
### CLAUDE.md
- C-a: line 10 "Haiku relays summary_ko via `haiku-tui`" -> "Haiku relays R-decide `relay`(없으면 summary_ko) via `haiku-tui`"
- C-b: insert 2 Korean lines after line 10 (쓰기 흐름, 읽기 흐름)

## must NOT change
- main-thread-guard.py, agent-model-pin.py, settings.json; main gets no file tools
- R-read 3.2 fields; B형식 3.1; R-decide existing fields (relay is added, optional)
- haiku-prompter loop role (rule_7)
- originals not deleted; no commit

## acceptance
- A1: caveman-structure.md has "### 2.1 쓰기·읽기 흐름", new 3.3 block, S6 line
- A2: CLAUDE.md has "쓰기 흐름:" and "읽기 흐름:" lines, line 10 mentions relay
- A3: no remaining text in either file says write return includes diffstat/check/summary_ko as required, or main relays summary_ko only
- A4: CLAUDE.md flows match caveman-structure.md 2.1 (same agents, same return content)
- A5: main-thread-guard.py unchanged (git diff empty for that file vs session start)

## [haiku] steps (S6, later, user go needed)
- haiku-batch-worker: CAVEMAN_FORMAT.md 3.3 -> 2.1 R-write; haiku-coder.md:14,22-30 and haiku-batch-worker.md return = status + changed only; haiku-checker.md = status + check
- haiku-batch-worker: haiku-scout.md, haiku-researcher.md add 1 line "반환은 호출자(opus)가 relay로 haiku-tui에 그대로 넘김"; haiku-tui.md:11 input adds relay + write/read output shapes; opus-architect.md R-decide adds `relay:` (optional, ≤1500 chars)
- haiku-coder: caveman-format.py:13-17 REQUIRED["R-write"] = ["changed"]; update tests/hooks accordingly; run tests/hooks

## review merged (261101)
- source: notes/runs/261101-plan-caveman-flow-review.md
- R1 -> S6 haiku-coder step: grep tests/hooks for "diffstat" before editing caveman-format.py; update same tests
- R2 -> acceptance note: log-mode missing-field rows for R-write are expected until S6
- R3 -> bridge rule until S6: write briefs put "return: status + changed(경로: 바꾼 부분) only" in limits
- R4 -> out of scope: haiku-prompter loop return unchanged
- R5 -> relay cap 1500 chars kept
- R6 -> open_q to user (raw caveman lines in pane or not)
