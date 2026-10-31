# v0.8.1 release digest (read-only, no code edits)
sources: notes/runs/261029-review-v081.md, 261029-scope-v081-release.md, 261029-scope-v081-release-gate.md
status: A/B/C implemented, NOT committed/tagged/released; review verdict FAIL (1 blocking)

## remaining blocking
- B1 classifier.go:15 routerModelDefault "sonnet"->"haiku" (+ classifier_model_test.go, conductor_test.go, routerprompt_test.go). Not in v0.8.1 scope, not in CHANGELOG; comment classifier.go:17 says "pending user approval of the default flip".
  decision (user): (a) revert hunks in classifier.go + 3 tests, exclude; or (b) approve and add CHANGELOG line.
- open: commit trailer (gate note: Co-Authored-By: Claude Haiku 5.5; scope note flagged conflict with Sonnet 5.5 reminder; current session reminder says Haiku 5.5).
- open: commit dirty tree as-is or only v0.8.1 files.
non-blocking: CHANGELOG omits CLAUDE.md policy changes; "Edit removed" overstated (sonnet-analyst keeps Write+Bash); haiku-context-guard.py Grep/Glob deny dead unless settings.json PreToolUse matcher includes Grep|Glob (docstring says Read only); agentview.go:166 panelWheel clamp loose; CLAUDE.md sonnet-analyst rule duplicated (~line 112).

## v0.8.1 commit file list (INFERRED, not stated explicitly in notes; all appear in git status as M)
.claude/agents/sonnet-analyst.md
.claude/agents/opus-architect.md
.claude/hooks/haiku-context-guard.py
CLAUDE.md
CHANGELOG.md
cmd/chaude/main.go
internal/harness/repl.go
internal/harness/drain.go
internal/harness/tui.go
internal/harness/agentview.go
maybe: .claude/settings.json (only if Grep|Glob matcher added; not in git status snippet)
excluded pending decision: internal/harness/classifier.go, classifier_model_test.go, conductor_test.go, routerprompt_test.go
unclear (dirty tree, not mentioned as v0.8.1): internal/harness/children.go, internal/harness/livechild.go, notes/FACTS.md, notes/LEDGER.md, notes/metrics/2026-10-29.jsonl

## CHANGELOG.md lines 1-30 (verbatim)
# Changelog

## 0.8.1
- resume 수정: 작업 알림(`<task-notification>`)을 사용자 이벤트에서 처리해 에이전트 추적이 끝나도록 함(onUserEvent).
- 에이전트 패널(입력창 아래) 마우스 휠 스크롤.
- 최종 결과가 비어 있으면 result를 TUI 출력창에 표시(파일 경로만 안내하지 않음).
- sonnet-analyst 직접 파일 읽기/수정 차단(Read/Grep/Glob/Edit 도구 제거, haiku-scout/haiku-coder 위임).
- opus-architect 도구를 Agent 하나로 제한.

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

## classifier.go "pending user approval" (grep for "승인 대기": NO MATCH)
15:	routerModelDefault = "haiku" // classifier model when CHAUDE_ROUTER_MODEL is unset or invalid
17:	// Deprecated: the Sonnet router path is deprecated in favor of Haiku (pending user approval of the default flip).
