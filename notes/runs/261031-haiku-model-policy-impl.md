task: 모델호출 정책 구현 (라우터 삭제, haiku 80k / opus 200k, sonnet 제거)
repo: /Users/leekwangseok/PycharmProjects/claude-harness (master, HEAD 248b5e0)
pre-existing dirty (do not touch): notes/metrics/2026-10-31.jsonl, imports/, .DS_Store, scripts/.DS_Store, notes/runs/261031-plan-model-call-policy*.md
agent-tools: no Agent/subagent tool available to this run -> haiku-coder/haiku-checker split not possible; work done inline

step1 grep parked|switchSession|has:
  parked(map repl.go:175): used by switchchild.go:179-180 (switch UI), repl.go:1169, livechild.go parkedIdleEvent -> KEEP (not route-only)
  switchSession(classifier.go:279): used by orchws.go:62,74, switchchild.go:59,147 -> KEEP
  has(haikupool.go:68): only conductor_test.go:201 -> KEEP (review 11 read API)
  route-only: classifier.go:253,258-261 (route body)
step2-3 status: pending
open-decisions:
  D-A: 개선안 1 vs 1단계: route 호출(repl.go:879) 제거 시 runTurn(repl.go:500-505) 이 이미 예산게이트+spend리셋+Send 수행 -> submit 별도 추가하면 이중 전송. 계획: submit 미생성, text=line 직행, runTurn 단일 전송점 유지. 보고 필요.
  D-B: Conductor 플래그(repl.go:61,878,1083, /mode conductor tui.go:938,1161, setConductor repl.go:1204) 는 route 전용. 제거 범위 밖이라 유지, 도움말 문구 stale -> 보고.
  D-C: conductor_test.go 의 parseRoute/parseRouteFull/hint 테스트 다수 -> 삭제 심볼 참조로 컴파일 필수 삭제. 계획서 미기재 -> 보고.
  D-D: effectiveThreshold = thresholdFor(model) only; compact-window min 아님 (repl.go:860) -> 기록.

## 7. step 9-10 (done 261031)
- deleted: .claude/agents/sonnet-analyst.md, .claude/agents/sonnet-reviewer.md
- edited: .claude/hooks/agent-model-pin.py (SUB_CALLERS={"opus-architect"}; haiku-prompter target rule; E-gate opus-only; kind prefixes haiku-/opus-; BUILTIN asked!="haiku"; docstring)
- edited: tests/hooks/test_orch_gate.py test_4a/4b/6 sonnet-analyst -> opus-architect (old test failed: 'deny' not found in '')
- go build ./... exit 0 | go vet ./... exit 0 | gofmt -l internal/harness exit 0
- go test ./internal/... -count=1 exit 0 (harness ok 0.616s)
- python3 -I -m unittest discover -s tests/hooks: exit 0, Ran 10 tests OK (pytest not installed)
- orch.go orchDefaultModel: no sonnet (returns opus/haiku). orch.go rule text orchRuleB/C/E still mentions sonnet-analyst (not changed, out of scope).
- leftover sonnet refs (not changed): repl.go:1081 -conductor help, repl.go:1202-1205 setConductor, tui.go:171 /mode help, orch.go:38-44 rule texts,
  main-thread-guard.py:32, haiku-context-guard.py:4-6,29,55, ledger.py:41,94, caveman-format.py:11 TARGETS, haiku-prompter.md:12, opus-architect.md:3,12,15, haiku-coder.md:13, skills/correction, skills/distill
- open: D-A..D-D, rotate-boundary tests not written, TestMainRouteSwitchPolicy not rewritten, settings autoCompactWindow 150000 < opus 200000
